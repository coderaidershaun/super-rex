// The policy's rules over rex_merge of a tree job, checked before the merge module runs: the pinned policy, the merge order, the paths that always ask, and the checks a policy answer needs.
// In a repo with auto-merge no path asks: the answer is yes with no question, and the merge checks still run.
// Every merge question of a tree job, its own or one the merge module parked, is shown to the person with its status line and dialog.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob } from '../../types'
import { applyAutoMerge, errorText, fail, readBook } from '../book/index'
import { answer, baseRef, callerOf, deny, git, repoPlaces } from '../tools/index'
import { announceQuestion, parkQuestion } from './ask'
import { pinnedPolicy } from './pin'
import { matchesGlob } from './policy'
import type { Policy } from './policy'

const TOOL = 'rex_merge'
const CHECK_TIMEOUT_MS = 600_000

function mergeAfterRefusal(book: RexBook, job: RexJob): string | null {
  for (const id of job.mergeAfter) {
    const other = book.jobs.find(each => each.id === id)
    const isMerged = other?.deliveries.some(delivery => delivery.outcome === 'merged') ?? false
    if (!isMerged) {
      return `${TOOL}: job ${job.id} merges after job ${id}, got ${id} ${other?.status ?? 'unknown'} and not merged; call again after it merges`
    }
  }
  return null
}

/** The merge question of a job: the one already parked, else a new one naming the paths that ask. */
async function parkedQuestion($: EngineInterface, book: RexBook, job: RexJob, paths: readonly string[]): Promise<string> {
  const parked = book.questions.find(question => question.mergeFor === job.id && question.answer === null)
  if (parked !== undefined) {
    return parked.id
  }
  const question = await parkQuestion($, {
    tree: job.tree,
    job: job.id,
    question: `Merge job ${job.id} (${job.branch}) to ${job.base} of ${job.repo}? It touches ${paths.join(', ')}, which the policy always asks about.`,
    options: ['Merge', 'Keep the branch'],
    recommendation: 'Keep the branch',
    category: 'merge',
    mergeFor: job.id,
  })
  return question.id
}

/** Each merge check in the job's worktree; null when all exit 0 or the policy names none (the builder's own checks passed), else the refusal of the first that failed. */
async function failedCheck($: EngineInterface, job: RexJob, policy: Policy): Promise<string | null> {
  const worktree = job.worktree ?? job.repo
  for (const argv of policy.merge_checks) {
    const ran = await $.process.run(argv, { cwd: worktree, timeoutMs: CHECK_TIMEOUT_MS })
    if (ran.exitCode !== 0) {
      const line = ran.stderr.split('\n')[0]?.trim() ?? ''
      return `${TOOL}: merge check ${argv.join(' ')} failed in ${worktree}, got exit ${String(ran.exitCode)}: ${line}; fix it and call again`
    }
  }
  return null
}

/** A pass names the tree job whose merge question the merge module may park, so this module shows it to the person. */
type Verdict = { readonly kind: 'pass'; readonly job: string | null } | { readonly kind: 'answer'; readonly result: ToolCallResult }

async function verdictOf($: EngineInterface, agentId: string | undefined): Promise<Verdict> {
  const caller = await callerOf($, agentId)
  const called = caller.job
  // A job with no plan passes on, so rex_merge refuses it before any policy check runs a command or parks a question.
  if (caller.kind !== 'builder' || called === null || called.tree === null || called.status !== 'planned') {
    return { kind: 'pass', job: null }
  }
  // A job opened before the repo got auto-merge takes its answer here, so the merge checks below run for it too.
  const job = await applyAutoMerge($, called)
  const passed: Verdict = { kind: 'pass', job: job.id }
  const book = await readBook($)
  const tree = book.trees.find(each => each.id === job.tree)
  const isPolicyAnswer = job.merge?.answer === 'yes' && job.merge.source === 'policy'
  if (tree === undefined) {
    return isPolicyAnswer ? { kind: 'answer', result: deny(`${TOOL}: job ${job.id} holds a policy answer, got tree ${job.tree} that is not in this session; ask the person with /rex answer`) } : passed
  }
  const pinned = await pinnedPolicy($, tree)
  if (pinned.kind === 'refused') {
    return { kind: 'answer', result: deny(pinned.text) }
  }
  const held = mergeAfterRefusal(book, job)
  if (held !== null) {
    return { kind: 'answer', result: deny(held) }
  }
  if (!isPolicyAnswer) {
    return passed
  }
  const policy = pinned.policy
  const ref = baseRef({ base: job.base, remote: (await repoPlaces($, job.repo)).remote })
  const diff = await git($, job.repo, ['diff', '--name-only', `${ref}...${job.branch}`])
  const touched = diff.stdout.split('\n').filter(path => path !== '' && policy.always_ask_paths.some(glob => matchesGlob(path, glob)))
  if (touched.length > 0 && !book.autoMerge) {
    return { kind: 'answer', result: answer(['merged=no', `question=${await parkedQuestion($, book, job, touched)}`]) }
  }
  const failed = await failedCheck($, job, policy)
  return failed === null ? passed : { kind: 'answer', result: deny(failed) }
}

/** Fails closed: a check that throws refuses the merge in this hook, since a catch that answers does not refuse everywhere. */
export async function serveMergePolicy<E extends { readonly agentId?: string }>($: EngineInterface, event: E, next: (e: E) => Promise<ToolCallResult>): Promise<ToolCallResult> {
  let verdict: Verdict
  try {
    verdict = await verdictOf($, event.agentId)
  } catch (error) {
    const name = error instanceof Error ? error.name : 'Error'
    await fail($, { kind: 'authority-check-failed', check: 'policy-merge', error: error instanceof Error ? `${name}: ${error.message}` : String(error) })
    return deny(`rex gate: policy-merge failed (${name}); refused`)
  }
  if (verdict.kind === 'answer') {
    return verdict.result
  }
  const before = new Set((await readBook($)).questions.map(question => question.id))
  const result = await next(event)
  await announceMergeQuestion($, verdict.job, before)
  return result
}

/**
 * The merge module parks the merge question of a tree job in the book alone;
 * this shows a new one to the person. It never throws: the merge already ran,
 * and the hook's catch would run it again.
 */
async function announceMergeQuestion($: EngineInterface, job: string | null, before: ReadonlySet<string>): Promise<void> {
  if (job === null) {
    return
  }
  try {
    const parked = (await readBook($)).questions.find(question => question.mergeFor === job && !before.has(question.id))
    if (parked !== undefined) {
      announceQuestion($, parked)
    }
  } catch (error) {
    await fail($, { kind: 'observer-failed', event: 'tool.call', module: 'master', agentId: null, error: errorText(error) })
  }
}

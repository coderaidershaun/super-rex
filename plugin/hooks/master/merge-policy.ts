// The policy's rules over rex_merge of a tree job, checked before the merge module runs: the pinned policy, the merge order, the paths that always ask, and the checks a policy answer needs.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob } from '../../types'
import { addQuestion, fail, readBook } from '../book/index'
import { answer, callerOf, deny } from '../tools/index'
import { pinnedPolicy } from './pin'
import { matchesGlob } from './policy'
import type { Policy } from './policy'
import { git } from './worktree'

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
  const question = await addQuestion($, {
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

/** Each merge check in the job's worktree; null when all exit 0, else the refusal of the first that failed. */
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

type Verdict = { readonly kind: 'pass' } | { readonly kind: 'answer'; readonly result: ToolCallResult }

async function verdictOf($: EngineInterface, agentId: string | undefined): Promise<Verdict> {
  const caller = await callerOf($, agentId)
  const job = caller.job
  if (caller.kind !== 'builder' || job === null || job.tree === null) {
    return { kind: 'pass' }
  }
  const book = await readBook($)
  const tree = book.trees.find(each => each.id === job.tree)
  const isPolicyAnswer = job.merge?.answer === 'yes' && job.merge.source === 'policy'
  if (tree === undefined) {
    return isPolicyAnswer ? { kind: 'answer', result: deny(`${TOOL}: job ${job.id} holds a policy answer, got tree ${job.tree} that is not in this session; ask the person with /rex answer`) } : { kind: 'pass' }
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
    return { kind: 'pass' }
  }
  const policy = pinned.policy
  if (policy.merge_checks.length === 0) {
    return { kind: 'answer', result: deny(`${TOOL}: policy allowed_if_checks_pass needs merge_checks, got none`) }
  }
  const diff = await git($, job.repo, ['diff', '--name-only', `origin/${job.base}...${job.branch}`])
  const touched = diff.stdout.split('\n').filter(path => path !== '' && policy.always_ask_paths.some(glob => matchesGlob(path, glob)))
  if (touched.length > 0) {
    return { kind: 'answer', result: answer(['merged=no', `question=${await parkedQuestion($, book, job, touched)}`]) }
  }
  const failed = await failedCheck($, job, policy)
  return failed === null ? { kind: 'pass' } : { kind: 'answer', result: deny(failed) }
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
  return verdict.kind === 'pass' ? next(event) : verdict.result
}

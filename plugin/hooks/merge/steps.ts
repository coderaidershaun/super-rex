// The steps of rex_merge, in order: the caller's own job, the repo lock, the git checks, the merge answer, then the one push to the base.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob } from '../../types'
import { addQuestion, fail, readBook, recordMergeDone } from '../book/index'
import type { ToolEntry } from '../parts'
import { answer, callerOf, callerRefusal, deny, jobArgRefusal } from '../tools/index'

const TOOL = 'rex_merge'
const GIT_TIMEOUT_MS = 120_000

/** The repos a merge runs in now, each with the job that holds it. A reload drops it, and with it no merge is left half done: the push is one call. */
const locks = new Map<string, string>()

type Git = { readonly exitCode: number; readonly stdout: string; readonly firstError: string }

async function git($: EngineInterface, repo: string, args: readonly string[]): Promise<Git> {
  const ran = await $.process.run(['git', '-C', repo, ...args], { cwd: repo, timeoutMs: GIT_TIMEOUT_MS })
  return { exitCode: ran.exitCode, stdout: ran.stdout.trim(), firstError: ran.stderr.split('\n')[0]?.trim() ?? '' }
}

function wasMergedHere(job: RexJob): boolean {
  return job.deliveries.some(delivery => delivery.outcome === 'merged' && delivery.repo === job.repo && delivery.mergeCommit !== null)
}

/**
 * True when a commit was ever made on the branch, by its own reflog. A branch
 * with none has no work to merge yet, even when its tip sits on the base. A
 * reflog that cannot be read counts as a commit: that keeps the halt for a
 * merge outside rex_merge whenever the branch's history is unknown.
 */
async function hadOwnCommit($: EngineInterface, repo: string, branch: string): Promise<boolean> {
  const reflog = await git($, repo, ['reflog', 'show', '--format=%gs', `refs/heads/${branch}`, '--'])
  const moves = reflog.stdout.split('\n').filter(line => line.length > 0)
  const isCommit = (move: string): boolean => /^(commit|merge|cherry-pick|revert)\b/.test(move)
  return reflog.exitCode !== 0 || moves.length === 0 || moves.some(isCommit)
}

/** A tree job without a yes waits on one merge question for the person; a repeat names the one already parked. */
async function parkedQuestion($: EngineInterface, book: RexBook, job: RexJob): Promise<string> {
  const parked = book.questions.find(question => question.mergeFor === job.id && question.answer === null)
  if (parked !== undefined) {
    return parked.id
  }
  const question = await addQuestion($, {
    tree: job.tree,
    job: job.id,
    question: `Merge job ${job.id} (${job.branch}) to ${job.base} of ${job.repo}?`,
    options: ['Merge', 'Keep the branch'],
    recommendation: 'Merge',
    category: 'merge',
    mergeFor: job.id,
  })
  return question.id
}

/** Steps 3 to 6 for a job whose repo lock this call holds. */
async function mergeLocked($: EngineInterface, book: RexBook, job: RexJob): Promise<ToolCallResult> {
  const { repo, base, branch } = job
  const fetched = await git($, repo, ['fetch', 'origin', base])
  if (fetched.exitCode !== 0) {
    return deny(`${TOOL}: fetch of origin/${base} failed, got exit ${String(fetched.exitCode)}: ${fetched.firstError}; call again`)
  }
  const counted = await git($, repo, ['rev-list', '--count', `origin/${base}..${branch}`])
  if (counted.stdout === '0' && counted.exitCode === 0) {
    const head = await git($, repo, ['rev-parse', branch])
    if (wasMergedHere(job)) {
      return answer(['merged=yes', `merge_commit=${head.stdout}`, `base=${base}`, `branch=${branch}`])
    }
    if (!(await hadOwnCommit($, repo, branch))) {
      await fail($, { kind: 'merge-not-fast-forward', branch, base })
      return deny(`${TOOL}: ${branch} must be one squash commit over origin/${base}, got 0 commits; squash it and call again`)
    }
    await fail($, { kind: 'merge-outside-rex', job: job.id, commit: head.stdout, base })
    return deny(`${TOOL}: origin/${base} already holds ${branch}, got commit ${head.stdout} that rex_merge did not push; job ${job.id} is blocked, ask the person`)
  }
  if (counted.exitCode !== 0 || counted.stdout !== '1') {
    await fail($, { kind: 'merge-not-fast-forward', branch, base })
    return deny(`${TOOL}: ${branch} must be one squash commit over origin/${base}, got ${counted.exitCode === 0 ? counted.stdout : counted.firstError} commits; squash it and call again`)
  }
  const ancestor = await git($, repo, ['merge-base', '--is-ancestor', `origin/${base}`, branch])
  if (ancestor.exitCode !== 0) {
    await fail($, { kind: 'merge-not-fast-forward', branch, base })
    return deny(`${TOOL}: origin/${base} moved, got ${branch} not on it; rebase the squash commit and call again`)
  }
  await git($, repo, ['diff', '--name-only', `origin/${base}...${branch}`])
  if (job.merge?.answer !== 'yes') {
    if (job.tree !== null && job.merge?.answer !== 'no') {
      return answer(['merged=no', `question=${await parkedQuestion($, book, job)}`])
    }
    await fail($, { kind: 'merge-answer-missing', job: job.id })
    const seen = job.merge === null ? 'none' : `${job.merge.answer} (${job.merge.source})`
    return deny(`${TOOL}: job ${job.id} has no merge answer, got ${seen}; ask the person (the builder skill's merge question), or close the job kept-local`)
  }
  const pushed = await git($, repo, ['push', 'origin', `${branch}:refs/heads/${base}`])
  if (pushed.exitCode !== 0) {
    await fail($, { kind: 'merge-push-rejected', exitCode: pushed.exitCode, stderr: pushed.firstError })
    return deny(`${TOOL}: push to origin/${base} failed, got exit ${String(pushed.exitCode)}: ${pushed.firstError}; fetch, rebase and call again`)
  }
  const head = await git($, repo, ['rev-parse', branch])
  await recordMergeDone($, job.id, { repo, base, branch, mergeCommit: head.stdout })
  return answer(['merged=yes', `merge_commit=${head.stdout}`, `base=${base}`, `branch=${branch}`])
}

export async function serveMerge($: EngineInterface, event: { readonly agentId?: string; readonly [key: string]: unknown }, entry: ToolEntry): Promise<ToolCallResult> {
  const caller = await callerOf($, event.agentId)
  const job = caller.job
  const refusal = callerRefusal(entry, caller)
  if (refusal !== null || caller.kind !== 'builder' || job === null) {
    await fail($, { kind: 'caller-not-allowed', tool: TOOL, caller: caller.kind })
    return deny(refusal ?? `${TOOL}: only a builder may call it, got ${caller.kind}`)
  }
  const jobText = jobArgRefusal(TOOL, event['job'], job)
  if (jobText !== null) {
    await fail($, { kind: 'bad-argument', tool: TOOL, argument: 'job', value: String(event['job']) })
    return deny(jobText)
  }
  const holder = locks.get(job.repo)
  if (holder !== undefined) {
    await fail($, { kind: 'merge-lock-held', repo: job.repo, job: holder })
    return deny(`${TOOL}: a merge of ${job.repo} runs now, got the lock held by job ${holder}; call again when it ends`)
  }
  locks.set(job.repo, job.id)
  try {
    return await mergeLocked($, await readBook($), job)
  } finally {
    locks.delete(job.repo)
  }
}

// The steps of rex_merge, in order: the caller's own job, the repo lock, the git checks, the merge answer, then the one move of the base: a push to origin, or a fast-forward of the local base.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob } from '../../types'
import { addQuestion, fail, readBook, recordMergeDone } from '../book/index'
import type { ToolEntry } from '../parts'
import { answer, baseRef, callerOf, callerRefusal, deny, git, hadOwnCommit, jobArgRefusal, repoPlaces } from '../tools/index'
import type { Git } from '../tools/index'

const TOOL = 'rex_merge'

/** The repos a merge runs in now, each with the job that holds it. A reload drops it, and with it no merge is left half done: the move of the base is one call. */
const locks = new Map<string, string>()

function wasMergedHere(job: RexJob): boolean {
  return job.deliveries.some(delivery => delivery.outcome === 'merged' && delivery.repo === job.repo && delivery.mergeCommit !== null)
}

/**
 * A tree job without a yes waits on one merge question for the person; a repeat names the one already parked.
 * The master module's hook around rex_merge shows a new one to the person, so this module stays free of the master.
 */
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

/** The worktree that has `base` checked out, by `git worktree list`; null when none has. */
async function checkoutOf($: EngineInterface, repo: string, base: string): Promise<string | null> {
  const listed = await git($, repo, ['worktree', 'list', '--porcelain'])
  const blocks = listed.stdout.split('\n\n').map(block => block.split('\n'))
  const holder = blocks.find(lines => lines.includes(`branch refs/heads/${base}`))
  return holder?.find(line => line.startsWith('worktree '))?.slice('worktree '.length) ?? null
}

type Landed = { readonly kind: 'landed' } | { readonly kind: 'refused'; readonly exitCode: number; readonly firstError: string; readonly text: string }

/** Landed when the git call exited 0, else refused with `text`. */
function landing(ran: Git, text: string): Landed {
  return ran.exitCode === 0 ? { kind: 'landed' } : { kind: 'refused', exitCode: ran.exitCode, firstError: ran.firstError, text }
}

/** Mode `origin`: the one push of the job branch to the base on origin. */
async function pushToOrigin($: EngineInterface, repo: string, base: string, branch: string): Promise<Landed> {
  const pushed = await git($, repo, ['push', 'origin', `${branch}:refs/heads/${base}`])
  return landing(pushed, `${TOOL}: push to origin/${base} failed, got exit ${String(pushed.exitCode)}: ${pushed.firstError}; fetch, rebase and call again`)
}

/**
 * A move of a base that no worktree has checked out, as `git merge --ff-only`
 * would make it: only when the base is an ancestor of the job branch now, and
 * only while the base still points where it was read, by update-ref's old hash.
 */
async function updateBaseRef($: EngineInterface, repo: string, base: string, branch: string): Promise<Landed> {
  const text = (ran: Git): string => `${TOOL}: update of ${base} failed, got exit ${String(ran.exitCode)}: ${ran.firstError}; call again`
  const heads = await git($, repo, ['rev-parse', branch, `refs/heads/${base}`])
  const [tip = '', old = ''] = heads.stdout.split('\n')
  if (heads.exitCode !== 0) {
    return landing(heads, text(heads))
  }
  const forward = await git($, repo, ['merge-base', '--is-ancestor', old, tip])
  if (forward.exitCode !== 0) {
    return landing(forward, `${TOOL}: ${base} moved, got ${branch} not on it; rebase the squash commit and call again`)
  }
  const moved = await git($, repo, ['update-ref', `refs/heads/${base}`, tip, old])
  return landing(moved, text(moved))
}

/**
 * Mode `none`: a fast-forward of the local base to the job branch, in the
 * worktree that has the base checked out, so the person sees the files there;
 * with no such worktree, a guarded update of its ref.
 */
async function fastForwardLocal($: EngineInterface, repo: string, base: string, branch: string): Promise<Landed> {
  const checkout = await checkoutOf($, repo, base)
  if (checkout === null) {
    return updateBaseRef($, repo, base, branch)
  }
  const merged = await git($, checkout, ['merge', '--ff-only', branch])
  const got = [merged.firstError, ...merged.listed].join(' ')
  return landing(merged, `${TOOL}: fast-forward of ${base} in ${checkout} failed, got exit ${String(merged.exitCode)}: ${got}; commit or remove the changes there that the merge would overwrite, then call again`)
}

/** Steps 3 to 6 for a job whose repo lock this call holds, read against `origin/<base>` in mode `origin` and the local base in mode `none`. */
async function mergeLocked($: EngineInterface, book: RexBook, job: RexJob): Promise<ToolCallResult> {
  const { repo, base, branch } = job
  const { remote } = await repoPlaces($, repo)
  const ref = baseRef({ base, remote })
  if (remote === 'origin') {
    const fetched = await git($, repo, ['fetch', 'origin', base])
    if (fetched.exitCode !== 0) {
      return deny(`${TOOL}: fetch of origin/${base} failed, got exit ${String(fetched.exitCode)}: ${fetched.firstError}; make sure git -C ${repo} fetch origin ${base} works, then call again`)
    }
  }
  const counted = await git($, repo, ['rev-list', '--count', `${ref}..${branch}`])
  if (counted.stdout === '0' && counted.exitCode === 0) {
    const head = await git($, repo, ['rev-parse', branch])
    if (wasMergedHere(job)) {
      return answer(['merged=yes', `merge_commit=${head.stdout}`, `base=${base}`, `branch=${branch}`])
    }
    if (!(await hadOwnCommit($, repo, branch))) {
      await fail($, { kind: 'merge-not-fast-forward', branch, base })
      return deny(`${TOOL}: ${branch} must be one squash commit over ${ref}, got 0 commits; squash it and call again`)
    }
    await fail($, { kind: 'merge-outside-rex', job: job.id, commit: head.stdout, base })
    return deny(`${TOOL}: ${ref} already holds ${branch}, got commit ${head.stdout} that rex_merge did not merge; job ${job.id} is blocked, ask the person`)
  }
  if (counted.exitCode !== 0 || counted.stdout !== '1') {
    await fail($, { kind: 'merge-not-fast-forward', branch, base })
    return deny(`${TOOL}: ${branch} must be one squash commit over ${ref}, got ${counted.exitCode === 0 ? counted.stdout : counted.firstError} commits; squash it and call again`)
  }
  const ancestor = await git($, repo, ['merge-base', '--is-ancestor', ref, branch])
  if (ancestor.exitCode !== 0) {
    await fail($, { kind: 'merge-not-fast-forward', branch, base })
    return deny(`${TOOL}: ${ref} moved, got ${branch} not on it; rebase the squash commit and call again`)
  }
  await git($, repo, ['diff', '--name-only', `${ref}...${branch}`])
  if (job.merge?.answer !== 'yes') {
    if (job.tree !== null && job.merge?.answer !== 'no') {
      return answer(['merged=no', `question=${await parkedQuestion($, book, job)}`])
    }
    await fail($, { kind: 'merge-answer-missing', job: job.id })
    const seen = job.merge === null ? 'none' : `${job.merge.answer} (${job.merge.source})`
    return deny(`${TOOL}: job ${job.id} has no merge answer, got ${seen}; ask the person (the builder skill's merge question), or close the job kept-local`)
  }
  const landed = remote === 'origin' ? await pushToOrigin($, repo, base, branch) : await fastForwardLocal($, repo, base, branch)
  if (landed.kind === 'refused') {
    await fail($, { kind: 'merge-rejected', exitCode: landed.exitCode, stderr: landed.firstError })
    return deny(landed.text)
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
  if (job.status !== 'planned') {
    await fail($, { kind: 'merge-not-planned', job: job.id, status: job.status })
    return deny(`${TOOL}: job ${job.id} has no plan, got status ${job.status}; record the plan with rex_job_plan, then call again`)
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

// What rex_job_close removes once the close is recorded: each worktree of the job that holds no uncommitted work, and each branch of the job, here and on origin, whose work is on the base; it keeps and names everything else, and never fails the close.
import type { EngineInterface } from 'claude-code'

import type { RexJob } from '../../types'
import { jobWorktree } from '../book/index'
import { attempt, git, hadOwnCommit } from './git'
import type { Git } from './git'
import { baseRef, repoPlaces } from './places'
import { removeWorktree } from './worktree'

function why(ran: Git): string {
  return ran.firstError === '' ? `exit ${String(ran.exitCode)}` : ran.firstError
}

function namesOf(ran: Git): readonly string[] {
  return ran.exitCode === 0 ? ran.stdout.split('\n').filter(line => line !== '') : []
}

async function isAncestor($: EngineInterface, repo: string, commit: string, of: string): Promise<boolean> {
  return (await attempt(() => git($, repo, ['merge-base', '--is-ancestor', commit, of]))).exitCode === 0
}

/**
 * The tip of `<job branch>--work` when its tree is the tree of the commit
 * rex_merge merged, and that commit is on the base: then every commit up to
 * that tip reached the base, in the squash. Null for a job with no such merge.
 */
async function squashedTip($: EngineInterface, job: RexJob, ref: string): Promise<string | null> {
  const merged = job.deliveries.find(delivery => delivery.outcome === 'merged' && delivery.repo === job.repo)?.mergeCommit ?? null
  if (merged === null) {
    return null
  }
  const work = `refs/heads/${job.branch}--work`
  const read = await attempt(() => git($, job.repo, ['rev-parse', work, `${work}^{tree}`, `${merged}^{tree}`]))
  const [tip = '', workTree = '', mergedTree = ''] = read.stdout.split('\n')
  const isSquash = read.exitCode === 0 && workTree !== '' && workTree === mergedTree
  return isSquash && (await isAncestor($, job.repo, merged, ref)) ? tip : null
}

type Landed = { readonly repo: string; readonly ref: string; readonly squashed: string | null }

/** A branch whose tip is on the base, or below the squashed tip, holds no work the base lacks. */
async function isOnBase($: EngineInterface, landed: Landed, branch: string): Promise<boolean> {
  return (await isAncestor($, landed.repo, branch, landed.ref)) || (landed.squashed !== null && (await isAncestor($, landed.repo, branch, landed.squashed)))
}

/** Each worktree of the job, removed without force: git keeps one that holds uncommitted or untracked files. */
async function cleanWorktrees($: EngineInterface, job: RexJob): Promise<readonly string[]> {
  const own = job.worktree ?? jobWorktree(job.repo, job.id)
  const listed = await attempt(() => git($, job.repo, ['worktree', 'list', '--porcelain']))
  if (listed.exitCode !== 0) {
    return [`kept.worktree=${own}: git worktree list failed, got ${why(listed)}`]
  }
  const paths = listed.stdout
    .split('\n')
    .filter(line => line.startsWith('worktree '))
    .map(line => line.slice('worktree '.length))
    .filter(path => path === own || path.startsWith(`${own}--`))
  const lines: string[] = []
  for (const path of paths) {
    const removed = await attempt(() => removeWorktree($, job.repo, path))
    lines.push(removed.exitCode === 0 ? `removed.worktree=${path}` : `kept.worktree=${path}: ${why(removed)}`)
  }
  return lines
}

/** Each local branch of the job whose work is on the base, or that never got a commit of its own. */
async function cleanBranches($: EngineInterface, job: RexJob, landed: Landed): Promise<readonly string[]> {
  const listed = await attempt(() => git($, job.repo, ['for-each-ref', '--format=%(refname:short)', `refs/heads/${job.branch}`, `refs/heads/${job.branch}--*`]))
  if (listed.exitCode !== 0) {
    return [`kept.branch=${job.branch}: git for-each-ref failed, got ${why(listed)}`]
  }
  const lines: string[] = []
  for (const branch of namesOf(listed)) {
    const isEmpty = async (): Promise<boolean> => !(await hadOwnCommit($, job.repo, branch).catch(() => true))
    if (!(await isOnBase($, landed, branch)) && !(await isEmpty())) {
      lines.push(`kept.branch=${branch}: it holds commits that are not on ${landed.ref}`)
      continue
    }
    const deleted = await attempt(() => git($, job.repo, ['branch', '-D', branch]))
    lines.push(deleted.exitCode === 0 ? `removed.branch=${branch}` : `kept.branch=${branch}: ${why(deleted)}`)
  }
  return lines
}

/** Mode `origin`: each branch of the job on origin whose work is on the base, deleted with one push that names them; nothing is force-pushed. */
async function cleanRemoteBranches($: EngineInterface, job: RexJob, landed: Landed, fetched: Git): Promise<readonly string[]> {
  const listed = await attempt(() => git($, job.repo, ['for-each-ref', '--format=%(refname:lstrip=3)', `refs/remotes/origin/${job.branch}`, `refs/remotes/origin/${job.branch}--*`]))
  const branches = namesOf(listed)
  if (fetched.exitCode !== 0) {
    return branches.map(branch => `kept.remote_branch=${branch}: git fetch --prune origin failed, got ${why(fetched)}`)
  }
  const lines: string[] = []
  const deleting: string[] = []
  for (const branch of branches) {
    if (await isOnBase($, landed, `refs/remotes/origin/${branch}`)) {
      deleting.push(branch)
    } else {
      lines.push(`kept.remote_branch=${branch}: it holds commits that are not on ${landed.ref}`)
    }
  }
  if (deleting.length === 0) {
    return lines
  }
  const pushed = await attempt(() => git($, job.repo, ['push', 'origin', '--delete', ...deleting]))
  return [...lines, ...deleting.map(branch => (pushed.exitCode === 0 ? `removed.remote_branch=${branch}` : `kept.remote_branch=${branch}: ${why(pushed)}`))]
}

/** The clean-up of a closed job, as `removed.` and `kept.` lines; in mode `origin` it fetches first, so the base and origin's branches are current. */
export async function cleanUp($: EngineInterface, job: RexJob): Promise<readonly string[]> {
  const places = await repoPlaces($, job.repo).catch(() => null)
  if (places === null) {
    return [`kept.worktree=${job.worktree ?? jobWorktree(job.repo, job.id)}: the places of ${job.repo} could not be read`]
  }
  const ref = baseRef(places)
  const fetched = places.remote === 'origin' ? await attempt(() => git($, job.repo, ['fetch', '--prune', 'origin'])) : null
  const landed: Landed = { repo: job.repo, ref, squashed: await squashedTip($, job, ref) }
  const worktrees = await cleanWorktrees($, job)
  const branches = await cleanBranches($, job, landed)
  const remote = fetched === null ? [] : await cleanRemoteBranches($, job, landed, fetched)
  return [...worktrees, ...branches, ...remote]
}

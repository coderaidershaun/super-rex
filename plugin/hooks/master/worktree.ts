// Makes and removes the worktree of one tree job with rex worktree, which also lists it for the sandbox, so the builder's git commits work there.
import type { EngineInterface } from 'claude-code'

const GIT_TIMEOUT_MS = 120_000

export type GitRun = { readonly exitCode: number; readonly stdout: string; readonly firstError: string }

async function run($: EngineInterface, repo: string, argv: readonly string[], timeoutMs: number): Promise<GitRun> {
  const ran = await $.process.run(argv, { cwd: repo, timeoutMs })
  return { exitCode: ran.exitCode, stdout: ran.stdout.trim(), firstError: ran.stderr.split('\n')[0]?.trim() ?? '' }
}

export function git($: EngineInterface, repo: string, args: readonly string[], timeoutMs: number = GIT_TIMEOUT_MS): Promise<GitRun> {
  return run($, repo, ['git', '-C', repo, ...args], timeoutMs)
}

/** The branch origin's HEAD names, without `origin/`; `main` when origin names none. */
export async function baseOf($: EngineInterface, repo: string): Promise<string> {
  const named = await git($, repo, ['symbolic-ref', '--short', 'refs/remotes/origin/HEAD'])
  return named.exitCode === 0 && named.stdout.startsWith('origin/') ? named.stdout.slice('origin/'.length) : 'main'
}

export type GitFailure = { readonly command: string; readonly exitCode: number; readonly firstError: string }

/**
 * Fetches the base, reads the full hash of `origin/<base>`, then makes the
 * job's worktree on a new job branch from that commit; null when each passed.
 */
export async function addJobWorktree($: EngineInterface, place: { readonly repo: string; readonly base: string; readonly branch: string; readonly worktree: string }): Promise<GitFailure | null> {
  const fetched = await git($, place.repo, ['fetch', 'origin', place.base])
  if (fetched.exitCode !== 0) {
    return { command: `git fetch origin ${place.base}`, exitCode: fetched.exitCode, firstError: fetched.firstError }
  }
  const start = await git($, place.repo, ['rev-parse', `origin/${place.base}`])
  if (start.exitCode !== 0 || start.stdout === '') {
    return { command: `git rev-parse origin/${place.base}`, exitCode: start.exitCode, firstError: start.firstError }
  }
  const added = await run($, place.repo, ['rex', 'worktree', 'add', place.repo, place.worktree, start.stdout, '--branch', place.branch], GIT_TIMEOUT_MS)
  return added.exitCode === 0 ? null : { command: 'rex worktree add', exitCode: added.exitCode, firstError: added.firstError }
}

/** Whether git lists `worktree` as a worktree of `repo`: a resumed job's worktree outlives the process that made it. */
export async function hasWorktree($: EngineInterface, repo: string, worktree: string): Promise<boolean> {
  const listed = await git($, repo, ['worktree', 'list', '--porcelain'])
  return listed.exitCode === 0 && listed.stdout.split('\n').includes(`worktree ${worktree}`)
}

/** Removes and unlists the worktree of a job that never started; the result is the answer of rex worktree, which the caller reports. */
export function removeJobWorktree($: EngineInterface, repo: string, worktree: string): Promise<GitRun> {
  return run($, repo, ['rex', 'worktree', 'remove', repo, worktree], GIT_TIMEOUT_MS)
}

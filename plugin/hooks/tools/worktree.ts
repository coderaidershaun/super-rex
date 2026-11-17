// Makes and removes the worktrees of a job with rex worktree, which also lists each one for the file tools, so no agent of the job runs a worktree command: the job worktree when the job opens, a unit worktree when an agent asks.
import type { EngineInterface } from 'claude-code'

import { attempt, git, rexWorktree } from './git'
import type { Git } from './git'
import { baseRef } from './places'
import type { RepoPlaces } from './places'

export type GitFailure = { readonly command: string; readonly exitCode: number; readonly firstError: string }

/** The commit a new job branch starts from; stale when the fetch failed and the base is the one this repo last fetched from origin. */
export type BaseStart = { readonly kind: 'start'; readonly commit: string; readonly isStale: boolean } | ({ readonly kind: 'failed' } & GitFailure)

/**
 * The full hash of the base: `origin/<base>` after a fetch in mode `origin`,
 * the local base in mode `none`. A fetch that fails stops nothing while this
 * repo still knows `origin/<base>`; a status line then says the base may be stale.
 */
export async function baseStart($: EngineInterface, places: RepoPlaces): Promise<BaseStart> {
  const ref = baseRef(places)
  const fetched = places.remote === 'origin' ? await git($, places.repo, ['fetch', 'origin', places.base]) : null
  const isFetchFailed = fetched !== null && fetched.exitCode !== 0
  const start = await git($, places.repo, ['rev-parse', ref])
  if (start.exitCode !== 0 || start.stdout === '') {
    // With no base ref, the fetch that failed says more than the rev-parse that followed it.
    return isFetchFailed
      ? { kind: 'failed', command: `git fetch origin ${places.base}`, exitCode: fetched.exitCode, firstError: fetched.firstError }
      : { kind: 'failed', command: `git rev-parse ${ref}`, exitCode: start.exitCode, firstError: start.firstError }
  }
  if (isFetchFailed) {
    $.ui.status(`rex: git fetch origin ${places.base} failed, got ${fetched.firstError}; the job starts from ${ref} as this repo last fetched it, which may be stale`)
  }
  return { kind: 'start', commit: start.stdout, isStale: isFetchFailed }
}

export type WorktreePlace = { readonly repo: string; readonly branch: string; readonly worktree: string }

/**
 * Makes a worktree on its branch at `commit`, a new branch, or one whose tip
 * `commit` is; null when it was made, else the failure. A call that threw is a
 * failure too, so the caller cancels the job it opened for this worktree.
 */
export async function addWorktree($: EngineInterface, place: WorktreePlace, commit: string): Promise<GitFailure | null> {
  const added = await attempt(() => rexWorktree($, place.repo, ['add', place.repo, place.worktree, commit, '--branch', place.branch]))
  return added.exitCode === 0 ? null : { command: 'rex worktree add', exitCode: added.exitCode, firstError: added.firstError }
}

/** The path of each worktree git lists for `repo`, the main checkout first; empty when git cannot list them. */
async function worktreePaths($: EngineInterface, repo: string): Promise<readonly string[]> {
  const listed = await git($, repo, ['worktree', 'list', '--porcelain'])
  return listed.exitCode === 0 ? listed.stdout.split('\n').filter(line => line.startsWith('worktree ')).map(line => line.slice('worktree '.length)) : []
}

/** Whether git lists `worktree` as a worktree of `repo`: a resumed job's worktree outlives the process that made it. */
export async function hasWorktree($: EngineInterface, repo: string, worktree: string): Promise<boolean> {
  return (await worktreePaths($, repo)).includes(worktree)
}

/** Removes and unlists a worktree without force, so git keeps one that holds uncommitted or untracked files. */
export function removeWorktree($: EngineInterface, repo: string, worktree: string): Promise<Git> {
  return rexWorktree($, repo, ['remove', repo, worktree])
}

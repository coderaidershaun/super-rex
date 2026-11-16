// A repo's places as git holds them now: its base branch and whether that base lives on `origin` or only here, and the first commit a repo with none gets, or the refusal when it cannot get one.
import type { EngineInterface } from 'claude-code'

import { assertNever } from '../book/index'
import { git } from './git'

const FIRST_COMMIT_MESSAGE = 'chore: first commit, the base of rex jobs'

/** The exit code of `git rev-parse` in a folder that no repo holds; a repo with no commit exits 1. */
const NOT_A_REPO_EXIT = 128

export type RepoPlaces = { readonly repo: string; readonly base: string; readonly remote: 'origin' | 'none' }

/** The base of a detached HEAD: `main` when that branch exists, else `master` when it does, else `main`. */
async function detachedBase($: EngineInterface, repo: string): Promise<string> {
  const heads = await git($, repo, ['for-each-ref', '--format=%(refname:short)', 'refs/heads/main', 'refs/heads/master'])
  return heads.stdout.split('\n')[0] || 'main'
}

/**
 * Mode `origin` when `origin` holds the base: the branch `origin/HEAD` names,
 * else the branch checked out here, else `main`. Mode `none` otherwise, with
 * the branch checked out here as base, or with HEAD detached `main` or `master`.
 */
export async function repoPlaces($: EngineInterface, repo: string): Promise<RepoPlaces> {
  const head = await git($, repo, ['symbolic-ref', '--quiet', '--short', 'HEAD'])
  const checkedOut = head.exitCode === 0 ? head.stdout : ''
  const local: RepoPlaces = { repo, base: checkedOut === '' ? await detachedBase($, repo) : checkedOut, remote: 'none' }
  const remotes = await git($, repo, ['remote'])
  if (remotes.exitCode !== 0 || !remotes.stdout.split('\n').includes('origin')) {
    return local
  }
  const named = await git($, repo, ['symbolic-ref', '--short', 'refs/remotes/origin/HEAD'])
  const heads = await git($, repo, ['for-each-ref', '--format=%(refname)', 'refs/remotes/origin'])
  const held = heads.exitCode === 0 ? heads.stdout.split('\n') : []
  const originHead = named.exitCode === 0 && named.stdout.startsWith('origin/') ? named.stdout.slice('origin/'.length) : ''
  const base = [originHead, checkedOut, 'main'].find(name => name !== '' && held.includes(`refs/remotes/origin/${name}`))
  return base === undefined ? local : { repo, base, remote: 'origin' }
}

/** The ref a job branch starts from and is checked against: `origin/<base>` in mode `origin`, the local base in mode `none`. */
export function baseRef(places: Pick<RepoPlaces, 'base' | 'remote'>): string {
  return places.remote === 'origin' ? `origin/${places.base}` : places.base
}

export type FirstCommit =
  | { readonly kind: 'had' }
  | { readonly kind: 'made' }
  | { readonly kind: 'no-repo'; readonly firstError: string }
  | { readonly kind: 'failed'; readonly command: string; readonly exitCode: number; readonly firstError: string }

/**
 * Makes the first commit of a repo that has none, of each file `.gitignore`
 * does not exclude, with the repo's own commit hooks; a job needs a commit to
 * start from. A repo with no file at all gets an empty commit. When the add
 * or the commit fails, the index goes back to what the person had staged, so
 * nothing is left half done. A folder no repo holds gets nothing.
 */
export async function firstCommit($: EngineInterface, repo: string): Promise<FirstCommit> {
  const head = await git($, repo, ['rev-parse', '--verify', '--quiet', 'HEAD'])
  if (head.exitCode === 0) {
    return { kind: 'had' }
  }
  if (head.exitCode === NOT_A_REPO_EXIT) {
    return { kind: 'no-repo', firstError: head.firstError }
  }
  const staged = await git($, repo, ['write-tree'])
  if (staged.exitCode !== 0) {
    return { kind: 'failed', command: 'git write-tree', exitCode: staged.exitCode, firstError: staged.firstError }
  }
  const added = await git($, repo, ['add', '--all'])
  const committed = added.exitCode === 0 ? await git($, repo, ['commit', '--allow-empty', '--message', FIRST_COMMIT_MESSAGE]) : added
  if (committed.exitCode !== 0) {
    await git($, repo, ['read-tree', staged.stdout])
    return { kind: 'failed', command: added.exitCode === 0 ? 'git commit' : 'git add --all', exitCode: committed.exitCode, firstError: committed.firstError }
  }
  $.ui.status(`rex: made the first commit of ${repo}, the base of rex jobs`)
  return { kind: 'made' }
}

/** The refusal of `tool` when the first commit of `repo` was not made, or null when the repo has its first commit. */
export function firstCommitRefusal(tool: string, repo: string, made: FirstCommit, todo: 'start' | 'call'): string | null {
  switch (made.kind) {
    case 'had':
    case 'made':
      return null
    case 'no-repo':
      return `${tool}: ${repo} is not a git repo, got ${made.firstError}; run git init there, then start again`
    case 'failed': {
      const commands = `git -C ${repo} add --all and git -C ${repo} commit`
      const failed = `${tool}: the first commit of ${repo} failed, got ${made.command} exit ${String(made.exitCode)}: ${made.firstError}`
      return todo === 'start'
        ? `${failed}; correct what it names, commit the files yourself with ${commands}, then start the job again`
        : `${failed}; ask the person to correct what it names and commit the files with ${commands}, then call ${tool} again`
    }
    default:
      return assertNever(made)
  }
}

/** The count of paths `git status --porcelain` lists in the main checkout: changes a job does not see, since it starts from the last commit. */
export async function uncommittedCount($: EngineInterface, repo: string): Promise<number> {
  const status = await git($, repo, ['status', '--porcelain'])
  return status.stdout.split('\n').filter(line => line.trim() !== '').length
}

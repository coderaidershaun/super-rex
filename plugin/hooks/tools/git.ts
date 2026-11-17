// The one way the mod runs git and `rex worktree`: one timeout, and no prompt for a password, so a remote that asks for one fails at once and does not hang; a call that throws can be read as one that failed.
import type { EngineInterface } from 'claude-code'

import { errorText } from '../book/index'

const GIT_TIMEOUT_MS = 120_000

/** Over the host's own environment; git then fails at once where it would ask for a user name or a password. */
const GIT_ENV: Readonly<Record<string, string>> = { GIT_TERMINAL_PROMPT: '0' }

/** `listed` holds the paths git names on the tab-indented lines after an error. */
export type Git = { readonly exitCode: number; readonly stdout: string; readonly firstError: string; readonly listed: readonly string[] }

/** A line that names a failure, as git or a hook writes one; a hook's lines of checks that passed come before it. */
const FAILURE_LINE = /\b(error|fatal|fail|failed|failure)\b/i

async function run($: EngineInterface, repo: string, argv: readonly string[]): Promise<Git> {
  const ran = await $.process.run(argv, { cwd: repo, env: { ...GIT_ENV }, timeoutMs: GIT_TIMEOUT_MS })
  // Git hands a hook's stdout to stderr, so stderr comes first; the first line that names a failure wins over a hook's first line.
  const lines = [...ran.stderr.split('\n'), ...ran.stdout.split('\n')].map(line => line.trim()).filter(line => line !== '')
  const listed = ran.stderr.split('\n').filter(line => line.startsWith('\t')).map(line => line.trim())
  return { exitCode: ran.exitCode, stdout: ran.stdout.trim(), firstError: lines.find(line => FAILURE_LINE.test(line)) ?? lines[0] ?? '', listed }
}

/** A call that threw, a timeout for one, reads as a call that failed, so the step that made it can say why and undo what it began. */
export async function attempt(call: () => Promise<Git>): Promise<Git> {
  try {
    return await call()
  } catch (error) {
    return { exitCode: -1, stdout: '', firstError: errorText(error), listed: [] }
  }
}

/** `git -C <repo> <args>`, run in `repo`. */
export function git($: EngineInterface, repo: string, args: readonly string[]): Promise<Git> {
  return run($, repo, ['git', '-C', repo, ...args])
}

/**
 * True when a commit was ever made on the branch, by its own reflog. A branch
 * with none holds no work of its own, even when its tip sits on the base. A
 * reflog that cannot be read counts as a commit, so a branch whose history is
 * unknown is never taken for one with no work.
 */
export async function hadOwnCommit($: EngineInterface, repo: string, branch: string): Promise<boolean> {
  const reflog = await git($, repo, ['reflog', 'show', '--format=%gs', `refs/heads/${branch}`, '--'])
  const moves = reflog.stdout.split('\n').filter(line => line.length > 0)
  const isCommit = (move: string): boolean => /^(commit|merge|cherry-pick|revert)\b/.test(move)
  return reflog.exitCode !== 0 || moves.length === 0 || moves.some(isCommit)
}

/** `rex worktree <args>`, run in `repo`; it runs git itself, and lists or unlists the worktree for the file tools. */
export function rexWorktree($: EngineInterface, repo: string, args: readonly string[]): Promise<Git> {
  return run($, repo, ['rex', 'worktree', ...args])
}

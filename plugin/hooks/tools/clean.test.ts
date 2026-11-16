// Proves the clean-up of rex_job_close with the answers real git gave: a merged job leaves nothing in either mode, a dirty worktree and an unmerged branch stay, an empty branch goes, and a step that fails keeps its thing with the reason while the close stands.
import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { mergeDoneIn } from '../book/jobs'
import { ROOT, START_MS, world } from '../book/kit'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { exit, files, JOB, LOCAL_PLACES, ok, ORIGIN_PLACES, resultText, scripted, seededBook, worktreeList } from './act-kit'
import type { Scripted } from './act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const CLOSE = 'mcp__super-rex__rex_job_close'
const BRANCH = `rex/${JOB}`
const JOB_DIR = `${ROOT}.worktrees/${JOB}`
const UNIT_DIR = `${JOB_DIR}--parser`
const MERGE = '3f2a9c1e8b7d6a5f4e3d2c1b0a9f8e7d6c5b4a39'
const WORK_TIP = '77aa88bb99cc00dd11ee22ff33aa44bb55cc66dd'
const TREE = 'e1b2c3d4e5f60718293a4b5c6d7e8f9001122334'
/** What `rex worktree remove` printed for a worktree with an untracked file. */
const DIRTY = exit(1, `error: "git worktree remove ${UNIT_DIR}" failed: fatal: '${UNIT_DIR}' contains modified or untracked files, use --force to delete it\n`)
/** What `git branch -D` printed for the branch of a worktree that stays. */
const CHECKED_OUT = exit(1, `error: Cannot delete branch '${BRANCH}--parser' checked out at '${UNIT_DIR}'\n`)
const NOT_ANCESTOR = exit(1, '')

function gitCall(args: readonly string[], answer: ProcessAnswer = ok()): Scripted {
  return { argv: ['git', '-C', ROOT, ...args], answer }
}

function removeCall(path: string, answer: ProcessAnswer = ok(`worktree=${path}\nsettings=removed\n`)): Scripted {
  return { argv: ['rex', 'worktree', 'remove', ROOT, path], answer }
}

/** The seeded job with the squash commit MERGE that rex_merge merged. */
function mergedBook(): RexBook {
  return mergeDoneIn(seededBook(), JOB, { repo: ROOT, base: 'main', branch: BRANCH, mergeCommit: MERGE }, START_MS)
}

/** The tip of `--work` and the trees of it and of the merge commit: the same tree, so the squash holds the work. */
function squashCalls(ref: string): readonly Scripted[] {
  return [
    gitCall(['rev-parse', `refs/heads/${BRANCH}--work`, `refs/heads/${BRANCH}--work^{tree}`, `${MERGE}^{tree}`], ok(`${WORK_TIP}\n${TREE}\n${TREE}\n`)),
    gitCall(['merge-base', '--is-ancestor', MERGE, ref]),
  ]
}

const WORKTREES = gitCall(['worktree', 'list', '--porcelain'], worktreeList([{ path: JOB_DIR, branch: BRANCH }, { path: UNIT_DIR, branch: `${BRANCH}--parser` }]))
const HEADS = gitCall(['for-each-ref', '--format=%(refname:short)', `refs/heads/${BRANCH}`, `refs/heads/${BRANCH}--*`], ok(`${BRANCH}\n${BRANCH}--parser\n${BRANCH}--work\n`))

/** Each local branch of a merged job: the job branch on the base, the unit branch and `--work` below the squashed tip. */
function mergedBranches(ref: string): readonly Scripted[] {
  return [
    gitCall(['merge-base', '--is-ancestor', BRANCH, ref]),
    gitCall(['branch', '-D', BRANCH]),
    gitCall(['merge-base', '--is-ancestor', `${BRANCH}--parser`, ref], NOT_ANCESTOR),
    gitCall(['merge-base', '--is-ancestor', `${BRANCH}--parser`, WORK_TIP]),
    gitCall(['branch', '-D', `${BRANCH}--parser`]),
    gitCall(['merge-base', '--is-ancestor', `${BRANCH}--work`, ref], NOT_ANCESTOR),
    gitCall(['merge-base', '--is-ancestor', `${BRANCH}--work`, WORK_TIP]),
    gitCall(['branch', '-D', `${BRANCH}--work`]),
  ]
}

function cleanLines(result: unknown): readonly string[] {
  return resultText(result)
    .split('\n')
    .filter(line => line.startsWith('removed.') || line.startsWith('kept.'))
}

async function closeMerged($: Engine): Promise<unknown> {
  return $.tool.call({ tool: CLOSE, status: 'delivered', outcome: 'merged', deliveries: [{ repo: ROOT, outcome: 'merged' }] })
}

test('the close of a merged job in mode none removes each worktree and branch of the job, and asks nothing of origin', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  const steps = [...LOCAL_PLACES, ...squashCalls('main'), WORKTREES, removeCall(JOB_DIR), removeCall(UNIT_DIR), HEADS, ...mergedBranches('main')]
  const session = world(on, { seed: mergedBook(), processAnswer: scripted(steps, calls) })
  await $.session.start(START)

  const closed = await closeMerged($)

  expect(cleanLines(closed)).toEqual([`removed.worktree=${JOB_DIR}`, `removed.worktree=${UNIT_DIR}`, `removed.branch=${BRANCH}`, `removed.branch=${BRANCH}--parser`, `removed.branch=${BRANCH}--work`])
  expect(calls.map(call => call.argv)).toEqual(steps.map(step => step.argv))
  expect(session.book().jobs[0]?.status).toBe('delivered')
})

test('the close of a merged job in mode origin fetches first, then deletes its branches on origin with one push that names them', async ($, on) => {
  files(on, {})
  const remotes = gitCall(['for-each-ref', '--format=%(refname:lstrip=3)', `refs/remotes/origin/${BRANCH}`, `refs/remotes/origin/${BRANCH}--*`], ok(`${BRANCH}\n${BRANCH}--parser\n`))
  const steps = [
    ...ORIGIN_PLACES,
    gitCall(['fetch', '--prune', 'origin']),
    ...squashCalls('origin/main'),
    WORKTREES,
    removeCall(JOB_DIR),
    removeCall(UNIT_DIR),
    HEADS,
    ...mergedBranches('origin/main'),
    remotes,
    gitCall(['merge-base', '--is-ancestor', `refs/remotes/origin/${BRANCH}`, 'origin/main'], NOT_ANCESTOR),
    gitCall(['merge-base', '--is-ancestor', `refs/remotes/origin/${BRANCH}`, WORK_TIP]),
    gitCall(['merge-base', '--is-ancestor', `refs/remotes/origin/${BRANCH}--parser`, 'origin/main'], NOT_ANCESTOR),
    gitCall(['merge-base', '--is-ancestor', `refs/remotes/origin/${BRANCH}--parser`, WORK_TIP]),
    gitCall(['push', 'origin', '--delete', BRANCH, `${BRANCH}--parser`], ok('')),
  ]
  const calls: ProcessCall[] = []
  world(on, { seed: mergedBook(), processAnswer: scripted(steps, calls) })
  await $.session.start(START)

  const closed = await closeMerged($)

  expect(cleanLines(closed).slice(5)).toEqual([`removed.remote_branch=${BRANCH}`, `removed.remote_branch=${BRANCH}--parser`])
  expect(calls.map(call => call.argv)).toEqual(steps.map(step => step.argv))
})

test('a worktree with untracked files stays with its branch, a branch with work not on the base stays, and a branch with no commit of its own goes', async ($, on) => {
  files(on, {})
  const heads = gitCall(['for-each-ref', '--format=%(refname:short)', `refs/heads/${BRANCH}`, `refs/heads/${BRANCH}--*`], ok(`${BRANCH}\n${BRANCH}--lexer\n${BRANCH}--parser\n`))
  const steps = [
    ...LOCAL_PLACES,
    WORKTREES,
    removeCall(JOB_DIR),
    removeCall(UNIT_DIR, DIRTY),
    heads,
    gitCall(['merge-base', '--is-ancestor', BRANCH, 'main'], NOT_ANCESTOR),
    gitCall(['reflog', 'show', '--format=%gs', `refs/heads/${BRANCH}`, '--'], ok(`commit: parser\nbranch: Created from ${WORK_TIP}\n`)),
    gitCall(['merge-base', '--is-ancestor', `${BRANCH}--lexer`, 'main'], NOT_ANCESTOR),
    gitCall(['reflog', 'show', '--format=%gs', `refs/heads/${BRANCH}--lexer`, '--'], ok(`branch: Created from ${WORK_TIP}\n`)),
    gitCall(['branch', '-D', `${BRANCH}--lexer`]),
    gitCall(['merge-base', '--is-ancestor', `${BRANCH}--parser`, 'main']),
    gitCall(['branch', '-D', `${BRANCH}--parser`], CHECKED_OUT),
  ]
  world(on, { seed: seededBook(), processAnswer: scripted(steps) })
  await $.session.start(START)

  const closed = await $.tool.call({ tool: CLOSE, status: 'blocked', outcome: 'stopped at the parser', deliveries: [] })

  expect(cleanLines(closed)).toEqual([
    `removed.worktree=${JOB_DIR}`,
    `kept.worktree=${UNIT_DIR}: error: "git worktree remove ${UNIT_DIR}" failed: fatal: '${UNIT_DIR}' contains modified or untracked files, use --force to delete it`,
    `kept.branch=${BRANCH}: it holds commits that are not on main`,
    `removed.branch=${BRANCH}--lexer`,
    `kept.branch=${BRANCH}--parser: error: Cannot delete branch '${BRANCH}--parser' checked out at '${UNIT_DIR}'`,
  ])
})

test('a clean-up step that fails keeps its thing with the reason, and the close is still recorded and answered', async ($, on) => {
  files(on, {})
  const steps = [
    ...ORIGIN_PLACES,
    gitCall(['fetch', '--prune', 'origin']),
    ...squashCalls('origin/main'),
    gitCall(['worktree', 'list', '--porcelain'], worktreeList([{ path: JOB_DIR, branch: BRANCH }])),
    { argv: ['rex', 'worktree', 'remove', ROOT, JOB_DIR], answer: { reject: 'rex worktree remove timed out after 120000 ms' } },
    gitCall(['for-each-ref', '--format=%(refname:short)', `refs/heads/${BRANCH}`, `refs/heads/${BRANCH}--*`], ok('')),
    gitCall(['for-each-ref', '--format=%(refname:lstrip=3)', `refs/remotes/origin/${BRANCH}`, `refs/remotes/origin/${BRANCH}--*`], ok(`${BRANCH}\n`)),
    gitCall(['merge-base', '--is-ancestor', `refs/remotes/origin/${BRANCH}`, 'origin/main']),
    gitCall(['push', 'origin', '--delete', BRANCH], exit(128, "fatal: could not read Username for 'https://github.com': terminal prompts disabled\n")),
  ]
  const session = world(on, { seed: mergedBook(), processAnswer: scripted(steps) })
  await $.session.start(START)

  const closed = await closeMerged($)

  expect(resultText(closed).startsWith(`job=${JOB}\nstatus=delivered\n`)).toBe(true)
  expect(cleanLines(closed)).toEqual([
    `kept.worktree=${JOB_DIR}: HooksError: super-rex: $.process.run: rex worktree remove timed out after 120000 ms`,
    `kept.remote_branch=${BRANCH}: fatal: could not read Username for 'https://github.com': terminal prompts disabled`,
  ])
  expect(session.book().jobs[0]?.status).toBe('delivered')
})

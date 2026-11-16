// Proves rex_merge in mode none, a repo with no origin: the same checks against the local base, no fetch or push, and a fast-forward of the local base.
import { expect, test } from 'claude-code/testing'

import type { RexBook } from '../../types'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { ROOT, START_MS, world } from '../book/kit'
import { exit, files, JOB, LOCAL_PLACES, ok, plannedAll, resultText, scripted, seededBook } from '../tools/act-kit'
import type { Scripted } from '../tools/act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const BRANCH = `rex/${JOB}`
const JOB_DIR = `${ROOT}.worktrees/${JOB}`

function gitIn(dir: string, args: readonly string[], answer: ProcessAnswer = ok()): Scripted {
  return { argv: ['git', '-C', dir, ...args], answer }
}

function gitCall(args: readonly string[], answer: ProcessAnswer = ok()): Scripted {
  return gitIn(ROOT, args, answer)
}

const CHECKS: readonly Scripted[] = [
  ...LOCAL_PLACES,
  gitCall(['rev-list', '--count', `main..${BRANCH}`], ok('1\n')),
  gitCall(['merge-base', '--is-ancestor', 'main', BRANCH]),
  gitCall(['diff', '--name-only', `main...${BRANCH}`], ok('src/a.rs\n')),
]

/** `git worktree list --porcelain` of the main checkout and the job's worktree; `base` is the branch the main checkout has, or a detached HEAD. */
function worktreeList(base: string | null): Scripted {
  const main = base === null ? `worktree ${ROOT}\nHEAD c0\ndetached\n` : `worktree ${ROOT}\nHEAD c0\nbranch refs/heads/${base}\n`
  return gitCall(['worktree', 'list', '--porcelain'], ok(`${main}\nworktree ${JOB_DIR}\nHEAD c1\nbranch refs/heads/${BRANCH}\n`))
}

function yes(book: RexBook): RexBook {
  return { ...book, jobs: book.jobs.map(job => ({ ...job, merge: { answer: 'yes', source: 'dialog', question: null, policyHash: null, at: START_MS } })) }
}

function noFetchOrPush(calls: readonly ProcessCall[]): boolean {
  return calls.every(call => !call.argv.includes('fetch') && !call.argv.includes('push'))
}

test('rex_merge in mode none fast-forwards the base where the main checkout has it, so the person sees the files there', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  const steps = [...CHECKS, worktreeList('main'), gitCall(['merge', '--ff-only', BRANCH]), gitCall(['rev-parse', BRANCH], ok('c1\n'))]
  const session = world(on, { seed: yes(plannedAll(seededBook())), processAnswer: scripted(steps, calls) })
  await $.session.start(START)

  const merged = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(resultText(merged)).toBe(`merged=yes\nmerge_commit=c1\nbase=main\nbranch=${BRANCH}\n`)
  expect(calls.map(call => call.argv)).toEqual(steps.map(one => one.argv))
  expect(noFetchOrPush(calls)).toBe(true)
  expect(session.book().jobs[0]?.deliveries).toEqual([{ repo: ROOT, branch: BRANCH, pr: null, outcome: 'merged', mergeCommit: 'c1', note: null }])
})

test('rex_merge in mode none moves a base that no worktree has checked out with a guarded update of its ref', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  const steps = [
    ...CHECKS,
    worktreeList(null),
    gitCall(['rev-parse', BRANCH, 'refs/heads/main'], ok('c1\nc0\n')),
    gitCall(['merge-base', '--is-ancestor', 'c0', 'c1']),
    gitCall(['update-ref', 'refs/heads/main', 'c1', 'c0']),
    gitCall(['rev-parse', BRANCH], ok('c1\n')),
  ]
  world(on, { seed: yes(plannedAll(seededBook())), processAnswer: scripted(steps, calls) })
  await $.session.start(START)

  const merged = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(resultText(merged)).toBe(`merged=yes\nmerge_commit=c1\nbase=main\nbranch=${BRANCH}\n`)
  expect(calls.map(call => call.argv)).toEqual(steps.map(one => one.argv))
})

test('rex_merge in mode none does not update a base that no worktree has checked out once it moved off the job branch, and records no merge', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  const steps = [...CHECKS, worktreeList(null), gitCall(['rev-parse', BRANCH, 'refs/heads/main'], ok('c1\nc9\n')), gitCall(['merge-base', '--is-ancestor', 'c9', 'c1'], exit(1, ''))]
  const session = world(on, { seed: yes(plannedAll(seededBook())), processAnswer: scripted(steps, calls) })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(refused.deny).toBe(`rex_merge: main moved, got ${BRANCH} not on it; rebase the squash commit and call again`)
  expect(calls.map(call => call.argv)).toEqual(steps.map(one => one.argv))
  expect(session.book().jobs[0]?.deliveries).toEqual([])
})

/** What real git 2.39 gave for `git merge --ff-only` in a checkout that holds an untracked file the merge would write: the paths come on the lines after the first, each after a tab. */
const UNTRACKED_IN_THE_WAY = exit(1, 'error: The following untracked working tree files would be overwritten by merge:\n\tnew.txt\nPlease move or remove them before you merge.\nAborting\n')

test('rex_merge in mode none refuses when the checkout holds changes the fast-forward would overwrite, names them, and records no merge', async ($, on) => {
  files(on, {})
  const steps = [...CHECKS, worktreeList('main'), gitCall(['merge', '--ff-only', BRANCH], UNTRACKED_IN_THE_WAY)]
  const session = world(on, { seed: yes(plannedAll(seededBook())), processAnswer: scripted(steps) })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(refused.deny).toBe(
    `rex_merge: fast-forward of main in ${ROOT} failed, got exit 1: error: The following untracked working tree files would be overwritten by merge: new.txt; commit or remove the changes there that the merge would overwrite, then call again`,
  )
  expect(session.book().jobs[0]?.deliveries).toEqual([])
})

test('rex_merge in mode none fast-forwards the base in the worktree that has it checked out when that is not the main checkout', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  const other = `${ROOT}.worktrees/person-main`
  const list = gitCall(['worktree', 'list', '--porcelain'], ok(`worktree ${ROOT}\nHEAD c0\nbranch refs/heads/topic\n\nworktree ${other}\nHEAD c0\nbranch refs/heads/main\n\nworktree ${JOB_DIR}\nHEAD c1\nbranch refs/heads/${BRANCH}\n`))
  const steps = [...CHECKS, list, gitIn(other, ['merge', '--ff-only', BRANCH]), gitCall(['rev-parse', BRANCH], ok('c1\n'))]
  world(on, { seed: yes(plannedAll(seededBook())), processAnswer: scripted(steps, calls) })
  await $.session.start(START)

  const merged = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(resultText(merged)).toBe(`merged=yes\nmerge_commit=c1\nbase=main\nbranch=${BRANCH}\n`)
  expect(calls.map(call => call.argv)).toEqual(steps.map(one => one.argv))
})

test('rex_merge in mode none still refuses a branch that is not one squash commit over the local base, and a job with no merge answer', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  const steps = [...LOCAL_PLACES, gitCall(['rev-list', '--count', `main..${BRANCH}`], ok('3\n')), ...LOCAL_PLACES, gitCall(['rev-list', '--count', `main..${BRANCH}`], ok('1\n')), gitCall(['merge-base', '--is-ancestor', 'main', BRANCH], exit(1, '')), ...CHECKS]
  const session = world(on, { seed: plannedAll(seededBook()), processAnswer: scripted(steps, calls) })
  await $.session.start(START)

  const many = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })
  const moved = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })
  const unanswered = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(many.deny).toBe(`rex_merge: ${BRANCH} must be one squash commit over main, got 3 commits; squash it and call again`)
  expect(moved.deny).toBe(`rex_merge: main moved, got ${BRANCH} not on it; rebase the squash commit and call again`)
  expect(unanswered.deny).toBe(`rex_merge: job ${JOB} has no merge answer, got none; ask the person (the builder skill's merge question), or close the job kept-local`)
  expect(noFetchOrPush(calls)).toBe(true)
  expect(session.book().jobs[0]?.deliveries).toEqual([])
})

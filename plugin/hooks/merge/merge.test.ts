// Proves rex_merge: its steps over exact git answers, the lock, the refusals, and that it pushes only the caller's own job branch after a merge answer of yes.
import { expect, test } from 'claude-code/testing'

import type { RexBook, RexMergeAnswer } from '../../types'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { ROOT, START_MS, world } from '../book/kit'
import { exit, files, JOB, ok, resultText, scripted, seededBook } from '../tools/act-kit'
import type { Scripted } from '../tools/act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const BRANCH = `rex/${JOB}`

function gitCall(args: readonly string[], answer: ProcessAnswer = ok()): Scripted {
  return { argv: ['git', '-C', ROOT, ...args], answer }
}

const CHECKS: readonly Scripted[] = [
  gitCall(['fetch', 'origin', 'main']),
  gitCall(['rev-list', '--count', `origin/main..${BRANCH}`], ok('1\n')),
  gitCall(['merge-base', '--is-ancestor', 'origin/main', BRANCH]),
  gitCall(['diff', '--name-only', `origin/main...${BRANCH}`], ok('src/a.rs\n')),
]
const PUSH: readonly Scripted[] = [gitCall(['push', 'origin', `${BRANCH}:refs/heads/main`]), gitCall(['rev-parse', BRANCH], ok('c1\n'))]

function answered(book: RexBook, merge: Omit<RexMergeAnswer, 'at'> | null): RexBook {
  return { ...book, jobs: book.jobs.map(job => ({ ...job, merge: merge === null ? null : { ...merge, at: START_MS } })) }
}

const DIALOG_YES = { answer: 'yes', source: 'dialog', question: null, policyHash: null } as const

test('rex_merge_pushes_only_own_branch_after_steps', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  const session = world(on, { seed: answered(seededBook(), DIALOG_YES), processAnswer: scripted([...CHECKS, ...PUSH], calls) })
  await $.session.start(START)

  const other = await $.tool.call({ tool: 'mcp__super-rex__rex_merge', job: '0gb0000000' })
  const merged = await $.tool.call({ tool: 'mcp__super-rex__rex_merge', branch: 'main', repo: '/elsewhere', base: 'release' })

  expect(other.deny).toBe(`rex_merge: job 0gb0000000 is not your job, got 0gb0000000; call it for ${JOB}`)
  expect(resultText(merged)).toBe(`merged=yes\nmerge_commit=c1\nbase=main\nbranch=${BRANCH}\n`)
  expect(calls.map(call => call.argv)).toEqual([...CHECKS, ...PUSH].map(one => one.argv))
  expect(calls.filter(call => call.argv.includes('push')).map(call => call.argv)).toEqual([['git', '-C', ROOT, 'push', 'origin', `${BRANCH}:refs/heads/main`]])
  expect(session.book().jobs[0]?.deliveries).toEqual([{ repo: ROOT, branch: BRANCH, pr: null, outcome: 'merged', mergeCommit: 'c1', note: null }])
})

test('rex_merge refuses while the lock of the repo is held', async ($, on) => {
  files(on, {})
  let release: (() => void) | null = null
  const held = new Promise<void>(resolve => {
    release = resolve
  })
  const answer = scripted([...CHECKS.slice(1), ...PUSH])
  on('process.run', { argv: /(?:)/ }, async (_$, e, next) => {
    if (!e.argv.includes('fetch')) {
      return next(e)
    }
    await held
    return { value: { exitCode: 0, stdout: '', stderr: '', isStdoutTruncated: false, isStderrTruncated: false } }
  })
  world(on, { seed: answered(seededBook(), DIALOG_YES), processAnswer: call => (call.argv.includes('fetch') ? ok() : answer(call)) })
  await $.session.start(START)

  const first = $.tool.call({ tool: 'mcp__super-rex__rex_merge' })
  const second = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })
  const finish: () => void = release ?? (() => undefined)
  finish()

  expect(second.deny).toBe(`rex_merge: a merge of ${ROOT} runs now, got the lock held by job ${JOB}; call again when it ends`)
  expect(resultText(await first)).toContain('merged=yes\n')
})

test('rex_merge refuses with no merge answer', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  world(on, { seed: answered(seededBook(), null), processAnswer: scripted([...CHECKS], calls) })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(refused.deny).toBe(`rex_merge: job ${JOB} has no merge answer, got none; ask the person (the builder skill's merge question), or close the job kept-local`)
  expect(calls.some(call => call.argv.includes('push'))).toBe(false)
})

test('rex_merge refuses a branch that is not one squash commit on the base, and a rejected push', async ($, on) => {
  files(on, {})
  const steps = [gitCall(['fetch', 'origin', 'main']), gitCall(['rev-list', '--count', `origin/main..${BRANCH}`], ok('3\n')), ...CHECKS.slice(0, 2), gitCall(['merge-base', '--is-ancestor', 'origin/main', BRANCH], exit(1, '')), ...CHECKS, gitCall(['push', 'origin', `${BRANCH}:refs/heads/main`], exit(1, 'error: failed to push some refs\nmore\n'))]
  world(on, { seed: answered(seededBook(), DIALOG_YES), processAnswer: scripted(steps) })
  await $.session.start(START)

  const many = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })
  const moved = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })
  const rejected = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(many.deny).toBe(`rex_merge: ${BRANCH} must be one squash commit over origin/main, got 3 commits; squash it and call again`)
  expect(moved.deny).toBe(`rex_merge: origin/main moved, got ${BRANCH} not on it; rebase the squash commit and call again`)
  expect(rejected.deny).toBe('rex_merge: push to origin/main failed, got exit 1: error: failed to push some refs; fetch, rebase and call again')
})

test('a branch with no commit over the base is asked for its squash commit; one whose own commit reached the base outside rex_merge blocks the job', async ($, on) => {
  files(on, {})
  const empty = [
    gitCall(['fetch', 'origin', 'main']),
    gitCall(['rev-list', '--count', `origin/main..${BRANCH}`], ok('0\n')),
    gitCall(['rev-parse', BRANCH], ok('b0\n')),
    gitCall(['reflog', 'show', '--format=%gs', `refs/heads/${BRANCH}`, '--'], ok('branch: Created from origin/main\n')),
  ]
  const outside = [
    gitCall(['fetch', 'origin', 'main']),
    gitCall(['rev-list', '--count', `origin/main..${BRANCH}`], ok('0\n')),
    gitCall(['rev-parse', BRANCH], ok('c9\n')),
    gitCall(['reflog', 'show', '--format=%gs', `refs/heads/${BRANCH}`, '--'], ok('commit: feat: the parser reads empty input\nbranch: Created from origin/main\n')),
  ]
  const calls: ProcessCall[] = []
  const session = world(on, { seed: answered(seededBook(), DIALOG_YES), processAnswer: scripted([...empty, ...outside], calls) })
  await $.session.start(START)

  const early = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })
  expect(early.deny).toBe(`rex_merge: ${BRANCH} must be one squash commit over origin/main, got 0 commits; squash it and call again`)
  expect(session.book().jobs[0]?.status).toBe('created')

  const halted = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })
  expect(halted.deny).toBe(`rex_merge: origin/main already holds ${BRANCH}, got commit c9 that rex_merge did not push; job ${JOB} is blocked, ask the person`)
  expect(session.book().jobs[0]?.status).toBe('blocked')
  expect(calls.some(call => call.argv.includes('push'))).toBe(false)
})

test('a tree job with no merge answer parks one merge question and names it again on the next call', async ($, on) => {
  files(on, {})
  const seed = seededBook({ builder: 'b1' })
  const treeJob = { ...seed, jobs: seed.jobs.map(job => ({ ...job, tree: 't1', origin: 'master' as const })) }
  const calls: ProcessCall[] = []
  const session = world(on, { seed: treeJob, processAnswer: scripted([...CHECKS, ...CHECKS], calls) })
  await $.session.start(START)

  const first = await $.tool.call({ tool: 'mcp__super-rex__rex_merge', agentId: 'b1' })
  const again = await $.tool.call({ tool: 'mcp__super-rex__rex_merge', agentId: 'b1' })
  expect(resultText(first)).toBe('merged=no\nquestion=q1-000000000001\n')
  expect(resultText(again)).toBe('merged=no\nquestion=q1-000000000001\n')
  expect(session.book().questions.map(question => [question.id, question.mergeFor, question.options])).toEqual([['q1-000000000001', JOB, ['Merge', 'Keep the branch']]])
  expect(calls.some(call => call.argv.includes('push'))).toBe(false)
})

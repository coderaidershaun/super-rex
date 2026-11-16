// Proves rex_worktree: the worktree of one unit made from the tip of the job branch for the builder or an agent of the job, the same lines with existing=yes on a second call, and refusals for a bad unit name and for a caller outside an open job.
import { expect, test } from 'claude-code/testing'

import { startRunIn } from '../book/runs'
import { ROOT, START_MS, world } from '../book/kit'
import type { ProcessCall } from '../book/kit'
import { exit, files, JOB, ok, resultText, scripted, seededBook, worktreeList } from './act-kit'
import type { Scripted } from './act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const TOOL = 'mcp__super-rex__rex_worktree'
const TIP = '9c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d'
const UNIT_DIR = `${ROOT}.worktrees/${JOB}--parser`
const UNIT_BRANCH = `rex/${JOB}--parser`
const MADE = `worktree=${UNIT_DIR}\nbranch=${UNIT_BRANCH}\nstart=${TIP}\n`

function gitCall(args: readonly string[], answer = ok()): Scripted {
  return { argv: ['git', '-C', ROOT, ...args], answer }
}

/** The calls of a first rex_worktree for unit parser: no such worktree yet, the tip of the job branch, then rex worktree add. */
const FIRST: readonly Scripted[] = [
  gitCall(['worktree', 'list', '--porcelain'], worktreeList([{ path: `${ROOT}.worktrees/${JOB}`, branch: `rex/${JOB}` }])),
  gitCall(['rev-parse', `refs/heads/rex/${JOB}`], ok(`${TIP}\n`)),
  { argv: ['rex', 'worktree', 'add', ROOT, UNIT_DIR, TIP, '--branch', UNIT_BRANCH], answer: ok(`worktree=${UNIT_DIR}\nsettings=added\n`) },
]

/** The calls of a repeat: git lists the unit worktree, and its branch's reflog names the commit it was made from. */
const AGAIN: readonly Scripted[] = [
  gitCall(['worktree', 'list', '--porcelain'], worktreeList([{ path: UNIT_DIR, branch: UNIT_BRANCH }])),
  gitCall(['reflog', 'show', '--format=%H', `refs/heads/${UNIT_BRANCH}`, '--'], ok(`8f00aa11bb22cc33dd44ee55ff66778899aabbcc\n${TIP}\n`)),
]

test('rex_worktree makes the unit worktree from the tip of the job branch, and a second call answers the same lines with existing=yes', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  world(on, { seed: seededBook(), processAnswer: scripted([...FIRST, ...AGAIN], calls) })
  await $.session.start(START)

  const made = await $.tool.call({ tool: TOOL, unit: 'parser' })
  const again = await $.tool.call({ tool: TOOL, unit: 'parser' })

  expect(resultText(made)).toBe(`${MADE}existing=no\n`)
  expect(resultText(again)).toBe(`${MADE}existing=yes\n`)
  expect(calls.map(call => call.argv)).toEqual([...FIRST, ...AGAIN].map(step => step.argv))
})

test('an agent of the job gets a unit worktree of that job, read from its run and never from an argument', async ($, on) => {
  files(on, {})
  const seed = startRunIn(seededBook(), { agentId: 'a1', toolUseId: null, parentAgentId: null, agentType: 'super-rex:model-high-effort-high', description: 'lead', isOwnSpawn: false }, START_MS)
  world(on, { seed, processAnswer: scripted(FIRST) })
  await $.session.start(START)

  const made = await $.tool.call({ tool: TOOL, agentId: 'a1', unit: 'parser', job: 'someone-else' })

  expect(seed.runs[0]?.job).toBe(JOB)
  expect(resultText(made)).toBe(`${MADE}existing=no\n`)
})

test('rex_worktree refuses a unit name that is not one to three lower-case words, and the reserved ones, and makes nothing', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  world(on, { seed: seededBook(), processAnswer: scripted([], calls) })
  await $.session.start(START)

  const names = ['Parser', 'a/b', 'one-two-three-four', 'work', '--branch', '']
  const refusals = []
  for (const unit of names) {
    refusals.push(await $.tool.call({ tool: TOOL, unit }))
  }
  const none = await $.tool.call({ tool: TOOL })

  expect(refusals.map(refused => refused.deny)).toEqual(
    names.map(unit => `rex_worktree: unit must be one to three lower-case words of a-z and 0-9 joined by single hyphens, and not work or review, got ${unit}; call it again with a unit name such as parser or api-client`),
  )
  expect(none.deny).toContain('got none;')
  expect(calls).toEqual([])
})

test('rex_worktree refuses the main loop once its job is closed, and an agent of no job', async ($, on) => {
  files(on, {})
  const closed = { ...seededBook(), jobs: seededBook().jobs.map(job => ({ ...job, status: 'delivered' as const })) }
  world(on, { seed: closed, processAnswer: scripted([]) })
  await $.session.start(START)

  const main = await $.tool.call({ tool: TOOL, unit: 'parser' })
  const stranger = await $.tool.call({ tool: TOOL, agentId: 'a9', unit: 'parser' })

  expect(main.deny).toBe('rex_worktree: only a builder or an agent may call it, got main')
  expect(stranger.deny).toBe('rex_worktree: only the builder or an agent of an open job may call it, got agent of no job')
})

test('a job branch with no tip refuses rex_worktree with what git said', async ($, on) => {
  files(on, {})
  world(on, { seed: seededBook(), processAnswer: scripted([FIRST[0] ?? gitCall([]), gitCall(['rev-parse', `refs/heads/rex/${JOB}`], exit(128, `fatal: ambiguous argument 'refs/heads/rex/${JOB}': unknown revision or path not in the working tree.\n`))]) })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: TOOL, unit: 'parser' })

  expect(refused.deny).toBe(
    `rex_worktree: the job branch rex/${JOB} has no tip, got exit 128: fatal: ambiguous argument 'refs/heads/rex/${JOB}': unknown revision or path not in the working tree.; call rex_status, and report the job blocked if its branch is gone`,
  )
})

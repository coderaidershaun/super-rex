// Proves the job worktree that rex_job_open makes: from origin/<base> after a fetch, or from the local base, from a stale base when the fetch fails, a refusal with no base at all, and every git call with no password prompt and one timeout.
import { expect, test } from 'claude-code/testing'

import { CONFIG_OFF, ROOT, world } from '../book/kit'
import type { ProcessCall } from '../book/kit'
import { exit, EXISTING_CALLS, files, HAS_COMMIT, LOCAL_PLACES, ok, ORIGIN_PLACES, promptsPass, resultText, scripted, START_HASH, statusCall, WORKTREE_ADD } from './act-kit'
import type { Scripted } from './act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const RESTATED = { isAnswered: true, text: 'Goal: fix the parser.', usage: { input_tokens: 1, output_tokens: 1, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 } } as const
const OPEN = 'mcp__super-rex__rex_job_open'
const JOB_START = { text: '/super-rex:orchestration-builder Fix the parser', command: { name: 'super-rex:orchestration-builder', args: 'Fix the parser' }, origin: { kind: 'composer' }, wait: false } as const

/** What real git 2.39 printed for a fetch from an origin folder that is gone. */
const ORIGIN_GONE = exit(128, "fatal: '/home/person/origin.git' does not appear to be a git repository\nfatal: Could not read from remote repository.\n\nPlease make sure you have the correct access rights\nand the repository exists.\n")

function gitCall(args: readonly string[], answer = ok()): Scripted {
  return { argv: ['git', '-C', ROOT, ...args], answer }
}

function lineOf(result: unknown, key: string): string | undefined {
  return resultText(result)
    .split('\n')
    .find(line => line.startsWith(`${key}=`))
    ?.slice(key.length + 1)
}

/** The argv of the rex worktree call of the job the book holds: its own path and branch, from the start commit. */
function addOf(job: string): readonly string[] {
  return ['rex', 'worktree', 'add', ROOT, `${ROOT}.worktrees/${job}`, START_HASH, '--branch', `rex/${job}`]
}

test('rex_job_open in mode origin fetches the base, makes the job worktree on its job branch from origin/main, and makes nothing for the same job again', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const calls: ProcessCall[] = []
  const steps = [HAS_COMMIT, ...ORIGIN_PLACES, gitCall(['fetch', 'origin', 'main']), gitCall(['rev-parse', 'origin/main'], ok(`${START_HASH}\n`)), WORKTREE_ADD, statusCall(), ...EXISTING_CALLS]
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted(steps, calls), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(JOB_START)
  const opened = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })
  const again = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  const job = session.book().jobs[0]?.id ?? ''
  expect(calls[7]?.argv).toEqual(addOf(job))
  expect(lineOf(opened, 'worktree')).toBe(`${ROOT}.worktrees/${job}`)
  expect([lineOf(opened, 'existing'), lineOf(opened, 'base_stale')]).toEqual(['no', undefined])
  expect([lineOf(again, 'existing'), lineOf(again, 'worktree')]).toEqual(['yes', `${ROOT}.worktrees/${job}`])
  expect(calls.filter(call => call.argv[1] === 'worktree')).toHaveLength(1)
})

test('rex_job_open in mode none makes the job worktree from the local base, with no fetch', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const calls: ProcessCall[] = []
  const steps = [HAS_COMMIT, ...LOCAL_PLACES, gitCall(['rev-parse', 'main'], ok(`${START_HASH}\n`)), WORKTREE_ADD, statusCall()]
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted(steps, calls), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(JOB_START)
  const opened = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  const job = session.book().jobs[0]?.id ?? ''
  expect(calls.map(call => call.argv)).toEqual([...steps.slice(0, 4).map(step => step.argv), addOf(job), steps[5]?.argv])
  expect([lineOf(opened, 'remote'), lineOf(opened, 'worktree')]).toEqual(['none', `${ROOT}.worktrees/${job}`])
})

test('a fetch that fails opens the job from origin/main as the repo last fetched it, and says the base may be stale in the answer and the status line', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const steps = [HAS_COMMIT, ...ORIGIN_PLACES, gitCall(['fetch', 'origin', 'main'], ORIGIN_GONE), gitCall(['rev-parse', 'origin/main'], ok(`${START_HASH}\n`)), WORKTREE_ADD, statusCall()]
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted(steps), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(JOB_START)
  const opened = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  expect([lineOf(opened, 'existing'), lineOf(opened, 'base_stale')]).toEqual(['no', 'yes'])
  expect(session.statuses).toContain(
    "rex: git fetch origin main failed, got fatal: '/home/person/origin.git' does not appear to be a git repository; the job starts from origin/main as this repo last fetched it, which may be stale",
  )
})

test('a fetch that fails with no origin/main known refuses rex_job_open with the fetch, and opens no job', async ($, on) => {
  files(on, {})
  promptsPass(on)
  // Real git echoes the name it cannot resolve on stdout.
  const unknown = { exitCode: 128, stdout: 'origin/main\n', stderr: "fatal: ambiguous argument 'origin/main': unknown revision or path not in the working tree.\n" }
  const steps = [HAS_COMMIT, ...ORIGIN_PLACES, gitCall(['fetch', 'origin', 'main'], ORIGIN_GONE), gitCall(['rev-parse', 'origin/main'], unknown)]
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted(steps), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(JOB_START)
  const refused = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  expect(refused.deny).toBe(
    `rex_job_open: git fetch origin main failed for ${ROOT}, got exit 128: fatal: '/home/person/origin.git' does not appear to be a git repository; make sure git -C ${ROOT} fetch origin main works, then start the job again`,
  )
  expect(session.book().jobs).toEqual([])
})

test('a job worktree that rex worktree cannot make cancels the job and refuses with its line', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const taken = exit(1, `error: ${ROOT}.worktrees/x already exists; give a path that does not exist yet\n`)
  const steps = [HAS_COMMIT, ...LOCAL_PLACES, gitCall(['rev-parse', 'main'], ok(`${START_HASH}\n`)), { argv: WORKTREE_ADD.argv, answer: taken }]
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted(steps), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(JOB_START)
  const refused = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  const job = session.book().jobs[0]
  expect(refused.deny).toBe(`rex_job_open: rex worktree add failed for job ${job?.id ?? ''}, got exit 1: error: ${ROOT}.worktrees/x already exists; give a path that does not exist yet; the job is cancelled, start the job again`)
  expect(job?.status).toBe('cancelled')
})

test('a rex worktree call that throws, as one past its timeout does, cancels the job as a failed one does, so no open job is left with no worktree', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const steps = [HAS_COMMIT, ...LOCAL_PLACES, gitCall(['rev-parse', 'main'], ok(`${START_HASH}\n`)), { argv: WORKTREE_ADD.argv, answer: { reject: 'the process ran past its timeout of 120000 ms' } }]
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted(steps), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(JOB_START)
  const refused = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  const job = session.book().jobs[0]
  expect(refused.deny).toMatch(new RegExp(`^rex_job_open: rex worktree add failed for job ${job?.id ?? ''}, got exit -1: .*timeout.*; the job is cancelled, start the job again$`))
  expect(job?.status).toBe('cancelled')
})

test('every git and rex worktree call of the mod runs in the repo with no password prompt and one timeout', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const calls: ProcessCall[] = []
  const steps = [HAS_COMMIT, ...ORIGIN_PLACES, gitCall(['fetch', 'origin', 'main']), gitCall(['rev-parse', 'origin/main'], ok(`${START_HASH}\n`)), WORKTREE_ADD, statusCall()]
  world(on, { config: CONFIG_OFF, processAnswer: scripted(steps, calls), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(JOB_START)
  await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  expect(calls).toHaveLength(steps.length)
  expect(calls.every(call => call.argv[0] === 'git' || (call.argv[0] === 'rex' && call.argv[1] === 'worktree'))).toBe(true)
  expect(calls.map(call => [call.cwd, call.env, call.timeoutMs])).toEqual(calls.map(() => [ROOT, { GIT_TERMINAL_PROMPT: '0' }, 120_000]))
})

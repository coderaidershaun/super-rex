// Proves what ingest does when rex ingest fails, by failure class: retry, renew the stream, or stop and keep every event.
import { expect, test } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { acked, emptyBook } from '../book/events'
import { limitIn, openJobIn, questionIn } from '../book/jobs'
import { CONFIG_ON, ingestAnswer, ROOT, SESSION, spawnInput, START_MS, world } from '../book/kit'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { sessionIn } from '../book/session'
import { sharedJson } from '../book/shared'
import { treeIn } from '../book/trees'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const HEADLESS = { cwd: ROOT, surface: null, isInteractive: false } as const
const STREAM = 'aaaaaaaaaaaaaaaa'
const SECRET = '1'.repeat(64)

const JOB_INPUT = {
  builder: 'main',
  tree: null,
  origin: 'composer',
  request: 'Fix the parser',
  repo: ROOT,
  base: 'main',
  type: 'fix',
  size: 'small',
  consequence: 'low',
  langs: [],
  prefs: { duration: null, budget: null, autonomy: null },
  limit: { nano: 10_000_000_000, source: 'default' },
  dependsOn: [],
  mergeAfter: [],
} as const

/** A session whose stream the binary already holds, with one open job whose opening waits to be sent. */
function bookWithWaitingJob(): RexBook {
  const setup = { session: SESSION, storage: 'on', jobsFromSdk: true, inert: null, claudeCode: '2.1.323', plugin: '0.8.0' } as const
  const started = acked(sessionIn(emptyBook(), setup, START_MS, { stream: STREAM, secret: SECRET }), 0)
  return openJobIn(started, JOB_INPUT, '0gaq820001', START_MS + 2000).book
}

function fieldOf(value: unknown, key: string): unknown {
  return typeof value === 'object' && value !== null ? Reflect.get(value, key) : undefined
}

type Sent = { readonly stream: string; readonly secret: string; readonly seqs: readonly number[] }

function sentOf(call: ProcessCall): Sent {
  const sent: unknown = JSON.parse(call.stdin ?? '{}')
  const events = fieldOf(sent, 'events')
  const seqs = Array.isArray(events) ? events.map((event: unknown) => Number(fieldOf(event, 'seq'))) : []
  return { stream: String(fieldOf(sent, 'stream')), secret: String(fieldOf(sent, 'secret')), seqs }
}

/** Answers the calls in order from a list; a call past its end fails the test. */
function inOrder(answers: readonly ((call: ProcessCall) => ProcessAnswer)[]): (call: ProcessCall) => ProcessAnswer {
  let index = 0
  return call => {
    const answer = answers[index]
    index += 1
    return answer === undefined ? { reject: `call ${String(index)} was not expected: ${call.argv.join(' ')}` } : answer(call)
  }
}

const stored = (call: ProcessCall): ProcessAnswer => ingestAnswer(Math.max(0, ...sentOf(call).seqs))
const exitOne = (): ProcessAnswer => ({ exitCode: 1, stdout: '', stderr: 'rex: database is locked\n' })

test('ingest_retries_then_resets_stream', async ($, on) => {
  const session = world(on, {
    config: CONFIG_ON,
    seed: bookWithWaitingJob(),
    processAnswer: inOrder([stored, () => ({ reject: 'timed out after 30000 ms' }), exitOne, exitOne, () => ingestAnswer(0), stored]),
  })
  on('tool.call', () => ({ result: {} }))

  await $.session.start(START)
  await $.tool.call({ tool: 'Bash', command: 'ls' })
  await session.clock.settle()
  expect(session.calls).toHaveLength(2)
  await session.clock.advance(12_000)
  expect(session.calls).toHaveLength(3)
  await session.clock.advance(22_000)
  expect(session.calls).toHaveLength(4)
  expect(session.book().jobs[0]?.degraded).toEqual(['ingest:1'])

  await session.clock.advance(42_000)

  const sent = session.calls.map(sentOf)
  expect(sent.slice(0, 4).map(one => one.stream)).toEqual([STREAM, STREAM, STREAM, STREAM])
  expect(sent[4]?.stream).not.toBe(STREAM)
  expect(sent[4]?.seqs).toEqual([])
  expect(sent[5]).toEqual({ stream: sent[4]?.stream, secret: sent[4]?.secret, seqs: [1, 2, 3] })
  expect(sent[5]?.secret).not.toBe(SECRET)
  expect(session.book().ingest?.queue).toEqual([])
  expect(session.book().jobs[0]?.degraded).toEqual(['ingest:1'])
})

test('the last flush at session end sends every queued event in order, the failed batch included, while a retry waits', async ($, on) => {
  const session = world(on, { config: CONFIG_ON, seed: bookWithWaitingJob(), processAnswer: inOrder([stored, exitOne, stored]) })
  on('tool.call', () => ({ result: {} }))
  on('agent.spawn', () => ({ model: 'claude-sonnet-5-5', agentId: 'a1' }))
  on('session.end', (_$, e) => ({ sessionId: e.sessionId }))

  await $.session.start(START)
  await $.tool.call({ tool: 'Bash', command: 'ls' })
  await session.clock.settle()
  expect(session.calls.map(sentOf).map(one => one.seqs)).toEqual([[], [1, 2]])
  const teammate = await $.agent.spawn(spawnInput({ description: 'worker', isTeammate: true }))
  expect(teammate.agentId).toBe('a1')
  await $.session.end({ reason: 'other', sessionId: SESSION, resume: { id: SESSION } })

  const queuedBeforeEnd = [1, 2, 3]
  expect(session.calls.map(sentOf).map(one => one.seqs)).toEqual([[], [1, 2], queuedBeforeEnd])
  expect(session.book().ingest?.queue).toEqual([])
  await session.clock.advance(120_000)
  expect(session.calls).toHaveLength(3)
})

test('ingest_reset_fails_twice_halts_ingest', async ($, on) => {
  const session = world(on, {
    config: CONFIG_ON,
    seed: bookWithWaitingJob(),
    processAnswer: inOrder([stored, exitOne, exitOne, exitOne, exitOne, exitOne]),
  })
  on('tool.call', () => ({ result: {} }))

  await $.session.start(START)
  await $.tool.call({ tool: 'Bash', command: 'ls' })
  await session.clock.settle()
  await session.clock.advance(12_000)
  await session.clock.advance(22_000)
  await session.clock.advance(42_000)
  await session.clock.advance(42_000)

  expect(session.calls).toHaveLength(6)
  expect(session.book().ingest?.halted).toBe('halted')
  expect(session.book().jobs[0]?.degraded).toEqual(['ingest:1', 'ingest:halted'])
  expect(session.book().ingest?.queue.map(event => event.kind)).toEqual(['session.start', 'job.open', 'job.degraded', 'job.degraded'])
  expect(session.statuses).toContain('rex: ingest stopped; run rex doctor')

  await $.tool.call({ tool: 'Bash', command: 'ls' })
  await session.clock.advance(120_000)
  expect(session.calls).toHaveLength(6)
})

test('ingest_conflict_halts_ingest', async ($, on) => {
  const session = world(on, {
    config: CONFIG_ON,
    seed: bookWithWaitingJob(),
    processAnswer: inOrder([stored, () => ({ exitCode: 2, stdout: '', stderr: 'ingest refused: conflict at seq 7\n' })]),
  })
  on('tool.call', () => ({ result: {} }))

  await $.session.start(START)
  await $.tool.call({ tool: 'Bash', command: 'ls' })
  await session.clock.settle()
  await session.clock.advance(120_000)

  expect(session.calls).toHaveLength(2)
  expect(session.book().ingest?.halted).toBe('conflict:7')
  expect(session.book().jobs[0]?.degraded).toEqual(['ingest:conflict:7'])
  expect(session.book().ingest?.queue.map(event => event.seq)).toEqual([1, 2, 3])
  expect(session.statuses).toContain('rex: ingest stopped: conflict at seq 7; run rex doctor')
})

test('ingest_without_store_halts_ingest', async ($, on) => {
  const stderr = 'ingest refused: no store at /r/.rex/rex.db; run rex init --with-storage\n'
  const session = world(on, { config: CONFIG_ON, seed: bookWithWaitingJob(), processAnswer: inOrder([() => ({ exitCode: 4, stdout: '', stderr })]) })
  on('tool.call', () => ({ result: {} }))

  await $.session.start(START)
  await $.tool.call({ tool: 'Bash', command: 'ls' })
  await session.clock.advance(120_000)

  expect(session.calls).toHaveLength(1)
  expect(session.book().ingest?.halted).toBe('no-store')
  expect(session.book().jobs[0]?.degraded).toEqual(['ingest:no-store'])
  expect(session.book().ingest?.queue.map(event => event.kind)).toEqual(['session.start', 'job.open', 'job.degraded'])
  expect(session.statuses).toEqual(['rex: ingest stopped: no store at /r/.rex/rex.db; run rex init --with-storage'])
})

test('ingest_wrong_secret_resets_stream', async ($, on) => {
  const refused = sharedJson('fixtures/shared/calls/ingest-wrong-secret.json')
  expect(refused).toMatchObject({ expect: { exit: 3 } })
  const stderr = String(fieldOf(fieldOf(refused, 'expect'), 'stderr'))
  const opened = acked(bookWithWaitingJob(), 3)
  const seed = limitIn(opened, '0gaq820001', { nano: 20_000_000_000, source: 'set' }, START_MS + 5000)
  const session = world(on, {
    config: CONFIG_ON,
    seed,
    processAnswer: inOrder([() => ingestAnswer(3), () => ({ exitCode: 3, stdout: '', stderr }), () => ingestAnswer(3), stored]),
  })

  await $.session.start(START)
  await session.clock.advance(10_000)

  const sent = session.calls.map(sentOf)
  expect(sent.map(one => one.seqs)).toEqual([[], [4], [], [4]])
  expect(sent[1]?.stream).toBe(STREAM)
  expect(sent[2]?.stream).not.toBe(STREAM)
  expect(sent.slice(2).every(one => one.secret !== SECRET)).toBe(true)
  expect(session.book().ingest?.queue).toEqual([])
  expect(session.book().ingest?.acked).toBe(4)
})

test('parked_question_blocks_job_at_session_end', async ($, on) => {
  const setup = { session: SESSION, storage: 'on', jobsFromSdk: true, inert: null, claudeCode: '2.1.323', plugin: '0.8.0' } as const
  const started = acked(sessionIn(emptyBook(), setup, START_MS, { stream: STREAM, secret: SECRET }), 0)
  const tree = treeIn(started, { id: 't1', charter: 'Fix the parser.', charterHash: 'sha256:0', policyHash: 'sha256:1', state: 'running', capNano: 0 }, START_MS)
  const job = openJobIn(tree, { ...JOB_INPUT, builder: 'b1', tree: 't1', origin: 'master' }, '0gaq820001', START_MS + 1000).book
  const asked = questionIn(
    job,
    { tree: 't1', job: '0gaq820001', question: 'Merge job 0gaq820001?', options: ['Merge', 'Keep the branch'], recommendation: 'Merge', category: 'merge', mergeFor: '0gaq820001' },
    START_MS + 2000,
  ).book
  const session = world(on, { config: CONFIG_ON, seed: acked(asked, 4), processAnswer: inOrder([() => ingestAnswer(4), stored]) })
  on('session.end', (_$, e) => ({ sessionId: e.sessionId }))

  await $.session.start(HEADLESS)
  await $.session.end({ reason: 'other', sessionId: SESSION, resume: { id: SESSION } })

  expect(session.book().jobs[0]?.status).toBe('blocked')
  expect(session.book().questions[0]?.answer).toBeNull()
  const last: unknown = JSON.parse(session.calls[1]?.stdin ?? '{}')
  expect(last).toMatchObject({ events: [{ seq: 5, kind: 'job.close', job: '0gaq820001', data: { status: 'blocked' } }] })
  expect(session.book().ingest?.queue).toEqual([])
})

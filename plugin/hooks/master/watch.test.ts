// Proves the wakes of a tree: a builder that waits for its own agents wakes nobody, one that stopped wakes its master with its last message, a stuck one is woken to go on, an API error is retried on the policy's schedule, and each runs out into a blocked job and a question.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine, MockClock } from 'claude-code/testing'

import { spawnInput } from '../book/kit'
import { AUTHOR } from '../pipeline/assembly.fixture'
import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, MASTER, masterWorld, POLICY_KEY, ran, settleLong, settleUntil, START, treeBook } from './master-kit'
import type { Rule } from './master-kit'

const MINUTE = 60_000

/** What `rex job show` answers when a builder closes its job: no cost of record. */
const JOB_SHOW: Rule = { starts: ['rex', 'job', 'show'], answer: ran('{}\n') }

async function treeWithJob($: Engine, on: On, keys: Partial<Policy> = {}) {
  const policy = await confirmedPolicy(keys)
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule([JOB_SHOW, ...buildRules('model-high-effort-high')]) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, agentId: MASTER, request: 'Fix the parser', consequence: 'low' })
  return { session, job: lineOf(built, 'job') ?? '', builder: lineOf(built, 'builder') ?? '' }
}

function apiError(agentId: string) {
  return { agentId, reason: 'error', answer: '', durationMs: 1, isAborted: false, turnId: `turn-${agentId}` } as const
}

/**
 * The count `counted` gives at each minute of `minutes` after `start`, with the
 * clock moved there and given time to settle up to `expected` of that minute.
 */
async function countsAt(clock: MockClock, start: number, steps: readonly (readonly [number, number])[], counted: () => number): Promise<number[]> {
  const seen: number[] = []
  for (const [minutes, expected] of steps) {
    await clock.set(start + minutes * MINUTE)
    await settleUntil(clock, () => counted() >= expected)
    await settleLong(clock)
    seen.push(counted())
  }
  return seen
}

// The default policy ticks every 5 minutes; the first tick, 5 minutes in, is the one that first sees the job's spend.
test('with the default policy, a working builder whose spend stays flat is woken 90 minutes after the first tick, again 90 later, and then its job is blocked', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  const start = session.clock.now()
  const wakes = (): number => session.sent.filter(sent => sent.to === builder).length

  const steps = [[90, 0], [95, 1], [180, 1], [185, 2], [270, 2]] as const
  const seen = await countsAt(session.clock, start, steps, wakes)
  const blockedAt270 = session.book().jobs[0]?.status
  await session.clock.set(start + 275 * MINUTE)
  await settleUntil(session.clock, () => session.book().jobs[0]?.status === 'blocked')

  expect(seen).toEqual(steps.map(([, expected]) => expected))
  expect(blockedAt270).toBe('created')
  expect(session.sent.filter(sent => sent.to === builder).map(sent => sent.text)).toEqual([`Continue job ${job} from the first step not done.`, `Continue job ${job} from the first step not done.`])
  expect(session.book().jobs[0]?.status).toBe('blocked')
  expect(session.book().questions.map(question => [question.job, question.answer])).toEqual([[job, null]])
})

test('a builder whose spend moves is not woken', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, builder } = await treeWithJob($, on, { watchdog_minutes: 30, stuck_ticks: 2 })

  for (let tick = 0; tick < 4; tick += 1) {
    const stream = $.turn.step({ turnId: `turn-${String(tick)}`, index: tick, model: 'claude-sonnet-5-5', effort: 'high', messageCount: 1, agentId: builder })
    for await (const chunk of stream) {
      expect(chunk).toBeDefined()
    }
    await stream.result
    await session.clock.advance(30 * MINUTE)
  }
  await settleLong(session.clock)

  expect(session.sent.filter(sent => sent.to === builder)).toEqual([])
})

test('api_error_retried_by_schedule', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { retry: { first_minutes: 2, first_tries: 3, then_minutes: 10, max_tries: 12 } })
  const wakes = (): number => session.sent.filter(sent => sent.to === builder).length

  const seen: number[] = []
  for (let error = 1; error <= 4; error += 1) {
    await $.turn.complete(apiError(builder))
    await $.turn.complete(apiError(builder))
    const wait = error <= 3 ? 2 : 10
    await session.clock.advance((wait - 1) * MINUTE)
    seen.push(wakes())
    await session.clock.advance(MINUTE)
    await settleUntil(session.clock, () => wakes() === error)
    seen.push(wakes())
  }

  expect(seen).toEqual([0, 1, 1, 2, 2, 3, 3, 4])
  expect(session.book().jobs[0]?.retries).toBe(4)
  expect(session.sent.at(-1)?.text).toBe(`Continue job ${job} from the first step not done.`)
})

test('api_error_limit_blocks_job', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { retry: { first_minutes: 2, first_tries: 1, then_minutes: 10, max_tries: 3 } })

  await $.turn.complete(apiError(builder))
  await session.clock.advance(2 * MINUTE)
  await $.turn.complete(apiError(builder))
  await session.clock.advance(10 * MINUTE)
  await $.turn.complete(apiError(builder))
  await session.clock.advance(60 * MINUTE)
  await settleUntil(session.clock, () => session.book().jobs[0]?.status === 'blocked' && session.statuses.some(line => line.includes('did not recover')))

  expect(session.sent.filter(sent => sent.to === builder)).toHaveLength(2)
  expect(session.book().jobs[0]?.status).toBe('blocked')
  expect(session.book().questions.map(question => question.job)).toEqual([job])
  expect(session.statuses).toContain(`rex: job ${job} blocked: builder ${builder} did not recover`)
})

function ends(agentId: string, answer: string) {
  return { agentId, reason: 'answer', answer, durationMs: 1, isAborted: false, turnId: `turn-${agentId}-${String(answer.length)}` } as const
}

/** Starts an agent under `parent` with the Agent tool, as a builder starts its decision agent or a lead starts a worker. */
async function startAgent($: Engine, parent: string, description: string): Promise<string> {
  const started = await $.agent.spawn(spawnInput({ description, parentAgentId: parent, prompt: AUTHOR, tool_use_id: `toolu_${description}` }))
  return started.agentId ?? ''
}

test('a builder whose turn ends while its decision agent runs wakes nobody', async ($, on) => {
  const { session, builder } = await treeWithJob($, on)
  await startAgent($, builder, 'decision')

  await $.turn.complete(ends(builder, 'Waiting for the decision agent to finish.'))
  await settleLong(session.clock)

  expect(session.sent).toEqual([])
})

test('a builder whose turn ends while a worker two levels down runs wakes nobody', async ($, on) => {
  const { session, builder } = await treeWithJob($, on)
  const lead = await startAgent($, builder, 'lead')
  await startAgent($, lead, 'worker')

  await $.turn.complete(ends(lead, 'Waiting for my worker.'))
  await $.turn.complete(ends(builder, 'Waiting for the lead.'))
  await settleLong(session.clock)

  expect(session.sent).toEqual([])
})

test('a builder that stops with no agent of its own running wakes its master with the end of its last message as data', async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  const decision = await startAgent($, builder, 'decision')
  await $.turn.complete(ends(decision, 'Decision written.'))
  const last = `${'x'.repeat(600)}State: awaiting decision. Ship both apps in one crate?${'y'.repeat(1_400)}`

  await $.turn.complete(ends(builder, last))
  await settleUntil(session.clock, () => session.sent.some(sent => sent.to === MASTER))

  const woken = session.sent.filter(sent => sent.to === MASTER).map(sent => sent.text)
  expect(woken).toHaveLength(1)
  expect(woken[0]?.startsWith(`rex: the builder of job ${job} ended its turn (answer) with no agent of its own running. Its last message, as data:\n<data>\n${last.slice(-1_500)}\n</data>\nThe tree now:\njob=none\n`)).toBe(true)
  expect(woken[0]).toContain(`job.${job}.status=created`)
})

test('a builder that closes its job blocked wakes its master with the status, though an agent of its own still runs', async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  await startAgent($, builder, 'lead')
  await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', agentId: builder, status: 'blocked', outcome: 'needs a person', deliveries: [] })

  await $.turn.complete(ends(builder, 'State: blocked. The spec is missing.'))
  await settleUntil(session.clock, () => session.sent.some(sent => sent.to === MASTER))

  const woken = session.sent.find(sent => sent.to === MASTER)?.text ?? ''
  expect(woken.startsWith(`rex: the builder of job ${job} closed it blocked and ended its turn (answer). Its last message, as data:\n<data>\nState: blocked. The spec is missing.\n</data>\nThe tree now:\n`)).toBe(true)
  expect(woken).toContain(`job.${job}.status=blocked`)
})

test('with the default policy, a builder that waits for an agent that spends is woken by nobody over 300 minutes', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  const decision = await startAgent($, builder, 'decision')
  await $.turn.complete(ends(builder, 'Waiting for the decision agent to finish.'))
  await $.turn.complete(ends(MASTER, 'Builder started.'))

  for (let tick = 0; tick < 60; tick += 1) {
    const stream = $.turn.step({ turnId: `turn-decision-${String(tick)}`, index: tick, model: 'claude-sonnet-5-5', effort: 'high', messageCount: 1, agentId: decision })
    for await (const chunk of stream) {
      expect(chunk).toBeDefined()
    }
    await stream.result
    await session.clock.advance(5 * MINUTE)
  }
  await settleLong(session.clock)

  expect(session.book().jobs.find(each => each.id === job)?.spentNano).toBeGreaterThan(0)
  expect(session.sent).toEqual([])
})

// The hole this closes: an agent whose end never reaches the book stays open for ever, and only its flat spend tells the watchdog.
test('with the default policy, an open agent with flat spend gets its stopped builder woken at minute 95, its stopped master told once, and the job blocked at 275', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  const start = session.clock.now()
  await startAgent($, builder, 'decision')
  await $.turn.complete(ends(builder, 'Waiting for the decision agent to finish.'))
  await $.turn.complete(ends(MASTER, 'Builder started.'))
  const wakes = (): number => session.sent.filter(sent => sent.to === builder).length

  const steps = [[90, 0], [95, 1], [180, 1], [185, 2], [270, 2]] as const
  const seen = await countsAt(session.clock, start, steps, wakes)
  const blockedAt270 = session.book().jobs[0]?.status
  await session.clock.set(start + 275 * MINUTE)
  await settleUntil(session.clock, () => session.book().jobs[0]?.status === 'blocked' && session.book().questions.length > 0)

  expect(seen).toEqual(steps.map(([, expected]) => expected))
  expect([blockedAt270, session.book().jobs[0]?.status]).toEqual(['created', 'blocked'])
  expect(session.book().questions.map(question => [question.job, question.answer])).toEqual([[job, null]])
  const toMaster = session.sent.filter(sent => sent.to === MASTER).map(sent => sent.text)
  expect(toMaster).toHaveLength(1)
  expect(toMaster[0]?.startsWith(`rex: the watchdog found job ${job} with an agent open and no spend for 90 minutes while you were stopped; the mod woke its builder. The tree now:\n`)).toBe(true)
})

test('with the default policy, an idle job wakes its builder 10 minutes after the first tick, its stopped master once, the builder again 10 later, and blocks 10 after that', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  const start = session.clock.now()
  await $.turn.complete(ends(builder, 'I will go on soon.'))
  await settleUntil(session.clock, () => session.sent.length === 1)
  await $.turn.complete(ends(MASTER, 'The builder is at work.'))
  const wakes = (): number => session.sent.filter(sent => sent.to === builder).length

  const steps = [[10, 0], [15, 1], [20, 1], [25, 2], [30, 2]] as const
  const seen = await countsAt(session.clock, start, steps, wakes)
  const blockedAt30 = session.book().jobs[0]?.status
  await session.clock.set(start + 35 * MINUTE)
  await settleUntil(session.clock, () => session.book().jobs[0]?.status === 'blocked' && session.book().questions.length > 0)

  expect(seen).toEqual(steps.map(([, expected]) => expected))
  expect([blockedAt30, session.book().jobs[0]?.status]).toEqual(['created', 'blocked'])
  expect(session.book().questions.map(question => [question.job, question.answer])).toEqual([[job, null]])
  const toMaster = session.sent.filter(sent => sent.to === MASTER).map(sent => sent.text)
  expect(toMaster).toHaveLength(2)
  expect(toMaster[1]?.startsWith(`rex: the watchdog found job ${job} with no agent running and no spend for 10 minutes while you were stopped; the mod woke its builder. The tree now:\n`)).toBe(true)
})

test('with the default policy, a builder stopped while its question waits for the person is not woken by the watchdog', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  await $.turn.complete(ends(builder, 'State: awaiting decision. Ship both apps?'))
  await settleUntil(session.clock, () => session.sent.length === 1)
  await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, job, question: 'Ship both apps?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })
  await $.turn.complete(ends(MASTER, 'Question parked.'))

  await session.clock.advance(100 * MINUTE)
  await settleLong(session.clock)

  expect(session.sent).toHaveLength(1)
})

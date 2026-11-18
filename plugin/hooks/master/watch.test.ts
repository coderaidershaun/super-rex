// Proves the wakes of a tree: a builder that waits for its own agents wakes nobody, one that stopped wakes its master, the main loop, with a prompt that carries its last message, a stuck one is woken to go on, an API error is retried on the policy's schedule, each runs out into a blocked job and a question whose answer builds the job again or leaves it, a job that moves again counts its wakes from nothing, and a wake no builder receives gets a new builder or a question.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine, MockClock } from 'claude-code/testing'

import { ROOT, spawnInput } from '../book/kit'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { AUTHOR } from '../pipeline/assembly.fixture'
import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, masterWorld, POLICY_KEY, ran, rex, settleLong, settleUntil, START, treeBook } from './master-kit'
import type { MasterOptions, Rule } from './master-kit'

const MINUTE = 60_000

/** What `rex job show` answers when a builder closes its job: no cost of record. */
const JOB_SHOW: Rule = { starts: ['rex', 'job', 'show'], answer: ran('{}\n') }

/** The process answers of a tree, where `git worktree list` names the worktree of each job in `jobs`, so a new builder of one starts in it. */
function treeAnswers(jobs: readonly string[]): (call: ProcessCall) => ProcessAnswer {
  const answer = byRule([JOB_SHOW, ...buildRules('model-high-effort-high')])
  return call => {
    if (call.argv.join(' ') !== `git -C ${ROOT} worktree list --porcelain`) {
      return answer(call)
    }
    return ran([`worktree ${ROOT}\nHEAD abc\n`, ...jobs.map(job => `worktree ${ROOT}.worktrees/${job}\nHEAD abc\nbranch refs/heads/rex/${job}\n`)].join('\n'))
  }
}

async function treeWithJob($: Engine, on: On, keys: Partial<Policy> = {}, options: MasterOptions = {}) {
  const policy = await confirmedPolicy(keys)
  const jobs: string[] = []
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: treeAnswers(jobs), ...options })
  await $.session.start(START)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })
  const job = lineOf(built, 'job') ?? ''
  jobs.push(job)
  return { session, job, builder: lineOf(built, 'builder') ?? '' }
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
test('with the default policy, a working builder whose spend stays flat is woken 90 minutes after the first tick, again 90 later, and then the person is asked while the job stays open', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  const start = session.clock.now()
  const wakes = (): number => session.sent.filter(sent => sent.to === builder).length

  const steps = [[90, 0], [95, 1], [180, 1], [185, 2], [270, 2]] as const
  const seen = await countsAt(session.clock, start, steps, wakes)
  const blockedAt270 = session.book().jobs[0]?.status
  await session.clock.set(start + 275 * MINUTE)
  await settleUntil(session.clock, () => session.book().questions.length > 0)

  expect(seen).toEqual(steps.map(([, expected]) => expected))
  expect(blockedAt270).toBe('created')
  expect(session.sent.filter(sent => sent.to === builder).map(sent => sent.text)).toEqual([`Continue job ${job} from the first step not done.`, `Continue job ${job} from the first step not done.`])
  expect(session.book().jobs[0]?.status).toBe('created')
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

test('api_error_limit_asks_the_person_and_keeps_the_job_open', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { retry: { first_minutes: 2, first_tries: 1, then_minutes: 10, max_tries: 3 } })

  await $.turn.complete(apiError(builder))
  await session.clock.advance(2 * MINUTE)
  await $.turn.complete(apiError(builder))
  await session.clock.advance(10 * MINUTE)
  await $.turn.complete(apiError(builder))
  await session.clock.advance(60 * MINUTE)
  await settleUntil(session.clock, () => session.book().questions.length > 0 && session.statuses.some(line => line.includes('did not recover')))
  // A further error end while the question waits asks nothing more.
  await $.turn.complete(apiError(builder))
  await settleLong(session.clock)

  expect(session.sent.filter(sent => sent.to === builder)).toHaveLength(2)
  expect(session.book().jobs[0]?.status).toBe('created')
  expect(session.book().questions.map(question => [question.job, question.options])).toEqual([[job, ['Build it again', 'Leave it blocked']]])
  expect(session.statuses).toContain(`rex: job ${job} waits on you: builder ${builder} did not recover`)
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
  expect(session.woken()).toEqual([])
})

test('a builder whose turn ends while a worker two levels down runs wakes nobody', async ($, on) => {
  const { session, builder } = await treeWithJob($, on)
  const lead = await startAgent($, builder, 'lead')
  await startAgent($, lead, 'worker')

  await $.turn.complete(ends(lead, 'Waiting for my worker.'))
  await $.turn.complete(ends(builder, 'Waiting for the lead.'))
  await settleLong(session.clock)

  expect(session.sent).toEqual([])
  expect(session.woken()).toEqual([])
})

test('a builder that stops with no agent of its own running wakes its master with the end of its last message as data', async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  const decision = await startAgent($, builder, 'decision')
  await $.turn.complete(ends(decision, 'Decision written.'))
  const last = `${'x'.repeat(600)}State: awaiting decision. Ship both apps in one crate?${'y'.repeat(1_400)}`

  await $.turn.complete(ends(builder, last))
  await settleUntil(session.clock, () => session.woken().length > 0)

  const woken = session.woken()
  expect(woken).toHaveLength(1)
  expect(woken[0]?.startsWith(`rex: the builder of job ${job} ended its turn (answer) with no agent of its own running. Its last message, as data:\n<data>\n${last.slice(-1_500)}\n</data>\nThe tree now:\njob=none\n`)).toBe(true)
  expect(woken[0]).toContain(`job.${job}.status=created`)
})

test('a builder that closes its job blocked wakes its master with the status, though an agent of its own still runs', async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  await startAgent($, builder, 'lead')
  await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', agentId: builder, status: 'blocked', outcome: 'needs a person', deliveries: [] })

  await $.turn.complete(ends(builder, 'State: blocked. The spec is missing.'))
  await settleUntil(session.clock, () => session.woken().length > 0)

  const woken = session.woken()[0] ?? ''
  expect(woken.startsWith(`rex: the builder of job ${job} closed it blocked and ended its turn (answer). Its last message, as data:\n<data>\nState: blocked. The spec is missing.\n</data>\nThe tree now:\n`)).toBe(true)
  expect(woken).toContain(`job.${job}.status=blocked`)
})

test('with the default policy, a builder that waits for an agent that spends is woken by nobody over 300 minutes', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  const decision = await startAgent($, builder, 'decision')
  await $.turn.complete(ends(builder, 'Waiting for the decision agent to finish.'))

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
  expect(session.woken()).toEqual([])
})

// The hole this closes: an agent whose end never reaches the book stays open for ever, and only its flat spend tells the watchdog.
test('with the default policy, an open agent with flat spend gets its stopped builder woken at minute 95, its master told once, and the person asked at 275 while the job stays open', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  const start = session.clock.now()
  await startAgent($, builder, 'decision')
  await $.turn.complete(ends(builder, 'Waiting for the decision agent to finish.'))
  const wakes = (): number => session.sent.filter(sent => sent.to === builder).length

  const steps = [[90, 0], [95, 1], [180, 1], [185, 2], [270, 2]] as const
  const seen = await countsAt(session.clock, start, steps, wakes)
  const blockedAt270 = session.book().jobs[0]?.status
  await session.clock.set(start + 275 * MINUTE)
  await settleUntil(session.clock, () => session.book().questions.length > 0)

  expect(seen).toEqual(steps.map(([, expected]) => expected))
  expect([blockedAt270, session.book().jobs[0]?.status]).toEqual(['created', 'created'])
  expect(session.book().questions.map(question => [question.job, question.answer])).toEqual([[job, null]])
  const toMaster = session.woken()
  expect(toMaster).toHaveLength(1)
  expect(toMaster[0]?.startsWith(`rex: the watchdog found job ${job} with an agent open and no spend for 90 minutes; the mod woke its builder. The tree now:\n`)).toBe(true)
})

test('with the default policy, an idle job wakes its builder 10 minutes after the first tick, its master once, the builder again 10 later, and asks the person 10 after that', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  const start = session.clock.now()
  await $.turn.complete(ends(builder, 'I will go on soon.'))
  await settleUntil(session.clock, () => session.woken().length === 1)
  const wakes = (): number => session.sent.filter(sent => sent.to === builder).length

  const steps = [[10, 0], [15, 1], [20, 1], [25, 2], [30, 2]] as const
  const seen = await countsAt(session.clock, start, steps, wakes)
  const blockedAt30 = session.book().jobs[0]?.status
  await session.clock.set(start + 35 * MINUTE)
  await settleUntil(session.clock, () => session.book().questions.length > 0)

  expect(seen).toEqual(steps.map(([, expected]) => expected))
  expect([blockedAt30, session.book().jobs[0]?.status]).toEqual(['created', 'created'])
  expect(session.book().questions.map(question => [question.job, question.answer])).toEqual([[job, null]])
  const toMaster = session.woken()
  expect(toMaster).toHaveLength(2)
  expect(toMaster[1]?.startsWith(`rex: the watchdog found job ${job} with no agent running and no spend for 10 minutes; the mod woke its builder. The tree now:\n`)).toBe(true)
})

test('with the default policy, a builder stopped while its question waits for the person is not woken by the watchdog', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)
  await $.turn.complete(ends(builder, 'State: awaiting decision. Ship both apps?'))
  await settleUntil(session.clock, () => session.woken().length === 1)
  await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', job, question: 'Ship both apps?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })

  await session.clock.advance(100 * MINUTE)
  await settleLong(session.clock)

  expect(session.woken()).toHaveLength(1)
  expect(session.sent).toEqual([])
})

/** One model request of `agentId`: $2 of spend on its job. */
async function spend($: Engine, agentId: string, turn: string): Promise<void> {
  const stream = $.turn.step({ turnId: turn, index: 0, model: 'claude-sonnet-5-5', effort: 'high', messageCount: 1, agentId })
  for await (const chunk of stream) {
    expect(chunk).toBeDefined()
  }
  await stream.result
}

/** An idle job of the default policy, run until the watchdog gave up on its builder and parked its question. */
async function blockedIdleJob($: Engine, on: On) {
  const made = await treeWithJob($, on)
  const start = made.session.clock.now()
  await $.turn.complete(ends(made.builder, 'I will go on soon.'))
  await made.session.clock.set(start + 35 * MINUTE)
  await settleUntil(made.session.clock, () => made.session.book().questions.length > 0)
  return { ...made, start, question: made.session.book().questions[0]?.id ?? '' }
}

test("the watchdog's blocked question answered Build it again starts a new builder, and no second question follows", { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, start, question } = await blockedIdleJob($, on)

  const reply = await $.command.run(rex(`answer ${question} Build it again`))
  await settleUntil(session.clock, () => session.book().trees[0]?.builders[0]?.agentId === 'b2')
  const restarted = session.book()
  await session.clock.set(start + 50 * MINUTE)
  await settleLong(session.clock)

  expect(reply.text).toBe(`question=${question}\nanswer=Build it again\n`)
  expect(session.spawned.map(spawn => spawn.description)).toEqual([`rex builder ${job}`, `rex builder ${job}`])
  expect(restarted.trees[0]?.builders).toEqual([{ job, agentId: 'b2' }])
  expect([restarted.jobs[0]?.status, restarted.jobs[0]?.stuckWakes]).toEqual(['created', 0])
  expect(session.book().questions.map(each => each.id)).toEqual([question])
})

test("the watchdog's blocked question answered Leave it blocked leaves the job blocked, and no further question comes", { timeoutMs: 60_000 }, async ($, on) => {
  const { session, start, question } = await blockedIdleJob($, on)

  const reply = await $.command.run(rex(`answer ${question} Leave it blocked`))
  await session.clock.set(start + 50 * MINUTE)
  await settleLong(session.clock)

  expect(reply.text).toBe(`question=${question}\nanswer=Leave it blocked\n`)
  expect(session.book().jobs[0]?.status).toBe('blocked')
  expect(session.book().questions.map(each => [each.id, each.answer?.option, each.answer?.source])).toEqual([[question, 'Leave it blocked', 'command']])
  expect(session.spawned).toHaveLength(1)
})

test('a stuck wake counts from nothing again once the job spent: the next wake after a quiet time is the first', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, builder } = await treeWithJob($, on)
  const start = session.clock.now()
  await $.turn.complete(ends(builder, 'I will go on soon.'))
  const wakes = (): number => session.sent.filter(sent => sent.to === builder).length

  await session.clock.set(start + 15 * MINUTE)
  await settleUntil(session.clock, () => wakes() === 1)
  const afterWake = session.book().jobs[0]?.stuckWakes
  await spend($, builder, 'turn-spend')
  await $.turn.complete(ends(builder, 'One more step done.'))
  await session.clock.set(start + 20 * MINUTE)
  await settleUntil(session.clock, () => session.book().jobs[0]?.stuckWakes === 0)
  const afterSpend = session.book().jobs[0]?.stuckWakes
  await session.clock.set(start + 30 * MINUTE)
  await settleUntil(session.clock, () => wakes() === 2)
  await settleLong(session.clock)

  expect([afterWake, afterSpend, session.book().jobs[0]?.stuckWakes]).toEqual([1, 0, 1])
  expect(session.book().jobs[0]?.status).toBe('created')
})

test('a turn that ends well after an API error counts the next error as the first again', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, builder } = await treeWithJob($, on)

  await $.turn.complete(apiError(builder))
  await settleUntil(session.clock, () => session.book().jobs[0]?.retries === 1)
  const afterError = session.book().jobs[0]?.retries
  await $.turn.complete(ends(builder, 'Back at work.'))
  await settleUntil(session.clock, () => session.book().jobs[0]?.retries === 0)

  expect([afterError, session.book().jobs[0]?.retries]).toEqual([1, 0])
})

test('a wake that reaches no builder parks one question under on_dead_builder ask, and the answer Build it again starts a new builder', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { on_dead_builder: 'ask' }, { deliver: () => ({ isDelivered: false, reason: 'no agent by that id' }) })

  await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job, message: 'Go on.' })
  await settleUntil(session.clock, () => session.book().questions.length > 0)
  await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job, message: 'Go on, again.' })
  await settleUntil(session.clock, () => session.sent.length === 2)
  await settleLong(session.clock)
  const asked = session.book().questions
  const reply = await $.command.run(rex(`answer ${asked[0]?.id ?? ''} Build it again`))
  await settleUntil(session.clock, () => session.book().trees[0]?.builders[0]?.agentId === 'b2')

  expect(asked.map(each => [each.job, each.question, each.options])).toEqual([
    [job, `Job ${job} is blocked: its builder ${builder} could not be woken (no agent by that id). What should the master do?`, ['Build it again', 'Leave it blocked']],
  ])
  expect(session.statuses).toContain(`rex: ${builder} was not woken: no agent by that id`)
  expect(reply.text).toBe(`question=${asked[0]?.id ?? ''}\nanswer=Build it again\n`)
  expect(session.book().trees[0]?.builders).toEqual([{ job, agentId: 'b2' }])
})

// The new builder starts from a timer, whose spawn passes none of the plugin's own spawn hooks. The kit's engine then answers the spawn with no agent id, so this test sees the spawn asked for, but not the run and the link that spawnOwn records from a live engine's answer.
test('a wake that reaches no builder asks for a new one under on_dead_builder continue, in the job worktree, and asks the person nothing', { timeoutMs: 20_000 }, async ($, on) => {
  const gone = { isDelivered: false, reason: 'no agent by that id' } as const
  const { session, job, builder } = await treeWithJob($, on, { on_dead_builder: 'continue' }, { deliver: e => (e.to === 'b1' ? gone : { isDelivered: true }) })

  await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job, message: 'Go on.' })
  await settleUntil(session.clock, () => session.spawned.length === 2)
  await settleLong(session.clock)

  expect(session.statuses).toContain(`rex: ${builder} was not woken: no agent by that id`)
  expect(session.spawned.map(spawn => [spawn.description, spawn.cwd])).toEqual([
    [`rex builder ${job}`, `${ROOT}.worktrees/${job}`],
    [`rex builder ${job}`, `${ROOT}.worktrees/${job}`],
  ])
  expect(session.spawned[1]?.prompt).toContain(`Continue job ${job} from the first step not done.`)
  expect(session.book().questions).toEqual([])
})

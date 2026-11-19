// Proves a master tree at its budget or its time: nothing new starts, the person is asked once, and each answer (raise, more time, stop) acts the same by the dialog and by /rex answer; the watchdog wakes no one while the tree waits and closes what a stopped tree leaves open; /rex budget and /rex time show and set the limits; rex_report ends a tree whose limit question waits.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import { spawnInput, START_MS } from '../book/kit'
import { AUTHOR } from '../pipeline/assembly.fixture'
import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, HEADLESS_START, KIND, lineOf, masterWorld, POLICY_KEY, ran, rex, sessionBook, settleLong, settleUntil, START, TREE, treeBook } from './master-kit'
import type { MasterOptions, MasterWorld, Rule, SeedLimits } from './master-kit'
import { isTreeLimitQuestion, whenText } from './tree-limits'

const DOLLAR = 1_000_000_000
const MINUTE = 60_000
const HOUR = 60 * MINUTE
const BUILD = 'mcp__super-rex__rex_build'
const JOB_SHOW: Rule = { starts: ['rex', 'job', 'show'], answer: ran('{}\n') }
const BUDGET_TEXT = 'Master tree t1 has spent its budget of $2. Raise it to $10?'
const DEADLINE = START_MS + HOUR
const TIME_TEXT = `Master tree t1 has run for its time of 1 hour, which ended at ${whenText(DEADLINE)}. Give it 1 hour more?`
const AT_BUDGET = "rex gate: master tree t1 has spent $2 of its budget of $2; nothing new starts until the person raises it (they answer the mod's question, or type /rex budget <usd>) or stops the tree; end your turn, and the mod wakes you"

type Setup = { readonly limits?: SeedLimits; readonly keys?: Partial<Policy>; readonly options?: MasterOptions; readonly headless?: boolean; readonly jobs?: number }

/** A tree of the budget and time given, with `jobs` jobs built (one when not given), each with its builder. */
async function treeWithJobs($: Engine, on: On, setup: Setup = {}) {
  const policy = await confirmedPolicy({ watchdog_minutes: 30, max_parallel_builders: 4, ...setup.keys })
  const session = masterWorld(on, { seed: await treeBook(policy, setup.limits), store: { [POLICY_KEY]: policy }, processAnswer: byRule([JOB_SHOW, ...buildRules('model-high-effort-high')]), ...setup.options })
  await $.session.start(setup.headless === true ? HEADLESS_START : START)
  const built: { readonly job: string; readonly builder: string }[] = []
  for (let index = 0; index < (setup.jobs ?? 1); index += 1) {
    const answer = await $.tool.call({ tool: BUILD, ...KIND, request: `Fix part ${String(index + 1)}`, consequence: 'low' })
    built.push({ job: lineOf(answer, 'job') ?? '', builder: lineOf(answer, 'builder') ?? '' })
  }
  const first = built[0] ?? { job: '', builder: '' }
  return { session, policy, built, job: first.job, builder: first.builder }
}

/** A tree whose budget is $2, with one job built, whose builder has spent the $2. */
async function atBudget($: Engine, on: On, setup: Setup = {}) {
  const made = await treeWithJobs($, on, { ...setup, limits: { capNano: 2 * DOLLAR, ...setup.limits } })
  await spend($, made.builder, 'turn-1')
  return made
}

/** One model request of an agent: $2 of spend on its job, by the shared prices table. */
async function spend($: Engine, agentId: string, turn: string): Promise<void> {
  const stream = $.turn.step({ turnId: turn, index: 0, model: 'claude-sonnet-5-5', effort: 'high', messageCount: 1, agentId })
  for await (const chunk of stream) {
    expect(chunk).toBeDefined()
  }
  await stream.result
}

/** The builder ends its turn with no agent of its own running: it waits, idle. */
async function idle($: Engine, agentId: string): Promise<void> {
  await $.turn.complete({ agentId, reason: 'answer', answer: 'Waiting.', durationMs: 1, isAborted: false, turnId: `turn-${agentId}-idle` })
}

function limitQuestions(session: MasterWorld) {
  return session.book().questions.filter(isTreeLimitQuestion)
}

/** Who a message was sent to, as the kit records it. */
function sentTo(to: unknown): unknown {
  return typeof to === 'object' && to !== null ? Reflect.get(to, 'agentId') : to
}

function toBuilder(session: MasterWorld, builder: string): readonly string[] {
  return session.sent.filter(sent => sentTo(sent.to) === builder).map(sent => sent.text)
}

function treeNotes(session: MasterWorld): readonly string[] {
  return (session.book().trees[0]?.notes ?? []).filter(note => note.job === null).map(note => note.text)
}

/** A dialog answer for the mod's limit question only; every other dialog is left to the hooks beneath. */
function limitDialog(answer: string | null): MasterOptions {
  return { dialog: question => (question.startsWith('Master tree ') ? answer : undefined) }
}

test('at its budget, rex_build is refused with what to do, one question is parked with its options, the commands are logged once, and the master is not woken by its own refused call', { timeoutMs: 20_000 }, async ($, on) => {
  const { session } = await atBudget($, on)

  const refused = await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleLong(session.clock)
  const again = await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleLong(session.clock)

  expect([refused.deny, again.deny]).toEqual([AT_BUDGET, AT_BUDGET])
  expect(session.book().jobs).toHaveLength(1)
  const asked = limitQuestions(session)
  expect(asked.map(question => [question.tree, question.job, question.category, question.question, question.options, question.recommendation])).toEqual([[TREE, null, 'spend', BUDGET_TEXT, ['Raise it', 'Stop the tree'], 'Stop the tree']])
  const id = asked[0]?.id ?? ''
  expect(session.logs.filter(line => line.includes(`/rex answer ${id} 1`))).toEqual([`  /rex answer ${id} 1      raise the budget to $10`])
  expect(session.logs[0]).toBe('rex: master tree t1 has spent $2 of its budget of $2. Nothing new starts. What you can type:')
  expect(session.logs).toHaveLength(8)
  expect(session.woken().filter(text => text.includes('asked the person to raise it'))).toEqual([])
  expect(session.statuses).toContain(`rex: question ${id} waits: ${BUDGET_TEXT} Answer with /rex answer ${id} <1-2>`)
})

test("at its budget, a builder's agent start is refused, the person is asked once, and the master is woken once with the question", { timeoutMs: 20_000 }, async ($, on) => {
  const { session, builder } = await atBudget($, on)

  const called = await $.tool.call({ tool: 'Agent', agentId: builder, description: 'engineer', prompt: AUTHOR })
  await settleLong(session.clock)
  const spawned = await $.agent.spawn(spawnInput({ description: 'engineer', parentAgentId: builder, prompt: AUTHOR }))
  await session.clock.advance(60 * MINUTE)
  await settleLong(session.clock)

  expect([called.deny, spawned.deny]).toEqual([AT_BUDGET, AT_BUDGET])
  expect(session.spawned.filter(each => each.description === 'engineer')).toEqual([])
  const asked = limitQuestions(session)
  expect(asked.map(question => question.question)).toEqual([BUDGET_TEXT])
  const woken = session.woken().filter(text => text.includes('asked the person to raise it'))
  expect(woken).toHaveLength(1)
  expect(woken[0]?.startsWith(`rex: the tree has spent $2 of its budget of $2, and the mod asked the person to raise it or stop the tree (question ${asked[0]?.id ?? ''}). Nothing new starts until they answer.`)).toBe(true)
  expect(woken[0]).toContain('rex_raise')
  expect(session.logs.filter(line => line.startsWith('rex: master tree t1'))).toHaveLength(1)
})

test('at its time, by the clock, the watchdog parks one question with its options, rex_build is refused, and the master is woken once', { timeoutMs: 20_000 }, async ($, on) => {
  const { session } = await treeWithJobs($, on, { limits: { timeMs: HOUR, deadlineAt: DEADLINE } })
  const before = await $.tool.call({ tool: BUILD, ...KIND, request: 'Second step', consequence: 'low' })

  await session.clock.set(DEADLINE)
  await settleUntil(session.clock, () => limitQuestions(session).length > 0)
  const refused = await $.tool.call({ tool: BUILD, ...KIND, request: 'Third step', consequence: 'low' })
  await session.clock.advance(60 * MINUTE)
  await settleLong(session.clock)

  expect(lineOf(before, 'status')).toBe('created')
  expect(refused.deny).toBe(`rex gate: master tree t1 has run for its time of 1 hour, which ended at ${whenText(DEADLINE)}; nothing new starts until the person raises it (they answer the mod's question, or type /rex time <span>) or stops the tree; end your turn, and the mod wakes you`)
  expect(limitQuestions(session).map(question => [question.question, question.options])).toEqual([[TIME_TEXT, ['Give it more time', 'Stop the tree']]])
  expect(session.woken().filter(text => text.includes('asked the person to raise it'))).toHaveLength(1)
  expect(session.logs.filter(line => line.includes('give the tree 1 hour more'))).toHaveLength(1)
})

test('headless, a tree at its budget stops with no question: its note, its builder told to close blocked, its master told to report', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await atBudget($, on, { headless: true, keys: { required_allow: ['Read'] }, options: { allow: ['Read'] } })

  const refused = await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleUntil(session.clock, () => toBuilder(session, builder).length > 0)
  await session.clock.advance(30 * MINUTE)
  await settleLong(session.clock)

  expect(refused.deny).toBe(AT_BUDGET)
  expect(session.book().questions).toEqual([])
  expect(session.dialogs).toEqual([])
  expect(treeNotes(session)).toEqual(['rex: this tree stopped at its budget of $2: no person is at the terminal to raise it'])
  expect(toBuilder(session, builder)).toEqual([`rex: master tree t1 stops at its budget of $2. Close your job ${job} with rex_job_close, status blocked, and say what is done and what is left.`])
  expect(session.woken().filter(text => text.startsWith('rex: the tree stops at its budget of $2: no person is at the terminal to raise it. Each builder was told to close its job blocked.'))).toHaveLength(1)
})

test('Raise it by /rex answer sets the next budget, notes the tree, wakes the idle builder and not the working one', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, built } = await treeWithJobs($, on, { limits: { capNano: 2 * DOLLAR }, jobs: 2 })
  const [first, second] = built
  // The second builder waits for its decision agent, which runs: it is at work, and no wake goes to it.
  await $.agent.spawn(spawnInput({ description: 'decision', parentAgentId: second?.builder ?? '', prompt: AUTHOR, tool_use_id: 'toolu_decision' }))
  await idle($, second?.builder ?? '')
  await spend($, first?.builder ?? '', 'turn-1')
  await idle($, first?.builder ?? '')
  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleLong(session.clock)
  const id = limitQuestions(session)[0]?.id ?? ''
  const sentBefore = session.sent.length

  const answered = await $.command.run(rex(`answer ${id} Raise it`))
  await settleUntil(session.clock, () => session.sent.length > sentBefore)
  await settleLong(session.clock)

  const what = `the person raised the budget of this tree from $2 to $10, by the answer Raise it to question ${id}`
  expect(answered.text).toBe(`question=${id}\nanswer=Raise it\n`)
  expect(session.book().trees[0]?.capNano).toBe(10 * DOLLAR)
  expect(treeNotes(session)).toEqual([`rex: ${what}`])
  expect(session.sent.slice(sentBefore).map(sent => [sentTo(sent.to), sent.text])).toEqual([[first?.builder, `rex: ${what}. Continue job ${first?.job ?? ''} from the first step not done.`]])
  expect((await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })).deny).toBeUndefined()
})

test('Raise it in the dialog sets the next budget and wakes the idle builder, as /rex answer does', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await atBudget($, on, { options: limitDialog('Raise it') })
  await idle($, builder)

  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleUntil(session.clock, () => session.book().trees[0]?.capNano === 10 * DOLLAR)
  await settleLong(session.clock)

  const question = limitQuestions(session)[0]
  expect(question?.answer).toMatchObject({ option: 'Raise it', source: 'dialog' })
  expect(treeNotes(session)).toEqual([`rex: the person raised the budget of this tree from $2 to $10, by the answer Raise it to question ${question?.id ?? ''}`])
  expect(toBuilder(session, builder).filter(text => text.endsWith(`Continue job ${job} from the first step not done.`) && text.includes('raised the budget'))).toHaveLength(1)
})

test('Give it more time by /rex answer gives the same span again, counted from the answer, and wakes the idle builder', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJobs($, on, { limits: { timeMs: HOUR, deadlineAt: DEADLINE } })
  await idle($, builder)
  await session.clock.set(DEADLINE)
  await settleUntil(session.clock, () => limitQuestions(session).length > 0)
  await session.clock.set(DEADLINE + 10 * MINUTE)
  const answeredAt = session.clock.now()
  const id = limitQuestions(session)[0]?.id ?? ''

  const answered = await $.command.run(rex(`answer ${id} 1`))
  await settleUntil(session.clock, () => toBuilder(session, builder).some(text => text.includes('more, until')))

  const what = `the person gave this tree 1 hour more, until ${whenText(answeredAt + HOUR)}, by the answer Give it more time to question ${id}`
  expect(answered.text).toBe(`question=${id}\nanswer=Give it more time\n`)
  expect([session.book().trees[0]?.timeMs, session.book().trees[0]?.deadlineAt]).toEqual([HOUR, answeredAt + HOUR])
  expect(treeNotes(session)).toEqual([`rex: ${what}`])
  expect(toBuilder(session, builder).filter(text => text.includes('more, until'))).toEqual([`rex: ${what}. Continue job ${job} from the first step not done.`])
})

test('Give it more time in the dialog gives the same span again, counted from the answer', { timeoutMs: 20_000 }, async ($, on) => {
  const { session } = await treeWithJobs($, on, { limits: { timeMs: HOUR, deadlineAt: DEADLINE }, options: limitDialog('Give it more time') })

  await session.clock.set(DEADLINE)
  await settleUntil(session.clock, () => session.book().trees[0]?.deadlineAt !== DEADLINE)

  expect(limitQuestions(session)[0]?.answer).toMatchObject({ option: 'Give it more time', source: 'dialog' })
  expect(session.book().trees[0]?.deadlineAt).toBe(session.clock.now() + HOUR)
})

test('Stop the tree by /rex answer notes the tree once, tells each builder once to close blocked, and tells the master to report', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await atBudget($, on)
  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleLong(session.clock)
  const id = limitQuestions(session)[0]?.id ?? ''

  const answered = await $.command.run(rex(`answer ${id} Stop the tree`))
  await settleUntil(session.clock, () => toBuilder(session, builder).length > 0)
  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await session.clock.advance(30 * MINUTE)
  await settleLong(session.clock)

  const why = `the person answered Stop the tree to question ${id}`
  expect(answered.text).toBe(`question=${id}\nanswer=Stop the tree\n`)
  expect(treeNotes(session)).toEqual([`rex: this tree stopped at its budget of $2: ${why}`])
  expect(toBuilder(session, builder)).toEqual([`rex: master tree t1 stops at its budget of $2. Close your job ${job} with rex_job_close, status blocked, and say what is done and what is left.`])
  const told = session.woken().filter(text => text.startsWith(`rex: the tree stops at its budget of $2: ${why}.`))
  expect(told).toHaveLength(1)
  expect(told[0]).toContain('end the tree with rex_report')
  expect(session.book().trees[0]?.capNano).toBe(2 * DOLLAR)
})

test('Stop the tree in the dialog stops the tree as /rex answer does', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, builder } = await atBudget($, on, { options: limitDialog('Stop the tree') })

  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleUntil(session.clock, () => toBuilder(session, builder).length > 0)

  expect(limitQuestions(session)[0]?.answer).toMatchObject({ option: 'Stop the tree', source: 'dialog' })
  expect(treeNotes(session)).toHaveLength(1)
  expect(treeNotes(session)[0]?.startsWith('rex: this tree stopped at its budget of $2: the person answered Stop the tree to question ')).toBe(true)
})

test('while the limit question waits, the watchdog wakes no builder and counts none stuck, for hours', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await atBudget($, on)
  await idle($, builder)
  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })

  for (let hour = 1; hour <= 6; hour += 1) {
    await session.clock.set(START_MS + hour * HOUR)
    await settleLong(session.clock)
  }

  expect(toBuilder(session, builder)).toEqual([])
  expect(session.book().questions.map(question => question.question)).toEqual([BUDGET_TEXT])
  expect(session.book().jobs.map(each => [each.id, each.status])).toEqual([[job, 'created']])
})

test('once the tree stopped, an idle job its builder did not close is closed blocked by the mod after two ticks, and the master is woken', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, job, builder } = await atBudget($, on)
  await idle($, builder)
  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleLong(session.clock)
  await $.command.run(rex(`answer ${limitQuestions(session)[0]?.id ?? ''} Stop the tree`))
  await settleLong(session.clock)
  const stoppedAt = session.clock.now()

  await session.clock.set(stoppedAt + 30 * MINUTE)
  await settleLong(session.clock)
  const afterOne = session.book().jobs[0]?.status
  await session.clock.set(stoppedAt + 90 * MINUTE)
  await settleUntil(session.clock, () => session.book().jobs[0]?.status === 'blocked')
  await settleLong(session.clock)

  expect(afterOne).toBe('created')
  expect(session.book().jobs.map(each => [each.id, each.status, each.outcome])).toEqual([[job, 'blocked', 'closed by the mod: the tree stopped at its budget of $2, and the builder did not close the job']])
  expect(session.woken().filter(text => text.startsWith(`rex: the mod closed job ${job} blocked: the tree stopped at its budget of $2, and its builder did not close it.`))).toHaveLength(1)
})

test('once the tree stopped, a job with an agent still running is not closed by the mod', { timeoutMs: 60_000 }, async ($, on) => {
  const { session, builder } = await treeWithJobs($, on, { limits: { capNano: 2 * DOLLAR } })
  const decision = await $.agent.spawn(spawnInput({ description: 'decision', parentAgentId: builder, prompt: AUTHOR, tool_use_id: 'toolu_decision' }))
  await spend($, builder, 'turn-1')
  await idle($, builder)
  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleLong(session.clock)
  await $.command.run(rex(`answer ${limitQuestions(session)[0]?.id ?? ''} Stop the tree`))

  for (let hour = 1; hour <= 4; hour += 1) {
    await session.clock.set(START_MS + hour * HOUR)
    await settleLong(session.clock)
  }

  expect(decision.agentId).toBeDefined()
  expect(session.book().jobs.map(each => each.status)).toEqual(['created'])
})

test('a wake that reaches no builder while the tree stands at its budget starts no builder and asks nothing more', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job } = await atBudget($, on, { keys: { on_dead_builder: 'continue' }, options: { deliver: () => ({ isDelivered: false, reason: 'no agent by that id' }) } })
  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })

  await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job, message: 'Go on.' })
  await settleUntil(session.clock, () => session.statuses.some(text => text.includes('was not woken')))
  await settleLong(session.clock)

  expect(session.spawned.filter(each => each.description === `rex builder ${job}`)).toHaveLength(1)
  expect(session.book().questions.map(question => question.question)).toEqual([BUDGET_TEXT])
})

test('a wake that reaches no builder while the tree stands at its time parks no builder question under on_dead_builder ask', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job } = await treeWithJobs($, on, { limits: { timeMs: HOUR, deadlineAt: DEADLINE }, keys: { on_dead_builder: 'ask' }, options: { deliver: () => ({ isDelivered: false, reason: 'no agent by that id' }) } })
  await session.clock.set(DEADLINE)
  await settleUntil(session.clock, () => limitQuestions(session).length > 0)

  await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job, message: 'Go on.' })
  await settleUntil(session.clock, () => session.statuses.some(text => text.includes('was not woken')))
  await settleLong(session.clock)

  expect(session.book().questions.map(question => question.question)).toEqual([TIME_TEXT])
  expect(session.spawned).toHaveLength(1)
})

test('/rex budget and /rex time with no value show the tree budget, its spend and its time', async ($, on) => {
  await treeWithJobs($, on, { limits: { capNano: 250 * DOLLAR, timeMs: HOUR, deadlineAt: DEADLINE } })

  const budget = await $.command.run(rex('budget'))
  const time = await $.command.run(rex('time'))

  const shown = `tree=t1\nbudget_usd=250.00\nspent_usd=0.00\ntime=1 hour\nends_at=${whenText(DEADLINE)}\n`
  expect([budget.text, time.text]).toEqual([shown, shown])
})

test('/rex budget sets the budget by tier name and by a typed amount, and notes the tree', async ($, on) => {
  const { session } = await treeWithJobs($, on)

  const high = await $.command.run(rex('budget high'))
  const typed = await $.command.run(rex('budget $1,500'))

  expect(lineOf({ result: high.text }, 'budget_usd')).toBe('5000.00')
  expect(lineOf({ result: typed.text }, 'budget_usd')).toBe('1500.00')
  expect(session.book().trees[0]?.capNano).toBe(1_500 * DOLLAR)
  expect(treeNotes(session)).toEqual(['rex: the person set the budget of this tree to $5,000, with /rex budget', 'rex: the person set the budget of this tree to $1,500, with /rex budget'])
})

test('/rex time sets the time from now by tier name and by a typed span, and unlimited takes the deadline off', async ($, on) => {
  const { session } = await treeWithJobs($, on)
  await session.clock.advance(7 * MINUTE)
  const now = session.clock.now()

  const short = await $.command.run(rex('time short'))
  const typed = await $.command.run(rex('time 90m'))
  const timed = [session.book().trees[0]?.timeMs, session.book().trees[0]?.deadlineAt]
  const off = await $.command.run(rex('time unlimited'))

  expect(lineOf({ result: short.text }, 'time')).toBe('1 hour')
  expect([lineOf({ result: typed.text }, 'time'), lineOf({ result: typed.text }, 'ends_at')]).toEqual(['90 minutes', whenText(now + 90 * MINUTE)])
  expect(timed).toEqual([90 * MINUTE, now + 90 * MINUTE])
  expect([lineOf({ result: off.text }, 'time'), lineOf({ result: off.text }, 'ends_at')]).toEqual(['unlimited', 'none'])
  expect([session.book().trees[0]?.timeMs, session.book().trees[0]?.deadlineAt]).toEqual([null, null])
  expect(treeNotes(session).at(-1)).toBe('rex: the person took the time limit off this tree, with /rex time')
})

test('/rex budget and /rex time refuse a value they cannot read, and change nothing', async ($, on) => {
  const { session } = await treeWithJobs($, on)

  const budget = await $.command.run(rex('budget lots'))
  const zero = await $.command.run(rex('budget 0'))
  const time = await $.command.run(rex('time soon'))
  const long = await $.command.run(rex('time 400d'))

  expect(budget.text).toBe('rex budget: lots is no budget, got lots; type one of low, medium, high, xhigh, or an amount in dollars such as 750')
  expect(zero.text).toBe('rex budget: 0 is no budget, got 0; type one of low, medium, high, xhigh, or an amount in dollars such as 750')
  expect(time.text).toBe('rex time: soon is no time, got soon; type one of short, medium, long, unlimited, or a span such as 90m, 36h or 2d')
  expect(long.text).toBe('rex time: 400d is no time, got 400d; type one of short, medium, long, unlimited, or a span such as 90m, 36h or 2d')
  expect([session.book().trees[0]?.capNano, session.book().trees[0]?.timeMs]).toEqual([100_000 * DOLLAR, null])
  expect(treeNotes(session)).toEqual([])
})

test('/rex budget and /rex time with no tree in the session say so', async ($, on) => {
  masterWorld(on, { seed: sessionBook(), store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const budget = await $.command.run(rex('budget 500'))
  const time = await $.command.run(rex('time'))

  expect(budget.text).toBe('rex budget: no master tree runs in this session, got none; the mod asks for a budget and a time when a tree opens with /super-rex:orchestration-master <goal>')
  expect(time.text).toBe('rex time: no master tree runs in this session, got none; the mod asks for a budget and a time when a tree opens with /super-rex:orchestration-master <goal>')
})

test("/rex budget and /rex time run only on the person's own typing", async ($, on) => {
  const { session } = await treeWithJobs($, on)
  const origins = [{ kind: 'sdk' }, { kind: 'task-notification' }, { kind: 'peer' }, { kind: 'plugin', name: 'other-plugin', asUser: true }, { kind: 'plugin', name: 'super-rex' }] as const

  const answers: (string | undefined)[] = []
  for (const origin of origins) {
    answers.push((await $.command.run(rex('budget 9000000', origin))).text)
    answers.push((await $.command.run(rex('time unlimited', origin))).text)
  }

  expect(answers).toEqual(origins.flatMap(origin => [`rex budget: only your own typing may run this, got origin ${origin.kind}`, `rex time: only your own typing may run this, got origin ${origin.kind}`]))
  expect([session.book().trees[0]?.capNano, treeNotes(session)]).toEqual([100_000 * DOLLAR, []])
})

test('/rex budget that lifts the tree off its budget closes the question Not needed, notes the tree, and wakes the master and the idle builder', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await atBudget($, on)
  await idle($, builder)
  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleLong(session.clock)
  const sentBefore = session.sent.length

  const set = await $.command.run(rex('budget 50'))
  await settleUntil(session.clock, () => session.sent.length > sentBefore)
  await settleLong(session.clock)

  const what = 'the person set the budget of this tree to $50, with /rex budget'
  expect(lineOf({ result: set.text }, 'budget_usd')).toBe('50.00')
  expect(session.book().questions.map(question => [question.question, question.answer?.option, question.answer?.source])).toEqual([[BUDGET_TEXT, 'Not needed', 'mod']])
  expect(treeNotes(session)).toEqual([`rex: ${what}`])
  expect(session.sent.slice(sentBefore).map(sent => sent.text)).toEqual([`rex: ${what}. Continue job ${job} from the first step not done.`])
  expect(session.woken().filter(text => text.startsWith(`rex: ${what}. The tree now:`))).toHaveLength(1)
})

test('/rex time that gives a tree past its end more time closes the question Not needed and counts from now', { timeoutMs: 20_000 }, async ($, on) => {
  const { session } = await treeWithJobs($, on, { limits: { timeMs: HOUR, deadlineAt: DEADLINE } })
  await session.clock.set(DEADLINE + 5 * MINUTE)
  await settleUntil(session.clock, () => limitQuestions(session).length > 0)
  const now = session.clock.now()

  await $.command.run(rex('time 2h'))
  await settleLong(session.clock)

  expect(session.book().trees[0]?.deadlineAt).toBe(now + 2 * HOUR)
  expect(limitQuestions(session).map(question => question.answer?.option)).toEqual(['Not needed'])
  expect(session.woken().filter(text => text.startsWith('rex: the person gave this tree 2 hours from now'))).toHaveLength(1)
})

test('rex_report refuses while a job is open at the budget, and once it closed, answers the waiting limit question Not needed and ends the tree', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, builder } = await atBudget($, on)
  await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })
  await settleLong(session.clock)

  const open = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Result: stopped at the budget.' })
  const waitingWhileOpen = limitQuestions(session).map(question => question.answer)
  await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', agentId: builder, status: 'blocked', outcome: 'the tree is at its budget', deliveries: [] })
  const reported = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Result: stopped at the budget.' })

  expect(open.deny).toMatch(/^rex_report: /)
  expect(waitingWhileOpen).toEqual([null])
  expect(reported.deny).toBeUndefined()
  expect(limitQuestions(session).map(question => [question.answer?.option, question.answer?.source])).toEqual([['Not needed', 'mod']])
  expect(session.book().trees[0]?.state).toBe('done')
})

test('/rex policy confirm on a paused tree at its budget keeps the budget it was given, so the tree still stands at it', { timeoutMs: 20_000 }, async ($, on) => {
  const { session } = await atBudget($, on)

  await $.command.run(rex('policy set tree_usd 500'))
  await $.command.run(rex('policy confirm'))
  const refused = await $.tool.call({ tool: BUILD, ...KIND, request: 'Another step', consequence: 'low' })

  expect([session.book().trees[0]?.state, session.book().trees[0]?.capNano]).toEqual(['running', 2 * DOLLAR])
  expect(refused.deny).toBe(AT_BUDGET)
})

test('at the highest budget an amount may be, no raise is offered: the tree stops, with one note however often the mod looks at it after', { timeoutMs: 20_000 }, async ($, on) => {
  const most = 9_000_000 * DOLLAR
  const policy = await confirmedPolicy({ watchdog_minutes: 30 })
  const seeded = await treeBook(policy, { capNano: most })
  const spent = { ...seeded, trees: seeded.trees.map(each => ({ ...each, ledgerNano: most })) }
  const session = masterWorld(on, { seed: spent, store: { [POLICY_KEY]: policy }, processAnswer: byRule([JOB_SHOW, ...buildRules('model-high-effort-high')]) })
  await $.session.start(START)

  for (let tick = 1; tick <= 3; tick += 1) {
    await $.tool.call({ tool: BUILD, ...KIND, request: 'x', consequence: 'low' })
    await session.clock.advance(30 * MINUTE)
    await settleLong(session.clock)
  }

  expect(limitQuestions(session)).toEqual([])
  expect(treeNotes(session)).toEqual(['rex: this tree stopped at its budget of $9,000,000: no higher budget can be set'])
})

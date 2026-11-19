// Proves rex_raise: only the master of a live tree asks, with a person at the terminal, for a value above the one now; one request of a kind waits at a time and a kept value is not asked again; only the person's answer sets the tree's budget, its time from the answer, or a job's limit; and a waiting request holds no tree open once no job is.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, HEADLESS_START, KIND, lineOf, masterWorld, POLICY_KEY, ran, resultText, rex, sessionBook, settleLong, settleUntil, START, TREE, treeBook } from './master-kit'
import type { MasterOptions, MasterWorld, Rule } from './master-kit'
import { whenText } from './tree-limits'

const DOLLAR = 1_000_000_000
const MINUTE = 60_000
const HOUR = 60 * MINUTE
const RAISE = 'mcp__super-rex__rex_raise'
const JOB_SHOW: Rule = { starts: ['rex', 'job', 'show'], answer: ran('{}\n') }
const REASON = 'Two more jobs are needed to finish the parser.'

type Setup = { readonly keys?: Partial<Policy>; readonly options?: MasterOptions; readonly headless?: boolean; readonly build?: boolean }

/** A tree with a budget of $250 and no time limit, and one job built unless `build` is false. */
async function tree($: Engine, on: On, setup: Setup = {}) {
  const policy = await confirmedPolicy({ watchdog_minutes: 30, ...setup.keys })
  const session = masterWorld(on, { seed: await treeBook(policy, { capNano: 250 * DOLLAR }), store: { [POLICY_KEY]: policy }, processAnswer: byRule([JOB_SHOW, ...buildRules('model-high-effort-high')]), ...setup.options })
  await $.session.start(setup.headless === true ? HEADLESS_START : START)
  if (setup.build === false) {
    return { session, job: '', builder: '' }
  }
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })
  return { session, job: lineOf(built, 'job') ?? '', builder: lineOf(built, 'builder') ?? '' }
}

function raise($: Engine, args: Readonly<Record<string, unknown>>, agentId?: string) {
  return $.tool.call({ tool: RAISE, reason: REASON, ...args, ...(agentId === undefined ? {} : { agentId }) })
}

function treeNotes(session: MasterWorld): readonly string[] {
  return (session.book().trees[0]?.notes ?? []).map(note => note.text)
}

test('only the master of a live tree may call rex_raise: an agent, a builder and a main loop with no tree are refused, and nothing is parked', async ($, on) => {
  const { session, builder } = await tree($, on)

  const agent = await raise($, { limit: 'budget', to: '500' }, 'a9')
  const fromBuilder = await raise($, { limit: 'budget', to: '500' }, builder)

  expect(agent.deny).toMatch(/^rex_raise: /)
  expect(fromBuilder.deny).toMatch(/^rex_raise: /)
  expect(session.book().questions).toEqual([])
})

test('the main loop of a session with no tree is refused rex_raise', async ($, on) => {
  const session = masterWorld(on, { seed: sessionBook(), store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const asked = await raise($, { limit: 'budget', to: '500' })

  expect(asked.deny).toMatch(/^rex_raise: only the master/)
  expect(session.book().questions).toEqual([])
})

test('rex_raise refuses a bad limit, a value it cannot read, an amount not above the one now, a closed job, a job of no tree of the caller, and no reason', async ($, on) => {
  const { session, job, builder } = await tree($, on, { keys: { per_job_usd: 10 } })

  const answers = [
    await raise($, { limit: 'tree', to: '500' }),
    await raise($, { limit: 'budget', to: 'lots' }),
    await raise($, { limit: 'time', to: 'soon' }),
    await raise($, { limit: 'budget', to: '250' }),
    await raise($, { limit: 'budget', to: 'low' }),
    await raise($, { limit: 'job', to: '5', job }),
    await raise($, { limit: 'job', to: '50', job: '0gb0000000' }),
    await raise($, { limit: 'job', to: '50' }),
    await $.tool.call({ tool: RAISE, limit: 'budget', to: '500' }),
  ]
  await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', agentId: builder, status: 'blocked', outcome: 'x', deliveries: [] })
  const closed = await raise($, { limit: 'job', to: '50', job })

  expect(answers.map(answer => answer.deny?.split(', got ')[0])).toEqual([
    'rex_raise: limit must be one of budget, time, job',
    'rex_raise: to must be an amount',
    'rex_raise: to must be a time',
    'rex_raise: to must be more than the budget now, $250',
    'rex_raise: to must be more than the budget now, $250',
    'rex_raise: to must be more than the limit now, $10',
    'rex_raise: job must be an open job of your tree',
    'rex_raise: job must be an open job of your tree',
    'rex_raise: reason must be text',
  ])
  expect(closed.deny?.split(', got ')[0]).toBe('rex_raise: job must be an open job of your tree')
  expect(session.book().questions).toEqual([])
})

test('headless, rex_raise is refused: no person is there to answer', async ($, on) => {
  const { session } = await tree($, on, { headless: true, build: false })

  const asked = await raise($, { limit: 'budget', to: '500' })

  expect(asked.deny).toBe('rex_raise: only the person raises a limit, got no person at the terminal; go on within the limits, or end the tree with rex_report')
  expect(session.book().questions).toEqual([])
})

test('rex_raise parks one spend question in the words of the request, and a second call of the same kind answers the waiting one', async ($, on) => {
  const { session } = await tree($, on)

  const first = await raise($, { limit: 'budget', to: '500' })
  const second = await raise($, { limit: 'budget', to: '900' })
  const time = await raise($, { limit: 'time', to: '8h' })

  const id = lineOf(first, 'question') ?? ''
  expect(resultText(first)).toBe(`question=${id}\nstate=parked\n`)
  expect(resultText(second)).toBe(`question=${id}\nstate=waiting\n`)
  expect(lineOf(time, 'state')).toBe('parked')
  expect(session.book().questions.map(question => [question.tree, question.job, question.category, question.question, question.options, question.recommendation])).toEqual([
    [TREE, null, 'spend', `The master of tree t1 asks you to raise its budget from $250 to $500. Its reason: ${REASON}`, ['Raise it to $500', 'Keep the budget'], 'Keep the budget'],
    [TREE, null, 'spend', `The master of tree t1 asks you to change its time from unlimited to 8 hours counted from your answer. Its reason: ${REASON}`, ['Give it 8 hours', 'Keep the time'], 'Keep the time'],
  ])
})

test('the answer that allows a budget request sets the budget and notes the tree; the keeping answer changes nothing, and the same value is not asked again', async ($, on) => {
  const { session } = await tree($, on)

  const kept = lineOf(await raise($, { limit: 'budget', to: '500' }), 'question') ?? ''
  const keep = await $.command.run(rex(`answer ${kept} Keep the budget`))
  const capAfterKeep = session.book().trees[0]?.capNano
  const again = await raise($, { limit: 'budget', to: '500' })
  const allowed = lineOf(await raise($, { limit: 'budget', to: '$1,000' }), 'question') ?? ''
  const allow = await $.command.run(rex(`answer ${allowed} 1`))

  expect(keep.text).toBe(`question=${kept}\nanswer=Keep the budget\n`)
  expect(capAfterKeep).toBe(250 * DOLLAR)
  expect(again.deny).toBe("rex_raise: the person already answered Keep the budget to this request, got 500; do not ask again: go on within the limits")
  expect(allow.text).toBe(`question=${allowed}\nanswer=Raise it to $1,000\n`)
  expect(session.book().trees[0]?.capNano).toBe(1_000 * DOLLAR)
  expect(treeNotes(session)).toEqual([`rex: the person raised the budget of this tree from $250 to $1,000, by the answer Raise it to $1,000 to the master's request, question ${allowed}`])
})

test('the answer that allows a time request gives the tree that time counted from the answer, and unlimited takes the deadline off', async ($, on) => {
  const { session } = await tree($, on)

  const id = lineOf(await raise($, { limit: 'time', to: '8h' }), 'question') ?? ''
  await session.clock.advance(20 * MINUTE)
  const answeredAt = session.clock.now()
  await $.command.run(rex(`answer ${id} Give it 8 hours`))
  const timed = [session.book().trees[0]?.timeMs, session.book().trees[0]?.deadlineAt]
  const forever = lineOf(await raise($, { limit: 'time', to: 'unlimited' }), 'question') ?? ''
  const options = session.book().questions.find(question => question.id === forever)?.options
  await $.command.run(rex(`answer ${forever} 1`))

  expect(timed).toEqual([8 * HOUR, answeredAt + 8 * HOUR])
  expect(options).toEqual(['Give it unlimited', 'Keep the time'])
  expect([session.book().trees[0]?.timeMs, session.book().trees[0]?.deadlineAt]).toEqual([null, null])
  expect(treeNotes(session)).toEqual([
    `rex: the person gave this tree 8 hours from now, until ${whenText(answeredAt + 8 * HOUR)}, by the answer Give it 8 hours to the master's request, question ${id}`,
    `rex: the person took the time limit off this tree, by the answer Give it unlimited to the master's request, question ${forever}`,
  ])
})

test("the answer that allows a job's request in the dialog sets the job's limit as the person's, and wakes the builder that stood at the old limit", { timeoutMs: 20_000 }, async ($, on) => {
  const dialog: MasterOptions = { dialog: question => (question.startsWith('The master of tree ') ? 'Raise it to $20' : undefined) }
  const { session, job, builder } = await tree($, on, { keys: { per_job_usd: 2, raise_limits: 'never' }, options: dialog })
  const stream = $.turn.step({ turnId: 'turn-1', index: 0, model: 'claude-sonnet-5-5', effort: 'high', messageCount: 1, agentId: builder })
  for await (const chunk of stream) {
    expect(chunk).toBeDefined()
  }
  await stream.result

  const asked = await raise($, { limit: 'job', to: '20', job })
  await settleUntil(session.clock, () => session.book().jobs[0]?.limit.nano === 20 * DOLLAR)
  await settleUntil(session.clock, () => session.sent.some(sent => sent.text.includes('is now $20.00')))

  const id = lineOf(asked, 'question') ?? ''
  const question = session.book().questions.find(each => each.id === id)
  expect([question?.job, question?.question, question?.options, question?.answer?.source]).toEqual([job, `The master of tree t1 asks you to raise the spend limit of job ${job} from $2 to $20. Its reason: ${REASON}`, ['Raise it to $20', 'Keep the limit'], 'dialog'])
  expect(session.book().jobs[0]?.limit).toEqual({ nano: 20 * DOLLAR, source: 'set' })
  expect(session.sent.filter(sent => sent.text.includes('is now $20.00')).map(sent => sent.text)).toEqual([`rex: the spend limit of job ${job} is now $20.00. Continue job ${job} from the first step not done.`])
})

test("the keeping answer to a job's request leaves its limit as it was", async ($, on) => {
  const { session, job } = await tree($, on, { keys: { per_job_usd: 10 } })

  const id = lineOf(await raise($, { limit: 'job', to: '20', job }), 'question') ?? ''
  await $.command.run(rex(`answer ${id} Keep the limit`))

  expect(session.book().jobs[0]?.limit).toEqual({ nano: 10 * DOLLAR, source: 'policy' })
  expect(session.book().questions.map(question => question.answer?.option)).toEqual(['Keep the limit'])
})

test("a waiting job request is not closed by the watchdog while its job is under its limit", { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job } = await tree($, on, { keys: { per_job_usd: 10 } })

  await raise($, { limit: 'job', to: '20', job })
  await session.clock.advance(90 * MINUTE)
  await settleLong(session.clock)

  expect(session.book().questions.map(question => question.answer)).toEqual([null])
})

test('rex_report answers a waiting request Not needed when no job is open, and ends the tree', async ($, on) => {
  const { session } = await tree($, on, { build: false })

  await raise($, { limit: 'budget', to: '500' })
  const reported = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Result: nothing more to build.' })

  expect(reported.deny).toBeUndefined()
  expect(session.book().questions.map(question => [question.answer?.option, question.answer?.source])).toEqual([['Not needed', 'mod']])
  expect([session.book().trees[0]?.state, session.book().trees[0]?.capNano]).toEqual(['done', 250 * DOLLAR])
})

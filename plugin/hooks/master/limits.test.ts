// Proves raise_limits of the policy: a tree job at its spend limit gets it doubled once by policy, by the person's answer to a spend question, or never, each up to the tree's cap.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, lineOf, MASTER, masterWorld, POLICY_KEY, rex, settleLong, settleUntil, START, treeBook } from './master-kit'

const MINUTE = 60_000
const PANE = {
  plugin: 'super-rex',
  component: 'Pane',
  requestId: 'rex',
  surface: 'terminal',
  props: { title: 'SuperRex', isFocused: true, bodyColumns: 120, placement: 'dock', scroll: { offset: 0, bodyRows: 40 }, view: {} },
} as const
const DOLLAR = 1_000_000_000

async function treeWithJob($: Engine, on: On, keys: Partial<Policy>) {
  const policy = await confirmedPolicy({ watchdog_minutes: 30, ...keys })
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', agentId: MASTER, request: 'Fix the parser', consequence: 'low' })
  return { session, job: lineOf(built, 'job') ?? '', builder: lineOf(built, 'builder') ?? '' }
}

/** One model request of the builder: $2 of spend on its job, by the shared prices table. */
async function spend($: Engine, builder: string, turn: string): Promise<void> {
  const stream = $.turn.step({ turnId: turn, index: 0, model: 'claude-sonnet-5-5', effort: 'high', messageCount: 1, agentId: builder })
  for await (const chunk of stream) {
    expect(chunk).toBeDefined()
  }
  await stream.result
}

test('raise_limits allowed doubles the limit of a job at its limit once, up to the tree cap, and wakes its builder', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { raise_limits: 'allowed', per_job_usd: 2, tree_usd: 3 })

  await spend($, builder, 'turn-1')
  await session.clock.advance(30 * MINUTE)
  await settleUntil(session.clock, () => session.sent.length > 0)
  const raised = session.book().jobs[0]?.limit
  await spend($, builder, 'turn-2')
  await session.clock.advance(30 * MINUTE)
  await settleLong(session.clock)

  expect(raised).toEqual({ nano: 3 * DOLLAR, source: 'policy' })
  expect(session.book().jobs[0]?.limit).toEqual({ nano: 3 * DOLLAR, source: 'policy' })
  expect(session.book().trees[0]?.notes.map(note => [note.job, note.text])).toEqual([[job, 'rex: raised the spend limit of this job from $2.00 to $3.00 by policy raise_limits allowed']])
  expect(session.sent.filter(sent => sent.to === builder).map(sent => sent.text)).toEqual([`rex: the spend limit of job ${job} is now $3.00. Continue job ${job} from the first step not done.`])
  expect(session.book().questions).toEqual([])
})

test('raise_limits ask parks one spend question, and the answer Raise doubles the limit', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { raise_limits: 'ask', per_job_usd: 2 })

  await spend($, builder, 'turn-1')
  await session.clock.advance(60 * MINUTE)
  await settleLong(session.clock)
  const asked = session.book().questions
  const id = asked[0]?.id ?? ''
  const answered = await $.command.run(rex(`answer ${id} Raise`))
  await settleUntil(session.clock, () => session.sent.some(sent => sent.to === builder))

  expect(asked.map(question => [question.job, question.category, question.question, question.options, question.recommendation])).toEqual([
    [job, 'spend', `Job ${job} reached its spend limit of $2.00. Raise it to $4.00?`, ['Raise', 'Keep the limit'], 'Keep the limit'],
  ])
  expect(answered.text).toBe(`question=${id}\nanswer=Raise\n`)
  expect(session.book().jobs[0]?.limit).toEqual({ nano: 4 * DOLLAR, source: 'set' })
  expect(session.sent.filter(sent => sent.to === builder).map(sent => sent.text)).toEqual([`rex: the spend limit of job ${job} is now $4.00. Continue job ${job} from the first step not done.`])
})

test('raise_limits never leaves the limit, and Keep the limit asks no second time', { timeoutMs: 20_000 }, async ($, on) => {
  const never = await treeWithJob($, on, { raise_limits: 'never', per_job_usd: 2 })

  await spend($, never.builder, 'turn-1')
  await never.session.clock.advance(60 * MINUTE)
  await settleLong(never.session.clock)

  expect(never.session.book().jobs[0]?.limit).toEqual({ nano: 2 * DOLLAR, source: 'policy' })
  expect(never.session.book().questions).toEqual([])
  expect(never.session.sent).toEqual([])
})

test('a spend question answered Keep the limit is not asked again at the same limit', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, builder } = await treeWithJob($, on, { raise_limits: 'ask', per_job_usd: 2 })

  await spend($, builder, 'turn-1')
  await session.clock.advance(30 * MINUTE)
  await settleUntil(session.clock, () => session.book().questions.length > 0)
  const id = session.book().questions[0]?.id ?? ''
  const kept = await $.command.run(rex(`answer ${id} Keep the limit`))
  await session.clock.advance(60 * MINUTE)
  await settleLong(session.clock)

  expect(kept.text).toBe(`question=${id}\nanswer=Keep the limit\n`)
  expect(session.book().questions.map(question => question.answer?.option ?? null)).toEqual(['Keep the limit'])
  expect(session.book().jobs[0]?.limit).toEqual({ nano: 2 * DOLLAR, source: 'policy' })
})

test('a spend question answered Raise in the pane raises the limit at the next tick', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, builder } = await treeWithJob($, on, { raise_limits: 'ask', per_job_usd: 2 })

  await spend($, builder, 'turn-1')
  await session.clock.advance(30 * MINUTE)
  await settleUntil(session.clock, () => session.book().questions.length > 0)
  const ui = await $.ui.mount(PANE)
  await ui.press({ key: 'view-questions' })
  await ui.press({ key: `answer-${session.book().questions[0]?.id ?? ''}-0` })
  await ui.unmount()
  await session.clock.advance(30 * MINUTE)
  await settleUntil(session.clock, () => session.sent.some(sent => sent.to === builder))

  expect(session.book().questions[0]?.answer).toMatchObject({ option: 'Raise', source: 'pane' })
  expect(session.book().jobs[0]?.limit).toEqual({ nano: 4 * DOLLAR, source: 'set' })
})

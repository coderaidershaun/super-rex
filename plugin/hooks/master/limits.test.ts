// Proves the spend limit of a tree job: raise_limits of the policy doubles it once, by policy or by the person's answer to a spend question, up to the tree's cap; a limit that stays has the builder told once to close blocked; a limit the person sets with /rex limit or at a shell settles the spend question and wakes the builder; and a confirmed tree_usd is the tree's cap.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, masterWorld, POLICY_KEY, ran, rex, settleLong, settleUntil, START, treeBook } from './master-kit'
import type { Rule } from './master-kit'

const MINUTE = 60_000
const PANE = {
  plugin: 'super-rex',
  component: 'Pane',
  requestId: 'rex',
  surface: 'terminal',
  props: { title: 'SuperRex', isFocused: true, bodyColumns: 120, placement: 'dock', scroll: { offset: 0, bodyRows: 40 }, view: {} },
} as const
const DOLLAR = 1_000_000_000

/** What `rex limit <job> --json` answers from the store: the limit and who set it. */
function storeLimit(usd: number, source: 'default' | 'policy' | 'set'): Rule {
  return { starts: ['rex', 'limit'], answer: ran(`${JSON.stringify({ job: 'any', spent_usd: 2, limit_usd: usd, source })}\n`) }
}

/** A tree with one built job, whose store holds a limit set by policy; a test puts a rule first in `rules` to change what the store holds. */
async function treeWithJob($: Engine, on: On, keys: Partial<Policy>) {
  const policy = await confirmedPolicy({ watchdog_minutes: 30, ...keys })
  const rules: Rule[] = [storeLimit(2, 'policy'), ...buildRules('model-high-effort-high')]
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(rules) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })
  return { session, rules, job: lineOf(built, 'job') ?? '', builder: lineOf(built, 'builder') ?? '' }
}

/** The messages about the spend limit that reached the builder. */
function limitWakes(sent: readonly { readonly to: unknown; readonly text: string }[], builder: string): readonly string[] {
  return sent.filter(each => each.to === builder && each.text.includes('spend limit')).map(each => each.text)
}

/** One model request of the builder: $2 of spend on its job, by the shared prices table. */
async function spend($: Engine, builder: string, turn: string): Promise<void> {
  const stream = $.turn.step({ turnId: turn, index: 0, model: 'claude-sonnet-5-5', effort: 'high', messageCount: 1, agentId: builder })
  for await (const chunk of stream) {
    expect(chunk).toBeDefined()
  }
  await stream.result
}

test('raise_limits allowed doubles the limit of a job at its limit once, up to the tree cap, and wakes its builder; at the cap the builder is told once to close blocked', { timeoutMs: 20_000 }, async ($, on) => {
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
  const kept = 'the cap of the tree, $3.00, leaves no room to raise it'
  expect(session.book().trees[0]?.notes.map(note => [note.job, note.text])).toEqual([
    [job, 'rex: raised the spend limit of this job from $2.00 to $3.00 by policy raise_limits allowed'],
    [job, `rex: kept the spend limit of this job at $3.00: ${kept}`],
  ])
  expect(limitWakes(session.sent, builder)).toEqual([
    `rex: the spend limit of job ${job} is now $3.00. Continue job ${job} from the first step not done.`,
    `rex: the spend limit of job ${job} stays at $3.00: ${kept}. Close your job with rex_job_close, status blocked.`,
  ])
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

test('raise_limits never leaves the limit, asks nothing, and tells the builder once to close its job blocked', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { raise_limits: 'never', per_job_usd: 2 })

  await spend($, builder, 'turn-1')
  await session.clock.advance(30 * MINUTE)
  await settleUntil(session.clock, () => limitWakes(session.sent, builder).length > 0)
  await session.clock.advance(60 * MINUTE)
  await settleLong(session.clock)

  expect(session.book().jobs[0]?.limit).toEqual({ nano: 2 * DOLLAR, source: 'policy' })
  expect(session.book().questions).toEqual([])
  expect(limitWakes(session.sent, builder)).toEqual([`rex: the spend limit of job ${job} stays at $2.00: raise_limits of the policy is never. Close your job with rex_job_close, status blocked.`])
  expect(session.statuses).toContain(`rex: job ${job} stops at its spend limit of $2.00: raise_limits of the policy is never; raise it with /rex limit ${job} <usd>`)
})

test('a spend question answered Keep the limit is not asked again at the same limit, and its builder is told once to close blocked', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { raise_limits: 'ask', per_job_usd: 2 })

  await spend($, builder, 'turn-1')
  await session.clock.advance(30 * MINUTE)
  await settleUntil(session.clock, () => session.book().questions.length > 0)
  const id = session.book().questions[0]?.id ?? ''
  const kept = await $.command.run(rex(`answer ${id} Keep the limit`))
  await settleUntil(session.clock, () => limitWakes(session.sent, builder).length > 0)
  const told = [...limitWakes(session.sent, builder)]
  await session.clock.advance(60 * MINUTE)
  await settleLong(session.clock)

  const stays = `rex: the spend limit of job ${job} stays at $2.00: the person answered Keep the limit to question ${id}. Close your job with rex_job_close, status blocked.`
  expect(kept.text).toBe(`question=${id}\nanswer=Keep the limit\n`)
  expect(told).toEqual([stays])
  expect(limitWakes(session.sent, builder)).toEqual([stays])
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

test('/rex limit on a tree job at its limit settles its spend question and wakes its builder with the new limit', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { raise_limits: 'ask', per_job_usd: 2 })

  await spend($, builder, 'turn-1')
  await session.clock.advance(30 * MINUTE)
  await settleUntil(session.clock, () => session.book().questions.length > 0)
  const set = await $.command.run(rex(`limit ${job} 10`))
  await settleUntil(session.clock, () => limitWakes(session.sent, builder).length > 0)

  expect(set.text).toBe(`job=${job}\nspent_usd=2.00\nlimit_usd=10.00\nlimit_source=set\n`)
  expect(session.book().questions.map(question => [question.category, question.answer?.option, question.answer?.source])).toEqual([['spend', 'Not needed', 'mod']])
  expect(limitWakes(session.sent, builder)).toEqual([`rex: the spend limit of job ${job} is now $10.00. Continue job ${job} from the first step not done.`])
  expect(session.book().trees[0]?.notes.map(note => [note.job, note.text])).toEqual([[job, 'rex: raised the spend limit of this job from $2.00 to $10.00 by the person, with /rex limit']])
})

test('/rex limit on a tree job under its limit sets it and wakes nobody', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { raise_limits: 'ask', per_job_usd: 2 })

  const set = await $.command.run(rex(`limit ${job} 50`))
  await settleLong(session.clock)

  expect(set.text).toBe(`job=${job}\nspent_usd=0.00\nlimit_usd=50.00\nlimit_source=set\n`)
  expect(session.sent.filter(each => each.to === builder)).toEqual([])
  expect(session.book().trees[0]?.notes).toEqual([])
})

test('the watchdog takes a higher limit the person set at a shell: the limit is set, the spend question settled, the builder woken', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, rules, job, builder } = await treeWithJob($, on, { raise_limits: 'ask', per_job_usd: 2 })

  await spend($, builder, 'turn-1')
  await session.clock.advance(30 * MINUTE)
  await settleUntil(session.clock, () => session.book().questions.length > 0)
  rules.unshift(storeLimit(10, 'set'))
  await session.clock.advance(30 * MINUTE)
  await settleUntil(session.clock, () => limitWakes(session.sent, builder).length > 0)

  expect(session.book().jobs[0]?.limit).toEqual({ nano: 10 * DOLLAR, source: 'set' })
  expect(session.book().questions.map(question => [question.answer?.option, question.answer?.source])).toEqual([['Not needed', 'mod']])
  expect(limitWakes(session.sent, builder)).toEqual([`rex: the spend limit of job ${job} is now $10.00. Continue job ${job} from the first step not done.`])
  expect(session.book().trees[0]?.notes.map(note => note.text)).toEqual(['rex: raised the spend limit of this job from $2.00 to $10.00 by the person, with rex limit at a shell'])
})

test('/rex policy set tree_usd, then confirm, gives the paused tree the new cap, which rex_status shows', { timeoutMs: 20_000 }, async ($, on) => {
  const { session } = await treeWithJob($, on, {})

  const before = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })
  const set = await $.command.run(rex('policy set tree_usd 500'))
  const confirmed = await $.command.run(rex('policy confirm'))
  const after = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })

  expect(lineOf(before, 'tree_cap_usd')).toBe('100000.00')
  expect(set.text).toContain('paused=t1')
  expect(confirmed.text).toContain('resumed=t1')
  expect(lineOf(after, 'tree_cap_usd')).toBe('500.00')
  expect([session.book().trees[0]?.state, session.book().trees[0]?.capNano]).toEqual(['running', 500 * DOLLAR])
})

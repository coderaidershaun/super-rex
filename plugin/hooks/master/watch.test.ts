// Proves the watchdog and the retry schedule: a builder whose spend stays flat is woken to go on, an API error is retried on the policy's schedule, and each runs out into a blocked job and a question.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, lineOf, MASTER, masterWorld, POLICY_KEY, settleLong, settleUntil, START, treeBook } from './master-kit'

const MINUTE = 60_000

async function treeWithJob($: Engine, on: On, keys: Partial<Policy> = {}) {
  const policy = await confirmedPolicy(keys)
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', agentId: MASTER, request: 'Fix the parser', consequence: 'low' })
  return { session, job: lineOf(built, 'job') ?? '', builder: lineOf(built, 'builder') ?? '' }
}

function apiError(agentId: string) {
  return { agentId, reason: 'error', answer: '', durationMs: 1, isAborted: false, turnId: `turn-${agentId}` } as const
}

test('flat_spend_for_n_ticks_resumes_builder', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { watchdog_minutes: 30, stuck_ticks: 3 })

  await session.clock.advance(60 * MINUTE)
  const beforeThird = session.sent.length
  await session.clock.advance(30 * MINUTE)
  await settleUntil(session.clock, () => session.sent.length > 0 && session.book().jobs[0]?.stuckWakes === 1)

  expect(beforeThird).toBe(0)
  expect(session.sent.map(sent => [sent.to, sent.text])).toEqual([[builder, `Continue job ${job} from the first step not done.`]])
  expect(session.book().jobs[0]?.stuckWakes).toBe(1)
})

test('stuck_twice_blocks_job', { timeoutMs: 20_000 }, async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on, { watchdog_minutes: 30, stuck_ticks: 3 })

  await session.clock.advance(270 * MINUTE)
  await settleUntil(session.clock, () => session.book().jobs[0]?.status === 'blocked' && session.book().questions.length > 0)

  expect(session.sent.filter(sent => sent.to === builder).map(sent => sent.text)).toEqual([`Continue job ${job} from the first step not done.`, `Continue job ${job} from the first step not done.`])
  expect(session.book().jobs[0]?.status).toBe('blocked')
  expect(session.book().questions.map(question => [question.job, question.answer])).toEqual([[job, null]])
})

test('a builder whose spend moves is not woken', async ($, on) => {
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

test('a builder that ends its turn wakes its master with the tree status', async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)

  await $.turn.complete({ agentId: builder, reason: 'answer', answer: 'awaiting decision: q1', durationMs: 1, isAborted: false, turnId: 'turn-b' })
  await settleUntil(session.clock, () => session.sent.some(sent => sent.to === MASTER))

  const woken = session.sent.find(sent => sent.to === MASTER)
  expect(woken?.text).toContain(`rex: the builder of job ${job} ended its turn (answer)`)
  expect(woken?.text).toContain(`job.${job}.status=created`)
})

// Proves that the main loop's own steps count toward the open job it builds, so the spend check sees the whole session's work on the job.
import { expect, test } from 'claude-code/testing'
import type { On, TurnStepResult, TurnUsage } from 'claude-code'

import { ROOT, world } from '../book/kit'
import { priceNano } from '../book/prices'
import { files, JOB, lineOf, seededBook } from '../tools/act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const USAGE: TurnUsage = { model: 'claude-sonnet-5-5', input_tokens: 2_000, output_tokens: 400_000, cache_read_input_tokens: 30_000, cache_creation_input_tokens: 5_000 }
const STEP_NANO = priceNano(USAGE)

/** The engine beneath the plugin: each model request answers with the same usage, and each other tool call runs. */
function engine(on: On): void {
  on('turn.step', async function* (_$, e) {
    const result: TurnStepResult = { turnId: e.turnId, index: e.index, answer: '', toolUses: [], stopReason: 'end_turn', usage: USAGE }
    return result
  })
  on('tool.call', (_$, e) => ({ result: `ran ${e.tool}` }))
}

function usd(nano: number): string {
  return (nano / 1_000_000_000).toFixed(2)
}

test("the main loop's own steps raise the spend of the job it builds, and the spend check then refuses a new agent", async ($, on) => {
  files(on, {})
  engine(on)
  const session = world(on, { seed: seededBook({ limitNano: STEP_NANO + 1 }) })
  await $.session.start(START)

  const mainStep = async (index: number): Promise<void> => {
    const stream = $.turn.step({ turnId: 't-main', index, model: USAGE.model, effort: 'medium', messageCount: 1 })
    for await (const chunk of stream) {
      expect(chunk).toBeDefined()
    }
    await stream.result
  }

  await mainStep(1)
  const underLimit = await $.tool.call({ tool: 'Agent', description: 'worker', prompt: 'Task: T2' })
  await mainStep(2)
  const atLimit = await $.tool.call({ tool: 'Agent', description: 'worker', prompt: 'Task: T2' })
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })

  expect(STEP_NANO).toBeGreaterThan(0)
  expect(session.book().jobs[0]?.spentNano).toBe(2 * STEP_NANO)
  expect(session.book().jobs[0]?.tokens).toEqual({ input: 4_000, output: 800_000, cacheRead: 60_000, cacheCreation: 10_000 })
  expect(underLimit.deny).toBeUndefined()
  expect(atLimit.deny).toBe(`rex gate: job ${JOB} has spent $${usd(2 * STEP_NANO)} of its $${usd(STEP_NANO + 1)} limit; a person raises it with /rex limit ${JOB} <usd>`)
  expect(lineOf(status, 'spent_usd')).toBe(usd(2 * STEP_NANO))
})

test('a step of the main loop charges no job when the main loop builds none', async ($, on) => {
  files(on, {})
  engine(on)
  const session = world(on, { seed: seededBook({ builder: 'b1' }) })
  await $.session.start(START)

  const stream = $.turn.step({ turnId: 't-main', index: 1, model: USAGE.model, effort: 'medium', messageCount: 1 })
  for await (const chunk of stream) {
    expect(chunk).toBeDefined()
  }
  await stream.result

  expect(session.book().jobs[0]?.spentNano).toBe(0)
})

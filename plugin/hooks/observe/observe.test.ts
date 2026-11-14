// Proves the run tree the observers build: the 0.7 capture fixture replayed through the engine gives its parents, depths and token totals.
import { expect, test } from 'claude-code/testing'
import type { TurnStepResult, TurnUsage } from 'claude-code'

import { ROOT, spawnInput, world } from '../book/kit'
import { CAPTURE_EVENTS, CAPTURE_TOTALS } from './capture'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const

test('tree_from_capture_fixtures_equals_totals', async ($, on) => {
  const session = world(on)
  const agentOf = new Map(CAPTURE_EVENTS.flatMap(event => (event.kind === 'spawn' ? [[event.description, event.agentId] as const] : [])))
  let usage: TurnUsage | null = null
  on('agent.spawn', (_$, e) => ({ model: 'claude-haiku-5-5', agentId: agentOf.get(e.description) ?? 'unknown' }))
  on('turn.step', async function* (_$, e) {
    const result: TurnStepResult = { turnId: e.turnId, index: e.index, answer: '', toolUses: [], stopReason: 'end_turn', usage }
    return result
  })
  on('turn.complete', (_$, e) => ({ text: e.answer }))

  await $.session.start(START)
  let index = 0
  for (const event of CAPTURE_EVENTS) {
    index += 1
    switch (event.kind) {
      case 'spawn': {
        const parent = event.parentAgentId === null ? {} : { parentAgentId: event.parentAgentId }
        await $.agent.spawn(spawnInput({ description: event.description, subagentType: event.subagentType, tool_use_id: event.toolUseId, background: event.background, ...parent }))
        break
      }
      case 'step': {
        usage = { model: event.model, input_tokens: event.input, output_tokens: event.output, cache_read_input_tokens: event.cacheRead, cache_creation_input_tokens: event.cacheCreation }
        const stream = $.turn.step({ turnId: `t-${event.agentId}`, index, model: event.model, effort: event.effort, messageCount: 1, agentId: event.agentId })
        for await (const chunk of stream) {
          expect(chunk).toBeDefined()
        }
        await stream.result
        break
      }
      case 'complete':
        await $.turn.complete({ agentId: event.agentId, reason: 'answer', answer: 'done', durationMs: 1, isAborted: false, turnId: `t-${event.agentId}` })
        break
      default:
        throw new Error(`unknown capture event ${JSON.stringify(event)}`)
    }
  }

  const runs = session.book().runs
  expect(runs.map(run => run.agentId).sort()).toEqual(CAPTURE_TOTALS.map(total => total.agentId).sort())
  for (const total of CAPTURE_TOTALS) {
    const run = runs.find(each => each.agentId === total.agentId)
    expect(run?.parent, total.agentId).toBe(total.parent)
    expect(run?.depth, total.agentId).toBe(total.depth)
    expect(run?.tokens, total.agentId).toEqual({ input: total.input, output: total.output, cacheRead: total.cacheRead, cacheCreation: total.cacheCreation })
    expect(run?.state, total.agentId).toBe('done')
    expect(run?.marks, total.agentId).toEqual([])
  }
})

// Fitness test of the third Never line: an observer that throws never blocks a prompt, a tool call or a spawn, and its job is marked degraded.
import { expect, test } from 'claude-code/testing'
import type { AgentSpawnInput, PromptSubmitInput, TurnStepResult } from 'claude-code'

import type { RexBook } from '../../types'
import { emptyBook } from '../book/events'
import { openJobIn } from '../book/jobs'
import { ROOT, SESSION, START_MS, spawnInput, world } from '../book/kit'
import { MERGE_NO, MERGE_QUESTION, MERGE_YES } from './index'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const BAND = {
  plugin: 'super-rex',
  component: 'AbovePrompt',
  props: { hasSurvey: false, isCollapsed: false, isWorking: true, maxRows: 3, bodyColumns: 100, scroll: { offset: 0, bodyRows: 3 }, view: {} },
} as const

function bookWithJob(): RexBook {
  const input = {
    builder: 'main',
    tree: null,
    origin: 'composer',
    request: 'Fix the parser',
    repo: ROOT,
    base: 'main',
    type: null,
    size: null,
    consequence: null,
    langs: [],
    prefs: { duration: null, budget: null, autonomy: null },
    limit: { nano: 10_000_000_000, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  return openJobIn({ ...emptyBook(), session: SESSION }, input, '0gaq820001', START_MS).book
}

test('observer_that_throws_never_blocks', async ($, on) => {
  const session = world(on, { seed: bookWithJob() })
  const prompts: PromptSubmitInput[] = []
  const spawns: AgentSpawnInput[] = []
  const tools: string[] = []
  const ends: string[] = []
  on('prompt.submit', (_$, e) => {
    prompts.push(e)
    return { text: e.text }
  })
  on('agent.spawn', (_$, e) => {
    spawns.push(e)
    return { model: 'claude-haiku-5-5', agentId: 'a1' }
  })
  on('tool.call', (_$, e) => {
    tools.push(e.tool)
    return { result: { questions: [], answers: { [MERGE_QUESTION]: MERGE_YES } } }
  })
  on('turn.step', async function* (_$, e) {
    const usage = { model: e.model, input_tokens: 2 ** 60, output_tokens: 1, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 }
    const result: TurnStepResult = { turnId: e.turnId, index: e.index, answer: '', toolUses: [], stopReason: 'end_turn', usage }
    return result
  })
  on('session.end', (_$, e) => {
    ends.push(e.reason)
    return { sessionId: e.sessionId }
  })
  on('ui.render', ($h, e) => {
    const { Text } = $h.ui.resolve(e)
    return <Text>engine</Text>
  })
  await $.session.start(START)

  session.failNextBookRead()
  const prompt = { text: '/super-rex:orchestration-builder Fix the parser', command: { name: 'super-rex:orchestration-builder', args: 'Fix the parser' }, origin: { kind: 'composer' }, wait: false } as const
  await $.prompt.submit(prompt)

  session.failNextBookRead()
  const spawn = spawnInput({ description: 'worker' })
  const spawned = await $.agent.spawn(spawn)

  session.failNextBookRead()
  const options = [
    { label: MERGE_YES, description: 'Merge when the checks pass' },
    { label: MERGE_NO, description: 'Keep the branch' },
  ]
  const called = await $.tool.call({ tool: 'AskUserQuestion', questions: [{ question: MERGE_QUESTION, header: 'Merge', multiSelect: false, options }] })

  const stream = $.turn.step({ turnId: 't1', index: 0, model: 'claude-haiku-5-5', effort: 'low', messageCount: 1, agentId: 'a1' })
  for await (const chunk of stream) {
    expect(chunk).toBeDefined()
  }
  await stream.result

  session.failNextBookRead()
  const band = await $.ui.mount({ ...BAND, surface: 'terminal' })
  expect((await band.find({ type: 'Text' }))?.text).toBe('engine')
  await band.unmount()
  await session.clock.settle()

  session.failNextBookRead()
  await $.session.end({ reason: 'other', sessionId: SESSION, resume: { id: SESSION } })
  await session.clock.settle()

  expect(prompts.map(e => [e.text, e.context])).toEqual([[prompt.text, undefined]])
  expect(spawns).toEqual([spawn])
  expect(spawned).toEqual({ model: 'claude-haiku-5-5', agentId: 'a1' })
  expect(tools).toEqual(['AskUserQuestion'])
  expect(called.result).toEqual({ questions: [], answers: { [MERGE_QUESTION]: MERGE_YES } })
  expect(ends).toEqual(['other'])
  expect(session.book().jobs[0]?.degraded).toEqual([
    'observer:prompt.submit:prompts',
    'observer:agent.spawn:observe',
    'observer:tool.call:observe',
    'observer:turn.step:observe',
    'observer:ui.render:AbovePrompt:pane',
    'observer:session.end:ingest',
  ])
})

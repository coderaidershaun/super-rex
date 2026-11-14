// Proves the master's four tools against the shared tools table that a Rust test checks the skill text with: each key the table lists is one a tool answers, and each key a tool answers is listed.
import { expect, test } from 'claude-code/testing'

import { sharedJson } from '../book/shared'
import { buildRules, byRule, confirmedPolicy, lineOf, MASTER, masterWorld, POLICY_KEY, resultText, START, treeBook } from './master-kit'

function field(value: unknown, key: string): unknown {
  return typeof value === 'object' && value !== null ? Reflect.get(value, key) : undefined
}

function listedKeys(tool: string): readonly string[] {
  const keys = field(field(field(sharedJson('fixtures/shared/tables/tools.json'), 'tools'), tool), 'keys')
  return Array.isArray(keys) ? keys.map(String).sort() : []
}

function answeredKeys(results: readonly unknown[]): readonly string[] {
  const keys = results.flatMap(result =>
    resultText(result)
      .split('\n')
      .filter(line => line.includes('='))
      .map(line => line.slice(0, line.indexOf('='))),
  )
  return [...new Set(keys)].sort()
}

test('rex_build, rex_continue, rex_note and rex_ask_owner answer exactly the keys of the shared tools table', async ($, on) => {
  const policy = await confirmedPolicy()
  masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)

  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', agentId: MASTER, request: 'Fix the parser', consequence: 'low' })
  const job = lineOf(built, 'job') ?? ''
  const continued = await $.tool.call({ tool: 'mcp__super-rex__rex_continue', agentId: MASTER, job, message: 'Go on.' })
  const noted = await $.tool.call({ tool: 'mcp__super-rex__rex_note', agentId: MASTER, text: 'One step.' })
  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, question: 'Go?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })

  expect(answeredKeys([built])).toEqual(listedKeys('rex_build'))
  expect(answeredKeys([continued])).toEqual(listedKeys('rex_continue'))
  expect(answeredKeys([noted])).toEqual(listedKeys('rex_note'))
  expect(answeredKeys([asked])).toEqual(listedKeys('rex_ask_owner'))
})

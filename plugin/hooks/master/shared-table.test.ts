// Proves the master's tools against the shared tools table that a Rust test checks the skill text with: each key the table lists is one a tool answers, and each key a tool answers is listed.
import { expect, test } from 'claude-code/testing'

import { ROOT } from '../book/kit'
import { sharedJson } from '../book/shared'
import { buildRules, byRule, firstCommitRules, confirmedPolicy, KIND, lineOf, masterWorld, openedTree, placesRules, POLICY_KEY, ran, resultText, START, treeBook } from './master-kit'

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
  const unreachable = { starts: ['git', '-C', ROOT, 'fetch'], answer: { exitCode: 128, stdout: '', stderr: "fatal: '/home/person/origin.git' does not appear to be a git repository\n" } }
  masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule([...firstCommitRules(), unreachable, ...buildRules('model-high-effort-high')]) })
  await $.session.start(START)

  // A repo with no commit and an origin that cannot be fetched, so rex_build answers base_commit and base_stale too.
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })
  const job = lineOf(built, 'job') ?? ''
  const continued = await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job, message: 'Go on.' })
  const noted = await $.tool.call({ tool: 'mcp__super-rex__rex_note', text: 'One step.' })
  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', question: 'Go?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })

  expect(answeredKeys([built])).toEqual(listedKeys('rex_build'))
  expect(answeredKeys([continued])).toEqual(listedKeys('rex_continue'))
  expect(answeredKeys([noted])).toEqual(listedKeys('rex_note'))
  expect(answeredKeys([asked])).toEqual(listedKeys('rex_ask_owner'))
})

test('rex_report answers exactly the keys of the shared tools table', async ($, on) => {
  const policy = await confirmedPolicy()
  masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy } })
  await $.session.start(START)

  const reported = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Result: nothing to build.' })

  expect(answeredKeys([reported])).toEqual(listedKeys('rex_report'))
})

/** The keys of the `key=value` lines before the blank line that starts a tree's brief, whose blocks hold lines of their own. */
function headKeys(results: readonly unknown[]): readonly string[] {
  return answeredKeys(results.map(result => ({ result: resultText(result).split('\n\n')[0] ?? '' })))
}

test('rex_tree_open answers exactly the keys of the shared tools table, a new tree and a resumed one together', async ($, on) => {
  const policy = await confirmedPolicy()
  const shown = { tree: { id: 't1', session_id: 'old', master_agent_id: null, charter: 'Ship the parser.', charter_hash: 'sha256:c', policy_hash: policy.confirmed_hash, state: 'running', cap_usd: 150, report_file: null, started_at: '2026-11-13T19:00:00.000Z', closed_at: null }, jobs: [], questions: [] }
  const session = masterWorld(on, { store: { [POLICY_KEY]: policy }, processAnswer: byRule([{ starts: ['rex', 'tree', 'show'], answer: ran(`${JSON.stringify(shown)}\n`) }, ...placesRules('origin')]) })
  await $.session.start(START)

  const opened = await openedTree($, 'Ship the lexer')
  await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Result: the lexer ships.' })
  const resumed = await openedTree($, 'resume')

  expect([lineOf(opened, 'resumed'), lineOf(resumed, 'resumed')]).toEqual(['no', 'yes'])
  expect(session.book().trees.map(tree => tree.id)).toContain('t1')
  expect(headKeys([opened, resumed])).toEqual(listedKeys('rex_tree_open'))
})

// Proves the master view of the watch pane: each tree with its state, pinned policy and spend, then each job the master built and each decision it noted.
import { expect, test } from 'claude-code/testing'

import { buildRules, byRule, confirmedPolicy, lineOf, MASTER, masterWorld, POLICY_KEY, START, treeBook } from './master-kit'

const PANE = {
  plugin: 'super-rex',
  component: 'Pane',
  requestId: 'rex',
  surface: 'terminal',
  props: { title: 'SuperRex', isFocused: true, bodyColumns: 120, placement: 'dock', scroll: { offset: 0, bodyRows: 40 }, view: {}, id: 'rex', plugin: 'super-rex' },
} as const

async function texts(ui: { readonly findAll: (query: { readonly type: string }) => Promise<readonly { readonly text: string }[]> }): Promise<string[]> {
  return (await ui.findAll({ type: 'Text' })).map(found => found.text)
}

test('the master view shows each tree, the jobs its master built and the decisions it noted', async ($, on) => {
  const policy = await confirmedPolicy()
  masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', agentId: MASTER, request: 'Fix the parser', consequence: 'low', size: 'small' })
  const job = lineOf(built, 'job') ?? ''
  await $.tool.call({ tool: 'mcp__super-rex__rex_note', agentId: MASTER, text: 'One builder: the parser is one step.', job })

  const ui = await $.ui.mount(PANE)
  await ui.press({ key: 'view-master' })

  const shown = await texts(ui)
  expect(shown.slice(3)).toEqual([
    `master tree t1 running · master m1 · policy ${policy.confirmed_hash?.slice(0, 15) ?? ''} · $0.00 of $150.00`,
    'charter: Ship the parser.',
    `${job} created · builder b1 · $0.00/$28.00 · merge not answered`,
    `note: ${job}: One builder: the parser is one step.`,
  ])
  await ui.unmount()
})

test('the master view says how to start a tree when none runs', async ($, on) => {
  masterWorld(on)
  await $.session.start(START)

  const ui = await $.ui.mount(PANE)
  await ui.press({ key: 'view-master' })

  expect((await texts(ui)).slice(3)).toEqual(['No master tree. Start one with /rex master <goal>.'])
  await ui.unmount()
})

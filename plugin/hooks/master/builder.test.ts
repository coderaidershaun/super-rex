// Proves that a builder rex_build started is a builder everywhere: its tool calls reach the dispatcher, the gate and the tally, its agents take roles from its plan, and only under it may agents run four deep.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import { spawnInput } from '../book/kit'
import { AUTHOR } from '../pipeline/assembly.fixture'
import { buildRules, byRule, confirmedPolicy, lineOf, MASTER, masterWorld, omsAnswer, POLICY_KEY, START, treeBook } from './master-kit'
import type { Rule } from './master-kit'

const BUILD = 'mcp__super-rex__rex_build' as const

async function builtTree($: Engine, on: On, perJobUsd: number | null, rules: readonly Rule[] = buildRules('model-high-effort-high')) {
  const policy = await confirmedPolicy({ per_job_usd: perJobUsd })
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(rules) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'Fix the parser', consequence: 'low' })
  return { session, job: lineOf(built, 'job') ?? '', builder: lineOf(built, 'builder') ?? '' }
}

async function step($: Engine, agentId: string): Promise<void> {
  const stream = $.turn.step({ turnId: `turn-${agentId}`, index: 0, model: 'claude-sonnet-5-5', effort: 'high', messageCount: 1, agentId })
  for await (const chunk of stream) {
    expect(chunk).toBeDefined()
  }
  await stream.result
}

test('builder_tool_calls_reach_dispatcher_and_gate', async ($, on) => {
  const limit: Rule = { starts: ['rex', 'limit'], answer: { exitCode: 0, stdout: '{"limit_usd":0.000001,"source":"policy"}\n', stderr: '' } }
  const { session, job, builder } = await builtTree($, on, 0.000001, [limit, ...buildRules('model-high-effort-high')])

  const opened = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', agentId: builder, request: 'anything' })
  await step($, builder)
  await step($, MASTER)
  const agent = await $.tool.call({ tool: 'Agent', agentId: builder, description: 'worker', prompt: AUTHOR })
  const nested = await $.tool.call({ tool: BUILD, agentId: builder, request: 'x', consequence: 'low' })

  expect([lineOf(opened, 'job'), lineOf(opened, 'existing')]).toEqual([job, 'yes'])
  const book = session.book()
  const spentJob = book.jobs.find(each => each.id === job)?.spentNano ?? 0
  const builderRun = book.runs.find(run => run.agentId === builder)
  expect(spentJob).toBeGreaterThan(0)
  expect(builderRun?.spentNano).toBe(spentJob)
  expect(book.trees[0]?.ledgerNano).toBe(book.runs.find(run => run.agentId === MASTER)?.spentNano)
  expect(book.trees[0]?.ledgerNano).toBeGreaterThan(0)
  expect(agent.deny).toMatch(new RegExp(`^rex gate: job ${job} has spent \\$\\d+\\.\\d\\d of its \\$0\\.00 limit`))
  expect(nested.deny).toBe('rex_build: only the master may call it, got builder')
})

test('role_from_plan_by_ancestry: an agent the builder starts takes the plan role of its grid agent', async ($, on) => {
  const rules: readonly Rule[] = [{ starts: ['rex', 'oms', '--json', '--duration', 'none', '--budget', 'none', '--autonomy', 'high', '--intelligence', 'medium'], answer: omsAnswer('model-medium-effort-medium') }, ...buildRules('model-high-effort-high')]
  const { session, builder } = await builtTree($, on, null, rules)

  const planned = await $.tool.call({
    tool: 'mcp__super-rex__rex_job_plan',
    agentId: builder,
    orchestrator: 'orchestration-builder 0.8.0',
    breakdown: 'T2 engineer',
    tasks: [{ id: 'T2', role: 'engineer', skills: [], open_when: [] }],
    roles: [{ role: 'engineer', intelligence: 'medium', reliability: 'medium', consequence: 'low', verification: 'medium' }],
  })
  await $.agent.spawn(spawnInput({ description: 'engineer', subagentType: 'super-rex:model-medium-effort-medium', parentAgentId: builder, prompt: AUTHOR }))
  await $.agent.spawn(spawnInput({ description: 'helper', subagentType: 'general-purpose', parentAgentId: builder, prompt: AUTHOR }))

  expect(lineOf(planned, 'role.engineer')).toBe('super-rex:model-medium-effort-medium')
  const runs = session.book().runs
  expect(runs.find(run => run.agentId === 'a2')?.role).toBe('engineer')
  expect(runs.find(run => run.agentId === 'a3')?.role).toBe(null)
  expect(runs.filter(run => run.parent === builder).map(run => run.job)).toEqual([session.book().jobs[0]?.id, session.book().jobs[0]?.id])
})

test('depth_4_allowed_only_below_master: four deep under a builder of the tree, refused outside a tree', async ($, on) => {
  const { session, builder } = await builtTree($, on, null)

  const lead = await $.agent.spawn(spawnInput({ description: 'lead', parentAgentId: builder, prompt: AUTHOR, tool_use_id: 'toolu_lead' }))
  const worker = await $.agent.spawn(spawnInput({ description: 'worker', parentAgentId: lead.agentId ?? '', prompt: AUTHOR, tool_use_id: 'toolu_worker' }))
  const fanout = await $.agent.spawn(spawnInput({ description: 'fanout', parentAgentId: worker.agentId ?? '', prompt: AUTHOR, tool_use_id: 'toolu_fanout' }))
  const x1 = await $.agent.spawn(spawnInput({ description: 'x1', tool_use_id: 'toolu_x1' }))
  const x2 = await $.agent.spawn(spawnInput({ description: 'x2', parentAgentId: x1.agentId ?? '', tool_use_id: 'toolu_x2' }))
  const x3 = await $.agent.spawn(spawnInput({ description: 'x3', parentAgentId: x2.agentId ?? '', tool_use_id: 'toolu_x3' }))
  const outside = await $.agent.spawn(spawnInput({ description: 'outside', parentAgentId: x3.agentId ?? '', tool_use_id: 'toolu_outside' }))

  const runs = session.book().runs
  expect([lead.deny, worker.deny, fanout.deny]).toEqual([undefined, undefined, undefined])
  expect(runs.find(run => run.agentId === fanout.agentId)?.depth).toBe(4)
  expect(outside.deny).toBe('rex gate: depth 4 is for master trees only; start this work one layer up')
})

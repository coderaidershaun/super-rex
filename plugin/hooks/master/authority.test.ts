// Proves Never 2 of the priority block over generated cases: a master starts only from the person's /rex master, and a builder is linked to a job only by rex_build of its tree's master.
import { expect, test } from 'claude-code/testing'
import type { PromptOrigin } from 'claude-code'

import { spawnInput } from '../book/kit'
import { buildRules, byRule, confirmedPolicy, lineOf, MASTER, masterWorld, POLICY_KEY, rex, START, TREE, treeBook } from './master-kit'

const SPAWN_TYPES = ['super-rex:master', 'super-rex:model-high-effort-high'] as const
const DESCRIPTIONS = ['rex master t1', 'rex master t9', 'rex builder 0gaq820001', 'master'] as const
const PARENTS = [undefined, MASTER, 'x9'] as const
const OTHER_ORIGINS: readonly PromptOrigin[] = [{ kind: 'sdk' }, { kind: 'peer' }, { kind: 'task-notification' }, { kind: 'plugin', name: 'other' }]

test('master_and_builder_start_only_from_person_or_master_tool', { timeoutMs: 30_000 }, async ($, on) => {
  const policy = await confirmedPolicy()
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', agentId: MASTER, request: 'Fix the parser', consequence: 'low' })
  const builder = lineOf(built, 'builder') ?? ''
  const job = lineOf(built, 'job') ?? ''

  const modelSpawned: string[] = []
  for (const subagentType of SPAWN_TYPES) {
    for (const description of DESCRIPTIONS) {
      for (const parentAgentId of PARENTS) {
        const parent = parentAgentId === undefined ? {} : { parentAgentId }
        const spawned = await $.agent.spawn(spawnInput({ subagentType, description, ...parent }))
        if (spawned.agentId !== undefined) {
          modelSpawned.push(spawned.agentId)
        }
      }
    }
  }
  const callers = [undefined, builder, 'x9', ...modelSpawned]
  const builds = []
  for (const agentId of callers) {
    const caller = agentId === undefined ? {} : { agentId }
    builds.push(await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...caller, request: 'Build it too', consequence: 'low' }))
  }
  const entries = []
  for (const origin of OTHER_ORIGINS) {
    entries.push(await $.command.run(rex('master Ship everything', origin)))
  }
  const offered = await $.agent.offer({ agent: 'super-rex:master', description: 'master', source: 'plugin', provider: { plugin: 'super-rex', tier: 'user' } })

  const book = session.book()
  expect(modelSpawned).toHaveLength(SPAWN_TYPES.length * DESCRIPTIONS.length * PARENTS.length)
  expect(builds.map(each => each.deny === undefined)).toEqual(callers.map(() => false))
  expect(entries.map(entry => entry.text?.includes('only your own typing may run this'))).toEqual(OTHER_ORIGINS.map(() => true))
  expect(offered.isOffered).toBe(false)
  expect(book.trees.map(tree => [tree.id, tree.master, tree.builders])).toEqual([[TREE, MASTER, [{ job, agentId: builder }]]])
  expect(book.runs.filter(run => run.tree !== null).map(run => run.agentId)).toEqual([MASTER])
  expect(book.jobs.map(each => [each.id, each.owner.builder])).toEqual([[job, builder]])
})

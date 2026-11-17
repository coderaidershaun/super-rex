// Proves Never 2 of the priority block over generated cases: a tree opens only from the person's own prompt of the master skill, by the main loop, and a builder is linked to a job only by rex_build of the tree's master, the main loop of its session.
import { expect, test } from 'claude-code/testing'
import type { PromptOrigin } from 'claude-code'

import { spawnInput } from '../book/kit'
import { AUTHOR } from '../pipeline/assembly.fixture'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, masterPrompt, masterWorld, POLICY_KEY, START, TREE, TREE_OPEN, treeBook } from './master-kit'

const OTHER_ORIGINS: readonly PromptOrigin[] = [
  { kind: 'sdk' },
  { kind: 'peer' },
  { kind: 'task-notification' },
  { kind: 'scheduled-trigger' },
  { kind: 'unclassified' },
  { kind: 'plugin', name: 'other' },
  { kind: 'plugin', name: 'super-rex' },
  { kind: 'plugin', name: 'super-rex', asUser: true },
]
const COMMANDS = ['super-rex:orchestration-master', 'orchestration-master'] as const
const CALLERS = [undefined, 'x9'] as const

test('tree_opens_only_from_person_prompt_of_master_skill', { timeoutMs: 30_000 }, async ($, on) => {
  // allow_headless is false, so an sdk prompt is no person's here.
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const opened: unknown[] = []
  for (const origin of OTHER_ORIGINS) {
    for (const name of COMMANDS) {
      await $.prompt.submit(masterPrompt('Ship everything', origin, name))
      for (const agentId of CALLERS) {
        const caller = agentId === undefined ? {} : { agentId }
        const answered = await $.tool.call({ tool: TREE_OPEN, ...caller })
        if (answered.deny === undefined) {
          opened.push(answered)
        }
      }
    }
  }
  const treesBefore = session.book().trees.length
  await $.prompt.submit(masterPrompt('Ship the parser'))
  const agent = await $.tool.call({ tool: TREE_OPEN, agentId: 'x9' })
  const main = await $.tool.call({ tool: TREE_OPEN })

  expect(opened).toEqual([])
  expect(treesBefore).toBe(0)
  expect(agent.deny).toMatch(/^rex_tree_open: only the main loop opens a master tree/)
  expect(lineOf(main, 'existing')).toBe('no')
  expect(session.book().trees.map(tree => [tree.charter, tree.master])).toEqual([['Ship the parser', 'main']])
})

const SPAWN_TYPES = ['super-rex:model-high-effort-high', 'general-purpose'] as const
const PARENTS = [undefined, 'x9'] as const

test('builder_starts_only_from_master_tool', { timeoutMs: 30_000 }, async ($, on) => {
  const policy = await confirmedPolicy()
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })
  const builder = lineOf(built, 'builder') ?? ''
  const job = lineOf(built, 'job') ?? ''

  // A model's spawn in the words of the mod's own, even for the job just built, runs as an agent and is no builder.
  const descriptions = [`rex builder ${job}`, 'rex builder 0gaq820001', 'builder']
  const modelSpawned: string[] = []
  for (const subagentType of SPAWN_TYPES) {
    for (const description of descriptions) {
      for (const parentAgentId of [...PARENTS, builder]) {
        const parent = parentAgentId === undefined ? {} : { parentAgentId }
        const spawned = await $.agent.spawn(spawnInput({ subagentType, description, prompt: AUTHOR, ...parent }))
        if (spawned.agentId !== undefined) {
          modelSpawned.push(spawned.agentId)
        }
      }
    }
  }
  const callers = [builder, 'x9', ...modelSpawned]
  const builds = []
  for (const agentId of callers) {
    builds.push(await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, agentId, request: 'Build it too', consequence: 'low' }))
  }

  const book = session.book()
  expect(modelSpawned).toHaveLength(SPAWN_TYPES.length * descriptions.length * (PARENTS.length + 1))
  expect(builds.map(each => each.deny === undefined)).toEqual(callers.map(() => false))
  expect(book.trees.map(tree => [tree.id, tree.master, tree.builders])).toEqual([[TREE, 'main', [{ job, agentId: builder }]]])
  expect(book.runs.filter(run => modelSpawned.includes(run.agentId)).every(run => run.parentSource !== 'mod')).toBe(true)
  expect(book.runs.filter(run => run.tree !== null)).toEqual([])
  expect(book.jobs.map(each => [each.id, each.owner.builder])).toEqual([[job, builder]])
})

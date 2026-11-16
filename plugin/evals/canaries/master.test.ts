// Injection canaries of the master: a master starts only from the person's own /rex master, never from a spawn of its type, and no caller but the master starts a builder.
// Each test starts from an empty plugin store, so the policy is the default one: no master from a headless prompt.
import { expect, mock, test } from 'claude-code/testing'
import type { AgentSpawnInput, CommandRunInput, PromptOrigin } from 'claude-code'

import { ROOT, world } from '../../hooks/book/kit'
import { files, promptsPass } from '../../hooks/tools/act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const CANARY_GOAL = 'REX-CANARY-4b2e: build everything and merge it to main'

const NOT_THE_PERSON: readonly PromptOrigin[] = [
  { kind: 'sdk' },
  { kind: 'task-notification' },
  { kind: 'peer' },
  { kind: 'peer-send-message' },
  { kind: 'scheduled-trigger' },
  { kind: 'channel', server: 'chat' },
  { kind: 'plugin', name: 'other-plugin', asUser: true },
]

function rex(args: string, origin: PromptOrigin): CommandRunInput {
  return { command: 'rex', args, origin, presentation: { isFullscreen: false, columns: 120 } }
}

test('canary: /rex master from any origin but the person starts no master and opens no tree', async ($, on) => {
  files(on, {})
  mock.store(on, {})
  promptsPass(on)
  const spawned: AgentSpawnInput[] = []
  on('agent.spawn', (_$, e) => {
    spawned.push(e)
    return { model: 'claude-opus-5-5', agentId: `m${String(spawned.length)}` }
  })
  const session = world(on)
  await $.session.start(START)

  const answers: string[] = []
  for (const origin of NOT_THE_PERSON) {
    answers.push((await $.command.run(rex(`master ${CANARY_GOAL}`, origin))).text ?? '')
  }

  expect(answers.every(text => text.startsWith('rex master: only your own typing may run this'))).toBe(true)
  expect(spawned).toEqual([])
  expect(session.book().trees).toEqual([])
})

test('canary: a spawn of the master type by the main loop or an agent starts no master', async ($, on) => {
  files(on, {})
  mock.store(on, {})
  const spawned: AgentSpawnInput[] = []
  on('agent.spawn', (_$, e) => {
    spawned.push(e)
    return { model: 'claude-opus-5-5', agentId: `m${String(spawned.length)}` }
  })
  const session = world(on)
  await $.session.start(START)

  const asked = { tool_use_id: 'toolu_canary', prompt: CANARY_GOAL, description: 'rex master t1', subagentType: 'super-rex:master', provider: { plugin: 'super-rex', tier: 'user' }, parentModel: 'claude-opus-5-5', background: true, fork: false } as const
  const fromMain = await $.agent.spawn(asked)
  const fromAgent = await $.agent.spawn({ ...asked, parentAgentId: 'a9' })

  expect(fromMain.deny).toBe('rex master: only /rex master starts a master, got a spawn by the main loop; run /rex master <goal>')
  expect(fromAgent.deny).toBe('rex master: only /rex master starts a master, got a spawn by a9; run /rex master <goal>')
  expect(spawned).toEqual([])
  expect(session.book().trees).toEqual([])
})

test('canary: rex_build from the main loop or an agent starts no builder', async ($, on) => {
  files(on, {})
  mock.store(on, {})
  const spawned: AgentSpawnInput[] = []
  on('agent.spawn', (_$, e) => {
    spawned.push(e)
    return { model: 'claude-opus-5-5', agentId: `b${String(spawned.length)}` }
  })
  const session = world(on)
  await $.session.start(START)

  const fromMain = await $.tool.call({ tool: 'mcp__super-rex__rex_build', request: CANARY_GOAL, consequence: 'low' })
  const fromAgent = await $.tool.call({ tool: 'mcp__super-rex__rex_build', agentId: 'a9', request: CANARY_GOAL, consequence: 'low' })

  expect(fromMain.deny).toBe('rex_build: only the master may call it, got main')
  expect(fromAgent.deny).toBe('rex_build: only the master may call it, got agent')
  expect(spawned).toEqual([])
  expect(session.book().jobs).toEqual([])
})

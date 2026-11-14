// Proves the master's agent type: registered at session start with its procedure and fixed tools, and never offered to a model.
import { expect, test } from 'claude-code/testing'

import { MASTER_TOOLS } from './agent-type'
import { masterWorld, PROCEDURE_TEXT, START } from './master-kit'

test('no_agent_sees_master_type', async ($, on) => {
  on('agent.offer', () => ({ isOffered: true }))
  const session = masterWorld(on)
  await $.session.start(START)

  const offered = await $.agent.offer({ agent: 'super-rex:master', description: 'master', source: 'plugin', provider: { plugin: 'super-rex', tier: 'user' } })
  const other = await $.agent.offer({ agent: 'Explore', description: 'explore', source: 'built-in', provider: { plugin: 'engine', tier: 'core' } })

  expect(offered.isOffered).toBe(false)
  expect(other.isOffered).toBe(true)
  expect(session.registered).toEqual([{ name: 'master', prompt: PROCEDURE_TEXT, tools: MASTER_TOOLS, model: 'opus', effort: 'high' }])
  expect(MASTER_TOOLS).toContain('ToolSearch')
  expect(MASTER_TOOLS).not.toContain('Agent')
})

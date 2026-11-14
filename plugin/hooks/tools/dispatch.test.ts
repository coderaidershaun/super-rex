// Proves the tool table: every entry registered with the engine at session start, the caller read from the engine and never from the call's text, and the dispatcher's answers.
import { expect, test } from 'claude-code/testing'
import type { ToolSpec } from 'claude-code'

import { ROOT, world } from '../book/kit'
import { createParts } from '../parts'
import { files, JOB, lineOf, resultText, seededBook } from './act-kit'
import { dispatchAnswer, toolSpecs } from './index'
import { registerProbeTool } from './probe-tool.fixture'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const

test('every tool of the table is registered at session start, and only rex_skill and rex_status load at once', async ($, on) => {
  const specs: ToolSpec[] = []
  on('tool.register', { name: /(?:)/ }, (_$, e) => {
    specs.push(e)
    return { value: { tool: `mcp__super-rex__${e.name}` } }
  })
  world(on)
  await $.session.start(START)

  expect(specs.map(spec => [spec.name, spec.isDeferred])).toEqual([
    ['rex_assign', true],
    ['rex_skill', false],
    ['rex_merge', true],
    ['rex_status', false],
    ['rex_job_open', true],
    ['rex_recall', true],
    ['rex_job_plan', true],
    ['rex_job_close', true],
  ])
})

test('caller from agentId, never from text', async ($, on) => {
  files(on, {})
  world(on, { seed: seededBook() })
  await $.session.start(START)

  const claimed = await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', status: 'blocked', outcome: 'x', deliveries: [], caller: 'builder', builder: 'main', agentId: 'a9' })
  const own = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: 'a9', job: undefined, role: 'builder' })
  const main = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })

  expect(claimed.deny).toBe('rex_job_close: only a builder may call it, got agent')
  expect(resultText(own)).toBe('job=none\n')
  expect(lineOf(main, 'job')).toBe(JOB)
})

test('a tool another module adds is served by its own hook, and the dispatcher lists it and denies a caller it steps past and any unknown name', async ($, on) => {
  const parts = createParts()
  const book = seededBook()
  registerProbeTool(on, parts, 'mcp__probe__')
  on('tool.call', { tool: /^mcp__probe__/ }, (_$, e) => ({ deny: dispatchAnswer(book, e, parts.tools, 'mcp__probe__') }))
  world(on, { seed: book })
  await $.session.start(START)

  const served = await $.tool.call({ tool: 'mcp__probe__rex_probe' })
  const stepped = await $.tool.call({ tool: 'mcp__probe__rex_probe', agentId: 'a9' })
  const unknown = await $.tool.call({ tool: 'mcp__probe__rex_nope' })
  const real = await $.tool.call({ tool: 'mcp__super-rex__rex_nope' })

  expect(toolSpecs(parts.tools).map(spec => spec.name)).toEqual(['rex_probe'])
  expect(resultText(served)).toBe('probe=yes\n')
  expect(stepped.deny).toBe('rex_probe: only a builder may call it, got agent')
  expect(unknown.deny).toBe('mcp__probe__rex_nope: no such rex tool, got rex_nope')
  expect(real.deny).toBe('mcp__super-rex__rex_nope: no such rex tool, got rex_nope')
})

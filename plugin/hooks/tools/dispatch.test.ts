// Proves the tool table: every entry registered with the engine at session start, the caller read from the engine and never from the call's text (the main loop is the master only while it runs a live tree and builds no job), and the dispatcher's answers.
import { expect, test } from 'claude-code/testing'
import type { ToolSpec } from 'claude-code'

import { emptyBook } from '../book/events'
import { ROOT, SESSION, START_MS, world } from '../book/kit'
import { treeIn, treeStateIn } from '../book/trees'
import { callerIn } from './caller'
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
    ['rex_tree_open', true],
    ['rex_build', true],
    ['rex_continue', true],
    ['rex_cancel', true],
    ['rex_note', true],
    ['rex_ask_owner', true],
    ['rex_report', true],
    ['rex_assign', true],
    ['rex_skill', false],
    ['rex_merge', true],
    ['rex_status', false],
    ['rex_job_open', true],
    ['rex_recall', true],
    ['rex_job_plan', true],
    ['rex_job_close', true],
    ['rex_worktree', true],
  ])
})

test('the main loop is a builder while it owns an open job, else the master while a live tree of its session runs, else main; no agent is ever a master', () => {
  const input = { id: 't1', charter: 'Ship it.', charterHash: 'sha256:0', policyHash: 'sha256:1', state: 'running', capNano: 0 } as const
  const tree = treeIn({ ...emptyBook(), session: SESSION }, input, START_MS)
  const paused = treeStateIn(tree, 't1', 'paused', null, START_MS)
  const done = treeStateIn(tree, 't1', 'done', null, START_MS)
  const elsewhere = { ...tree, session: '11111111-2222-4333-8444-555555555555' }
  const asAgent = { ...tree, trees: tree.trees.map(each => ({ ...each, master: 'm1' })) }
  const withJob = { ...seededBook(), trees: tree.trees }

  expect([tree, paused, done, elsewhere, withJob].map(book => callerIn(book, undefined).kind)).toEqual(['master', 'master', 'main', 'main', 'builder'])
  expect(callerIn(tree, undefined)).toEqual({ kind: 'master', agentId: null, job: null })
  expect(callerIn(asAgent, 'm1').kind).toBe('agent')
  expect(callerIn(asAgent, undefined).kind).toBe('main')
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

// Proves how the book starts a session: from .rex/config.json, inert without one or with one it cannot read, and keeping its stream over a reload.
import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'
import type { On } from 'claude-code'

import { emptyBook, newIngest } from './events'
import { jobIdOf } from './ids'
import type { RexRole } from '../../types'
import { openJobIn, planJobIn, questionIn } from './jobs'
import { startRunIn, stepRunIn } from './runs'
import { sessionIn } from './session'
import { pendingIn, treeIn } from './trees'
import { CONFIG_ON, ROOT, SESSION, spawnInput, START_MS, world } from './kit'
import { seededBook } from '../tools/act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const

const TEST_OUTPUT = 'running 1 test\ntest parser::reads_empty ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored\n'

/** The engine beneath the plugin: each spawn starts, and each tool call answers the same test output. */
function engineBelow(on: On): { readonly started: string[]; readonly ran: string[] } {
  const started: string[] = []
  const ran: string[] = []
  on('agent.spawn', (_$, e) => {
    started.push(e.description)
    return { model: 'claude-sonnet-5-5', agentId: e.description }
  })
  on('tool.call', (_$, e) => {
    ran.push(e.tool === 'Bash' ? e.command : e.tool)
    return { result: { stdout: TEST_OUTPUT, stderr: '', interrupted: false } }
  })
  return { started, ran }
}

/** An inert mod acts on nothing: a push to main runs, a fourth layer of agents starts, and a test run's output stays whole. */
async function expectActsOnNothing($: Engine, below: ReturnType<typeof engineBelow>): Promise<void> {
  const push = await $.tool.call({ tool: 'Bash', command: 'git push origin main' })
  const merge = await $.tool.call({ tool: 'Bash', command: 'gh pr merge 12 --squash' })
  const spawns = []
  for (const [index, description] of ['d1', 'd2', 'd3', 'd4'].entries()) {
    const parent = index === 0 ? {} : { parentAgentId: `d${String(index)}` }
    spawns.push(await $.agent.spawn(spawnInput({ description, ...parent })))
  }
  const tested = await $.tool.call({ tool: 'Bash', command: 'cargo test' })

  expect([push.deny, merge.deny]).toEqual([undefined, undefined])
  expect(spawns.map(spawn => spawn.deny)).toEqual([undefined, undefined, undefined, undefined])
  expect(below.started).toEqual(['d1', 'd2', 'd3', 'd4'])
  expect(below.ran).toEqual(['git push origin main', 'gh pr merge 12 --squash', 'cargo test'])
  expect(tested.result).toEqual({ stdout: TEST_OUTPUT, stderr: '', interrupted: false })
}

test('unreadable_config_makes_mod_inert', async ($, on) => {
  const session = world(on, { config: '{' })
  const below = engineBelow(on)
  const prompts: string[] = []
  on('prompt.submit', (_$, e) => {
    prompts.push(e.text)
    return { text: e.text }
  })

  await $.session.start(START)
  await $.prompt.submit({ text: '/super-rex:orchestration-builder Fix the parser', origin: { kind: 'composer' }, wait: false })

  expect(session.book().inert).toStartWith('SyntaxError')
  expect(session.statuses).toEqual([expect.stringMatching(/^rex: \.rex\/config\.json unreadable: SyntaxError/)])
  expect(session.commands).toEqual([])
  expect(prompts).toEqual(['/super-rex:orchestration-builder Fix the parser'])
  expect(session.completions).toEqual([])
  await expectActsOnNothing($, below)
})

test('a session root with no .rex/config.json leaves the mod inert, and every hook passes', async ($, on) => {
  const session = world(on, { config: null })
  const below = engineBelow(on)

  await $.session.start(START)

  expect(session.book().inert).toBe('no .rex/config.json in the session root')
  expect([session.commands, session.tools, session.statuses]).toEqual([[], [], []])
  await expectActsOnNothing($, below)
})

test('a reload whose config became unreadable leaves the open job alone: no spend refusal, no fork refusal, no brief assembly', async ($, on) => {
  const book = seededBook()
  const spent = { ...book, jobs: book.jobs.map(job => ({ ...job, spentNano: 12_000_000_000 })) }
  world(on, { config: '{', seed: spent })
  const prompts: string[] = []
  on('agent.spawn', (_$, e) => {
    prompts.push(e.prompt)
    return { model: 'claude-sonnet-5-5', agentId: e.description }
  })
  on('tool.call', () => ({ result: 'ran' }))

  await $.session.start(START)
  const call = await $.tool.call({ tool: 'Agent', description: 'worker', prompt: 'Task: T1' })
  const spawn = await $.agent.spawn(spawnInput({ description: 'worker', prompt: 'Task: T1' }))
  const fork = await $.agent.spawn(spawnInput({ description: 'forked', fork: true }))

  expect([call.deny, spawn.deny, fork.deny]).toEqual([undefined, undefined, undefined])
  expect(prompts).toEqual(['Task: T1', 'Read the file.'])
})

test('a session with storage on opens a stream and queues its start', async ($, on) => {
  const session = world(on, { config: CONFIG_ON, processAnswer: () => ({ exitCode: 0, stdout: 'last_sequence=0\n', stderr: '' }) })

  await $.session.start(START)

  const book = session.book()
  expect(book.session).toBe(SESSION)
  expect(book.storage).toBe('on')
  expect(book.ingest?.stream).toMatch(/^[0-9a-f]{16}$/)
  expect(book.ingest?.secret).toMatch(/^[0-9a-f]{64}$/)
  expect(book.ingest?.queue.map(event => [event.seq, event.kind, event.data])).toEqual([
    [1, 'session.start', { claude_code: '2.1.323', plugin: '0.8.0', storage: 'on' }],
  ])
})

test('a reload of the same session keeps the book and its stream', async ($, on) => {
  const seed = { ...emptyBook(), session: SESSION, storage: 'on' as const, ingest: { ...newIngest('aaaaaaaaaaaaaaaa', '1'.repeat(64)), acked: 3, nextSequence: 4 } }
  const session = world(on, { config: CONFIG_ON, seed, processAnswer: () => ({ exitCode: 0, stdout: 'last_sequence=3\n', stderr: '' }) })

  await $.session.start(START)

  expect(session.book().ingest?.stream).toBe('aaaaaaaaaaaaaaaa')
  expect(session.book().ingest?.queue).toEqual([])
})

test('a question id is unique across sessions: its number in the book, then the end of the session id', () => {
  const ask = { tree: null, job: null, question: 'Which parser?', options: ['Old', 'New'], recommendation: 'New', category: 'scope', mergeFor: null } as const
  const first = questionIn({ ...emptyBook(), session: SESSION }, ask, START_MS)
  const second = questionIn(first.book, ask, START_MS)
  const other = questionIn({ ...emptyBook(), session: '3F9A1C2E-4B7D-4E21-9C3A-0D5E6F7A8B9C' }, ask, START_MS)

  expect([first.question.id, second.question.id, other.question.id]).toEqual(['q1-000000000001', 'q2-000000000001', 'q1-0d5e6f7a8b9c'])
})

test('a job id counts seconds since 2026 and ends in four random characters', () => {
  expect(jobIdOf(START_MS + 2000, 1)).toBe('0gaq820001')
  expect(jobIdOf(START_MS + 2000, 36 ** 4 + 35)).toBe('0gaq82000z')
})

test('own_spawn_takes_pending_entry_and_master_charges_no_job', () => {
  const setup = { session: SESSION, storage: 'off', jobsFromSdk: true, inert: null, claudeCode: '2.1.323', plugin: '0.8.0' } as const
  const started = sessionIn(emptyBook(), setup, START_MS, { stream: 'a'.repeat(16), secret: '1'.repeat(64) })
  const input = {
    builder: 'main',
    tree: null,
    origin: 'composer',
    request: 'Fix the parser',
    repo: ROOT,
    base: 'main',
    type: null,
    size: null,
    consequence: null,
    langs: [],
    prefs: { duration: null, budget: null, autonomy: null },
    limit: { nano: 10_000_000_000, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  const withJob = openJobIn(started, input, '0gaq82j000', START_MS).book
  const withTree = treeIn(withJob, { id: 't1', charter: 'Ship it.', charterHash: 'sha256:0', policyHash: 'sha256:1', state: 'running', capNano: 0 }, START_MS)
  const pending = pendingIn(withTree, { description: 'rex master t1', kind: 'master', tree: 't1', job: null, at: START_MS })
  const spawned = startRunIn(pending, { agentId: 'm1', toolUseId: 'toolu_plugin_1', parentAgentId: null, agentType: 'super-rex:master', description: 'rex master t1', isOwnSpawn: true }, START_MS + 1000)
  const usage = { model: 'claude-opus-5-5', input_tokens: 1000, output_tokens: 0, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 }
  const stepped = stepRunIn(spawned, 'm1', { model: 'claude-opus-5-5', effort: 'high', usage }, START_MS + 2000)

  expect(stepped.pendingSpawns).toEqual([])
  expect(stepped.runs[0]).toMatchObject({ agentId: 'm1', parent: 'main', parentSource: 'mod', tree: 't1', job: null, depth: 1 })
  expect(stepped.trees[0]).toMatchObject({ master: 'm1', ledgerNano: 4_000_000 })
  expect(stepped.jobs[0]?.spentNano).toBe(0)
})

test('a run whose grid agent several roles share takes its parent role, else the first in plan order', () => {
  const ratings = { intelligence: 'high', reliability: 'high', consequence: 'medium', verification: 'low' } as const
  const role = (name: string): RexRole => ({ role: name, ratings, gridAgent: 'model-high-effort-high', conflict: null, omsVersion: 2 })
  const plan = {
    orchestrator: 'leads',
    breakdown: 'two leads on one grid agent',
    tasks: [
      { id: 'T1', role: 'engineer', skills: [], openWhen: [] },
      { id: 'T2', role: 'reviewer', skills: [], openWhen: [] },
    ],
    roles: [role('engineer'), role('reviewer'), { ...role('worker'), gridAgent: 'model-low-effort-low' }],
    manifest: [],
  }
  const input = {
    builder: 'main',
    tree: null,
    origin: 'composer',
    request: 'Fix the parser',
    repo: ROOT,
    base: 'main',
    type: null,
    size: null,
    consequence: null,
    langs: [],
    prefs: { duration: null, budget: null, autonomy: null },
    limit: { nano: 10_000_000_000, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  const base = planJobIn(openJobIn({ ...emptyBook(), session: SESSION }, input, '0gaq82j000', START_MS).book, '0gaq82j000', plan, START_MS)
  const spawn = { toolUseId: null, description: 'agent', isOwnSpawn: false, agentType: 'super-rex:model-high-effort-high' }
  const lead = startRunIn(base, { ...spawn, agentId: 'a1', parentAgentId: null }, START_MS)
  const reviewer = { ...lead, runs: lead.runs.map(run => ({ ...run, role: 'reviewer' })) }
  const helper = startRunIn(reviewer, { ...spawn, agentId: 'a2', parentAgentId: 'a1' }, START_MS)
  const worker = startRunIn(helper, { ...spawn, agentId: 'a3', parentAgentId: 'a1', agentType: 'super-rex:model-low-effort-low' }, START_MS)

  expect(lead.runs[0]?.role).toBe('engineer')
  expect(worker.runs.map(run => [run.agentId, run.role])).toEqual([
    ['a1', 'reviewer'],
    ['a2', 'reviewer'],
    ['a3', 'worker'],
  ])
})

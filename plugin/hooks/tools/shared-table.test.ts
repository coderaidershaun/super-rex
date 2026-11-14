// Proves the shared tools table that a Rust test checks the skill text against: each tool the plugin registers with each argument, and each result key a tool answers.
import { expect, test } from 'claude-code/testing'
import type { PromptOrigin, ToolSpec } from 'claude-code'

import type { RexBook } from '../../types'
import { CONFIG_OFF, CONFIG_ON, ROOT, spawnInput, START_MS, world } from '../book/kit'
import type { ProcessAnswer } from '../book/kit'
import { sharedJson } from '../book/shared'
import { AUTHOR, plannedBook, SKILL_FILES, spawnsKept } from '../pipeline/assembly.fixture'
import { BASE_CALL, callFile, files, JOB, ok, promptsPass, resultText, scripted, seededBook } from './act-kit'
import type { Scripted } from './act-kit'
import { jsonField } from './binary'
import { openJobIn } from '../book/jobs'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const TABLE = sharedJson('fixtures/shared/tables/tools.json')
const TOOLS = jsonField(TABLE, 'tools')
const LATER = jsonField(TABLE, 'later')

function namesOf(value: unknown): readonly string[] {
  return typeof value === 'object' && value !== null ? Object.keys(value).sort() : []
}

function listOf(tool: string, field: 'arguments' | 'keys'): readonly string[] {
  const list = jsonField(jsonField(TOOLS, tool), field)
  return Array.isArray(list) ? list.map(String) : []
}

/** Every property name of a JSON schema, at any depth, sorted once each. */
function propertyNames(schema: unknown): readonly string[] {
  const names = new Set<string>()
  const visit = (node: unknown): void => {
    if (Array.isArray(node)) {
      node.forEach(visit)
      return
    }
    if (typeof node !== 'object' || node === null) {
      return
    }
    const properties: unknown = Reflect.get(node, 'properties')
    if (typeof properties === 'object' && properties !== null) {
      Object.keys(properties).forEach(name => names.add(name))
    }
    Object.values(node).forEach(visit)
  }
  visit(schema)
  return [...names].sort()
}

/** The key of each `key=value` line before the blank line that starts a skill file's text. */
function keysOf(result: unknown): readonly string[] {
  const head = resultText(result).split('\n\n')[0] ?? ''
  return head
    .split('\n')
    .filter(line => line.includes('='))
    .map(line => line.slice(0, line.indexOf('=')))
}

/** True when `listed` names `seen`: the same text, where each `<placeholder>` of `listed` stands for any text. */
function names(listed: string, seen: string): boolean {
  const parts = listed.split(/<[^>]+>/).map(part => part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
  return new RegExp(`^${parts.join('.+')}$`).test(seen)
}

/** The keys the table lists for `tool` that no answer here gave; empty when each one is real. */
function unseen(tool: string, results: readonly unknown[]): readonly string[] {
  const seen = results.flatMap(keysOf)
  return listOf(tool, 'keys').filter(listed => !seen.some(key => names(listed, key)))
}

test('the shared tools table names each tool the plugin registers, each of its arguments, and no later tool', async ($, on) => {
  const specs: ToolSpec[] = []
  on('tool.register', { name: /(?:)/ }, (_$, e) => {
    specs.push(e)
    return { value: { tool: `mcp__super-rex__${e.name}` } }
  })
  world(on)
  await $.session.start(START)

  const registered = specs.map(spec => spec.name).sort()
  expect(registered).toEqual(namesOf(TOOLS))
  expect(specs.map(spec => [spec.name, propertyNames(spec.inputSchema)])).toEqual(specs.map(spec => [spec.name, listOf(spec.name, 'arguments')]))
  expect(namesOf(LATER).filter(name => registered.includes(name))).toEqual([])
})

const RESTATED = { isAnswered: true, text: 'Goal: fix the parser. Missing: merge answer.', usage: { input_tokens: 1, output_tokens: 1, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 } } as const

function jobStart(origin: PromptOrigin) {
  return { text: '/super-rex:orchestration-builder Fix the parser', command: { name: 'super-rex:orchestration-builder', args: 'Fix the parser' }, origin, wait: false } as const
}

test('rex_job_open answers each key of the shared tools table', async ($, on) => {
  files(on, {})
  promptsPass(on)
  world(on, { config: CONFIG_OFF, processAnswer: scripted([BASE_CALL]), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(jobStart({ kind: 'composer' }))
  const opened = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', request: 'Fix the parser' })

  expect(unseen('rex_job_open', [opened])).toEqual([])
})

test('rex_status and rex_job_close answer each key of the shared tools table', async ($, on) => {
  const show = callFile(sharedJson('fixtures/shared/calls/job-show.json'))
  files(on, {})
  world(on, { config: CONFIG_ON, seed: seededBook({ storage: 'on' }), processAnswer: scripted([{ argv: show.argv, answer: ok(show.stdout) }]) })
  await $.session.start(START)

  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })
  const closed = await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', status: 'blocked', outcome: 'stopped', deliveries: [] })

  expect(unseen('rex_status', [status])).toEqual([])
  expect(unseen('rex_job_close', [closed])).toEqual([])
})

// With storage on, rex_recall answers the lines of `rex recall` unchanged; the shared call file pins those.
test('rex_recall answers each key of the shared tools table with storage off', async ($, on) => {
  files(on, {})
  world(on, { config: CONFIG_OFF, seed: seededBook() })
  await $.session.start(START)

  const recall = await $.tool.call({ tool: 'mcp__super-rex__rex_recall', job: JOB })

  expect(unseen('rex_recall', [recall])).toEqual([])
})

test('rex_job_plan answers each key of the shared tools table', async ($, on) => {
  const oms = callFile(sharedJson('fixtures/shared/calls/oms.json'))
  files(on, { 'plugin:library/rex-code-tdd/SKILL.md': '# TDD\n' })
  world(on, { config: CONFIG_OFF, seed: seededBook({ prefs: { duration: null, budget: 'high', autonomy: 'high' } }), processAnswer: scripted([{ argv: oms.argv, answer: ok(oms.stdout) }]) })
  await $.session.start(START)

  const planned = await $.tool.call({
    tool: 'mcp__super-rex__rex_job_plan',
    orchestrator: 'orchestration-builder 0.8.0',
    breakdown: 'T2 engineer',
    tasks: [{ id: 'T2', role: 'engineer', skills: ['rex-code-tdd'], open_when: [] }],
    roles: [{ role: 'engineer', intelligence: 'high', reliability: 'high', consequence: 'high', verification: 'low' }],
  })

  expect(unseen('rex_job_plan', [planned])).toEqual([])
})

const TREE_JOB = '0gaq820002'
const TREE_BUILDER = 'b1'

function gitCalls(branch: string): readonly Scripted[] {
  const git = (args: readonly string[], answer: ProcessAnswer = ok()): Scripted => ({ argv: ['git', '-C', ROOT, ...args], answer })
  return [
    git(['fetch', 'origin', 'main']),
    git(['rev-list', '--count', `origin/main..${branch}`], ok('1\n')),
    git(['merge-base', '--is-ancestor', 'origin/main', branch]),
    git(['diff', '--name-only', `origin/main...${branch}`], ok('src/a.rs\n')),
  ]
}

/** The seeded job with the person's merge answer of yes, and a job of tree t1 whose builder b1 has no merge answer yet. */
function mergeBook(): RexBook {
  const seed = seededBook()
  const own = { ...seed, jobs: seed.jobs.map(job => ({ ...job, merge: { answer: 'yes', source: 'dialog', question: null, policyHash: null, at: START_MS } as const })) }
  const input = {
    builder: TREE_BUILDER,
    tree: 't1',
    origin: 'master',
    request: 'Fix the lexer',
    repo: ROOT,
    base: 'main',
    type: 'fix',
    size: 'small',
    consequence: 'low',
    langs: ['rust'],
    prefs: { duration: null, budget: null, autonomy: null },
    limit: { nano: 10_000_000_000, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  return openJobIn(own, input, TREE_JOB, START_MS).book
}

test('rex_merge answers each key of the shared tools table, merged or waiting on a question', async ($, on) => {
  files(on, {})
  const branch = `rex/${JOB}`
  const push: readonly Scripted[] = [
    { argv: ['git', '-C', ROOT, 'push', 'origin', `${branch}:refs/heads/main`], answer: ok() },
    { argv: ['git', '-C', ROOT, 'rev-parse', branch], answer: ok('c1\n') },
  ]
  world(on, { seed: mergeBook(), processAnswer: scripted([...gitCalls(branch), ...push, ...gitCalls(`rex/${TREE_JOB}`)]) })
  await $.session.start(START)

  const merged = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })
  const waiting = await $.tool.call({ tool: 'mcp__super-rex__rex_merge', agentId: TREE_BUILDER })

  expect([resultText(merged).split('\n')[0], resultText(waiting).split('\n')[0]]).toEqual(['merged=yes', 'merged=no'])
  expect(unseen('rex_merge', [merged, waiting])).toEqual([])
})

test('rex_skill answers each key of the shared tools table, with a name and with none', async ($, on) => {
  files(on, SKILL_FILES)
  spawnsKept(on)
  world(on, { seed: plannedBook('model-high-effort-high') })
  await $.session.start(START)
  await $.agent.spawn(spawnInput({ description: 'engineer', subagentType: 'super-rex:model-high-effort-high', prompt: AUTHOR }))

  const named = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: 'rex-code-tdd' })
  const late = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', agentId: 'a1' })

  expect(unseen('rex_skill', [named, late])).toEqual([])
})

// Proves the outline: a book with a tree four levels deep and a job with no tree gives its nodes, levels and order, hangs an orphan run below its job, and prints with tree guides.
import { expect, test } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { liveBook, WRITTEN_AT } from '../live/live-book.fixture'
import { flatten, outlineOf, outlineText, pathTo } from './outline'

/** The fixture's book with one more run whose parent the book never saw. */
function withOrphan(): RexBook {
  const book = liveBook()
  const helper = book.runs.find(run => run.agentId === 'ahelper01')
  if (helper === undefined) {
    throw new Error('the fixture book lost its helper run')
  }
  return { ...book, runs: [...book.runs, { ...helper, agentId: 'aorphan01', parent: 'agone99', state: 'open', endReason: null, endedAt: null }] }
}

test('the outline holds the session, each tree, its jobs and their agents by level, in book order', () => {
  const outline = outlineOf(liveBook(), WRITTEN_AT)

  expect(flatten(outline).map(node => `${node.id} ${node.level} ${node.state}`)).toEqual([
    'session session waiting',
    'session/tree:t0geh4lh3i master waiting',
    'session/tree:t0geh4lh3i/job:0geh5fafkb builder waiting',
    'session/tree:t0geh4lh3i/job:0geh5fafkb/agent:adecision01 lead done',
    'session/tree:t0geh4lh3i/job:0geh5fafkb/agent:alead01 lead open',
    'session/tree:t0geh4lh3i/job:0geh5fafkb/agent:alead01/agent:aworker01 worker open',
    'session/tree:t0geh4lh3i/job:0geh5fafkb/agent:alead01/agent:aworker01/agent:ahelper01 helper done',
    'session/job:0gegkccde2 builder done',
    'session/job:0gegkccde2/agent:asolo01 lead done',
  ])
})

test('the run of a master and of a builder are facts of their node, not rows', () => {
  const ids = flatten(outlineOf(liveBook(), WRITTEN_AT)).map(node => node.ref)

  expect(ids).not.toContain('amaster01')
  expect(ids).not.toContain('abuilder01')
})

test('a run whose parent is not in the book hangs below its job', () => {
  const job = pathTo(outlineOf(withOrphan(), WRITTEN_AT), 'session/tree:t0geh4lh3i/job:0geh5fafkb').at(-1)

  expect(job?.children.map(node => `${node.ref} ${node.level}`)).toEqual(['adecision01 lead', 'alead01 lead', 'aorphan01 lead'])
  expect(job?.openBelow).toBe(3)
})

test('a path to a node that is gone ends at its nearest parent that exists', () => {
  const path = pathTo(outlineOf(liveBook(), WRITTEN_AT), 'session/tree:t0geh4lh3i/job:0geh5fafkb/agent:agone99/agent:a2')

  expect(path.map(node => node.ref)).toEqual(['53f1c255-4567-4e00-ab35-553d90ce69d1', 't0geh4lh3i', '0geh5fafkb'])
})

test('the outline as text, with tree guides, each agent by its role and never by its id', () => {
  expect(outlineText(outlineOf(liveBook(), WRITTEN_AT), 200)).toEqual([
    '? session 53f1c255 · storage off · 1 question waits · 1 master tree · 2 jobs, 1 open · 2 agents open, 3 done · $5.45',
    '├─ ? master t0geh4lh3i · running · 1 question waits · 0/1 jobs delivered · 2 agents open, 2 done · $3.65 of $150.00 · 7m · Build a hello world app and a goodbye world app.',
    '│  └─ ? builder 0geh5fafkb · planned · 1 question waits · 2 agents open, 2 done · $3.25 of $28.00 · 6m · Build two small command-line apps, hello and goodbye, in th…',
    '│     ├─ ✓ lead decision-maker · done (answer) · $0.70 · 1m',
    '│     └─ ● lead engineer-lead · open · 1 agent open, 1 done · $0.60 · 3m',
    '│        └─ ● worker engineer T1 · open · 1 agent done · $0.45 · 3m',
    '│           └─ ✓ helper model-low-effort-low · done (answer) · $0.10 · 41s',
    '└─ ✓ builder 0gegkccde2 · delivered · 1 agent done · $1.80 of $28.00 · 6m · please build a hello world app',
    '   └─ ✓ lead engineer T1 · done (answer) · $0.20 · 44s',
  ])
})

test('an agent is named by its role, else by its task, else by its grid agent, and by role and task when they differ', () => {
  const book = liveBook()
  const named = (role: string | null, task: string | null): string => {
    const runs = book.runs.map(run => (run.agentId === 'adecision01' ? { ...run, role, task } : run))
    const decision = flatten(outlineOf({ ...book, runs }, WRITTEN_AT)).find(node => node.ref === 'adecision01')
    return decision?.name ?? 'gone'
  }

  expect(named(null, 'decision-challenger')).toBe('decision-challenger')
  expect(named('decision-maker', 'decision-maker')).toBe('decision-maker')
  expect(named('engineer', 'T2')).toBe('engineer T2')
  expect(named(null, null)).toBe('model-high-effort-medium')
})

test('a role that is the level word is said once, as the live run of a lead T5, a worker T1 and a helper T2 showed', () => {
  const roles: { readonly [agentId: string]: { readonly role: string; readonly task: string | null } } = {
    alead01: { role: 'lead', task: 'T5' },
    aworker01: { role: 'worker', task: 'T1' },
    ahelper01: { role: 'helper', task: 'T2' },
    asolo01: { role: 'lead', task: null },
  }
  const book = liveBook()
  const runs = book.runs.map(run => ({ ...run, ...roles[run.agentId] }))
  const lines = outlineText(outlineOf({ ...book, runs }, WRITTEN_AT), 200)

  expect(lines.filter(line => / (lead|worker|helper) /.test(line)).map(line => line.replace(/^[│ ├└─]*[●✓] /, '').split(' · ')[0])).toEqual(['lead decision-maker', 'lead T5', 'worker T1', 'helper T2', 'lead'])
})

test('a question of a job waits on its tree and the session too, so every level above it shows it', () => {
  const outline = outlineOf(liveBook(), WRITTEN_AT)

  expect(pathTo(outline, 'session/tree:t0geh4lh3i/job:0geh5fafkb').map(node => node.questions.map(question => question.id))).toEqual([['q1-553d90ce69d1'], ['q1-553d90ce69d1'], ['q1-553d90ce69d1']])
})

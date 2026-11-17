// Proves the pure changes a master tree rests on: a tree opens with the main loop as master and uses up its start prompt, the main loop's steps go to its job or else its tree's ledger, a tree re-pinned to a confirmed policy keeps its master and builders, a restored tree comes back with its jobs and questions by id, and neither queues anything for the store.
import { expect, test } from 'claude-code/testing'

import { emptyBook } from './events'
import { ROOT, SESSION, START_MS } from './kit'
import { openJobIn, questionIn, restoreIn } from './jobs'
import { stepMainIn } from './runs'
import { promptIn } from './session'
import { linkIn, MAIN_MASTER, repinIn, treeIn, treeStateIn } from './trees'

function runningTree() {
  const opened = treeIn({ ...emptyBook(), session: SESSION }, { id: 't1', charter: 'Ship it.', charterHash: 'sha256:0', policyHash: 'sha256:old', state: 'paused', capNano: 0 }, START_MS)
  return linkIn(opened, 't1', '0gaq820001', 'b1', START_MS)
}

const TREE_INPUT = { id: 't1', charter: 'Ship it.', charterHash: 'sha256:0', policyHash: 'sha256:1', state: 'running', capNano: 0 } as const
const PROMPT = { text: '/super-rex:orchestration-master Ship it.', origin: 'composer', command: { name: 'super-rex:orchestration-master', args: 'Ship it.' }, restatement: null, isRecorded: true, jobStart: null } as const

test('the prompt of the master skill is kept as the tree start, and the tree it opens has the main loop as master and uses the prompt up', () => {
  const prompted = promptIn({ ...emptyBook(), session: SESSION }, { ...PROMPT, treeStart: { origin: 'composer', text: 'Ship it.' } }, START_MS)

  const opened = treeIn(prompted, TREE_INPUT, START_MS + 1000)

  expect(prompted.treeStartPrompt).toEqual({ origin: 'composer', text: 'Ship it.', at: START_MS })
  expect(opened.trees.map(tree => [tree.id, tree.session, tree.master])).toEqual([['t1', SESSION, MAIN_MASTER]])
  expect(MAIN_MASTER).toBe('main')
  expect(opened.treeStartPrompt).toBeNull()
})

test('restoreIn uses up the start prompt of a resume', () => {
  const prompted = promptIn(runningTree(), { ...PROMPT, treeStart: { origin: 'composer', text: 'resume' } }, START_MS)
  const [held] = prompted.trees
  if (held === undefined) {
    throw new Error('the book holds no tree')
  }

  expect(restoreIn(prompted, { tree: held, jobs: [], questions: [] }).treeStartPrompt).toBeNull()
})

const USAGE = { model: 'claude-opus-5-5', input_tokens: 1000, output_tokens: 0, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 }
const STEP = { model: 'claude-opus-5-5', effort: 'high', usage: USAGE } as const
const MAIN_JOB_INPUT = {
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

test("a step of the main loop goes to the ledger of the live tree it runs as master, and to no job of the tree", () => {
  const tree = linkIn(openJobIn(treeIn({ ...emptyBook(), session: SESSION }, TREE_INPUT, START_MS), { ...MAIN_JOB_INPUT, builder: 'b1', tree: 't1', origin: 'master' }, '0gaq820001', START_MS).book, 't1', '0gaq820001', 'b1', START_MS)

  const stepped = stepMainIn(tree, STEP)

  expect(stepped.trees[0]?.ledgerNano).toBe(4_000_000)
  expect(stepped.jobs[0]?.spentNano).toBe(0)
})

test("a step of the main loop goes to its own open job before any tree, and to nothing with neither, or with only a tree that is closed or of another session", () => {
  const ownJob = openJobIn(treeIn({ ...emptyBook(), session: SESSION }, TREE_INPUT, START_MS), MAIN_JOB_INPUT, '0gaq820002', START_MS).book
  const closed = treeStateIn(treeIn({ ...emptyBook(), session: SESSION }, TREE_INPUT, START_MS), 't1', 'done', null, START_MS)
  const elsewhere = { ...treeIn({ ...emptyBook(), session: SESSION }, TREE_INPUT, START_MS), session: '11111111-2222-4333-8444-555555555555' }

  const onJob = stepMainIn(ownJob, STEP)

  expect([onJob.jobs[0]?.spentNano, onJob.trees[0]?.ledgerNano]).toEqual([4_000_000, 0])
  expect(stepMainIn({ ...emptyBook(), session: SESSION }, STEP)).toEqual({ ...emptyBook(), session: SESSION })
  expect(stepMainIn(closed, STEP)).toBe(closed)
  expect(stepMainIn(elsewhere, STEP)).toBe(elsewhere)
})

test('repinIn pins the tree to the new hash and keeps its master, builders and state, with no event for the store', () => {
  const before = runningTree()

  const after = repinIn(before, 't1', 'sha256:new')

  expect(after.trees[0]).toEqual({ ...before.trees[0], policyHash: 'sha256:new' })
  expect(after.ingest).toBe(before.ingest)
  expect(repinIn(before, 't9', 'sha256:new')).toEqual(before)
})

test('restoreIn puts a tree, its jobs and its questions back by id, replacing what the book holds of one id, with no event', () => {
  const before = runningTree()
  const asked = questionIn(before, { tree: 't1', job: null, question: 'Which parser?', options: ['Old', 'New'], recommendation: 'New', category: 'scope', mergeFor: null }, START_MS).book
  const [held] = asked.trees
  const [question] = asked.questions
  if (held === undefined || question === undefined) {
    throw new Error('the book holds no tree or no question')
  }
  const stored = { ...question, question: 'Which parser, old or new?' }
  const other = { ...stored, id: 'q1-555555555555' }

  const after = restoreIn(asked, { tree: { ...held, state: 'running' }, jobs: [], questions: [stored, other] })

  expect(after.trees.map(each => [each.id, each.master, each.state])).toEqual([['t1', 'main', 'running']])
  expect(after.questions.map(each => [each.id, each.question])).toEqual([
    [stored.id, 'Which parser, old or new?'],
    ['q1-555555555555', 'Which parser, old or new?'],
  ])
  expect(after.ingest).toBe(asked.ingest)
})

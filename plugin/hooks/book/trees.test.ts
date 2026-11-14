// Proves the pure changes a master's recovery rests on: a tree re-pinned to a confirmed policy keeps its master and builders, a restored tree comes back with its jobs and questions by id, and neither queues anything for the store.
import { expect, test } from 'claude-code/testing'

import { emptyBook } from './events'
import { SESSION, START_MS } from './kit'
import { questionIn, restoreIn } from './jobs'
import { linkIn, repinIn, treeIn } from './trees'

function runningTree() {
  const opened = treeIn({ ...emptyBook(), session: SESSION }, { id: 't1', charter: 'Ship it.', charterHash: 'sha256:0', policyHash: 'sha256:old', state: 'paused', capNano: 0 }, START_MS)
  const withMaster = { ...opened, trees: opened.trees.map(tree => ({ ...tree, master: 'm1' })) }
  return linkIn(withMaster, 't1', '0gaq820001', 'b1', START_MS)
}

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

  const after = restoreIn(asked, { tree: { ...held, master: null, state: 'running' }, jobs: [], questions: [stored, other] })

  expect(after.trees.map(each => [each.id, each.master, each.state])).toEqual([['t1', null, 'running']])
  expect(after.questions.map(each => [each.id, each.question])).toEqual([
    [stored.id, 'Which parser, old or new?'],
    ['q1-555555555555', 'Which parser, old or new?'],
  ])
  expect(after.ingest).toBe(asked.ingest)
})

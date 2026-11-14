// Proves the four book changes this unit needs: a lint key kept once, a merge done, a skill opened and a run's output file, each with the event the store expects.
import { expect, test } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { emptyBook, newIngest } from '../book/events'
import { mergeDoneIn, openJobIn } from '../book/jobs'
import { lintAttemptIn, runOutputIn, skillOpenIn, startRunIn } from '../book/runs'
import { ROOT, SESSION, START_MS } from '../book/kit'

function bookWithJobAndRun(): RexBook {
  const started = { ...emptyBook(), session: SESSION, storage: 'on' as const, ingest: newIngest('aaaaaaaaaaaaaaaa', '1'.repeat(64)) }
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
  const opened = openJobIn(started, input, '0gaq820001', START_MS).book
  const run = { agentId: 'a1', toolUseId: 'toolu_1', parentAgentId: null, agentType: 'general-purpose', description: 'worker', isOwnSpawn: false }
  return startRunIn(opened, run, START_MS)
}

function kinds(book: RexBook): string[] {
  return (book.ingest?.queue ?? []).map(event => event.kind)
}

test('a lint key is kept once and a later brief of it gets the stored attempt', () => {
  const book = bookWithJobAndRun()
  const first = lintAttemptIn(book, { key: 'main|0gaq820001|T2', findings: ['Acceptance: is missing'], at: START_MS })
  const second = lintAttemptIn(first.book, { key: 'main|0gaq820001|T2', findings: [], at: START_MS + 1 })

  expect(first.stored).toBeNull()
  expect(second.stored).toEqual({ key: 'main|0gaq820001|T2', findings: ['Acceptance: is missing'], at: START_MS })
  expect(second.book.lintAttempts).toHaveLength(1)
  expect(kinds(second.book)).toEqual(kinds(book))
})

test('a merge done is a merged delivery of the job and one merge.done event', () => {
  const book = bookWithJobAndRun()
  const done = { repo: ROOT, base: 'main', branch: 'rex/0gaq820001', mergeCommit: 'c1' }
  const merged = mergeDoneIn(book, '0gaq820001', done, START_MS + 5)

  expect(merged.jobs[0]?.deliveries).toEqual([{ repo: ROOT, branch: 'rex/0gaq820001', pr: null, outcome: 'merged', mergeCommit: 'c1', note: null }])
  expect(merged.ingest?.queue.at(-1)).toMatchObject({ kind: 'merge.done', job: '0gaq820001', data: { repo: ROOT, base: 'main', merge_commit: 'c1' } })
})

test('a skill opened is kept on its run once and sent as skill.open', () => {
  const book = bookWithJobAndRun()
  const open = { skill: 'rex-code-tdd', file: 'mocking.md', hash: 'sha256:00', how: 'rex_skill' } as const
  const opened = skillOpenIn(skillOpenIn(book, 'a1', open, START_MS + 1), 'a1', open, START_MS + 2)

  expect(opened.runs[0]?.skillsOpened).toEqual(['rex-code-tdd/mocking.md'])
  expect(opened.ingest?.queue.at(-1)).toMatchObject({ kind: 'skill.open', agent: 'a1', job: '0gaq820001', data: { agent_id: 'a1', skill: 'rex-code-tdd', file: 'mocking.md', hash: 'sha256:00', how: 'rex_skill' } })
})

test('a run keeps its output file, which its end then names', () => {
  const book = runOutputIn(bookWithJobAndRun(), 'a1', `${ROOT}/.rex/plans/0gaq820001/engineer.md`)

  expect(book.runs[0]?.outputFile).toBe(`${ROOT}/.rex/plans/0gaq820001/engineer.md`)
})

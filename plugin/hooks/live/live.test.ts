// Proves the live snapshot: the fixture's book gives exactly the shared snapshot file, no secret reaches it, and the file is written on a change and on the heartbeat, never otherwise, and a failed write blocks nothing.
import { expect, test } from 'claude-code/testing'

import { ROOT, SESSION, START_MS, world } from '../book/kit'
import { sharedJson } from '../book/shared'
import { CHECK_EVERY_MS, HEARTBEAT_MS, livePath } from './index'
import { liveBook, WRITTEN_AT } from './live-book.fixture'
import { snapshotOf } from './snapshot'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const SNAPSHOT = 'fixtures/shared/live/snapshot.json'
const PATH = livePath(ROOT, SESSION)

function writtenAt(text: string | undefined): unknown {
  const parsed: unknown = JSON.parse(text ?? '{}')
  return typeof parsed === 'object' && parsed !== null ? Reflect.get(parsed, 'written_at') : undefined
}

test('the book of the fixture gives exactly the shared snapshot file', () => {
  expect(snapshotOf(liveBook(), WRITTEN_AT)).toEqual(sharedJson(SNAPSHOT))
})

test('nothing of the ingest stream or its secret is in the snapshot', () => {
  const book = liveBook()
  const text = JSON.stringify(snapshotOf(book, WRITTEN_AT))

  expect(book.ingest?.secret).toBe('9f'.repeat(32))
  expect(text).not.toContain('9f9f')
  expect(text).not.toContain(book.ingest?.stream ?? 'no stream')
  expect(Object.keys(snapshotOf(book, WRITTEN_AT))).toEqual(Object.keys(sharedJson(SNAPSHOT) as object))
})

const PERSON = { origin: { kind: 'composer' }, presentation: { isFullscreen: false, columns: 120 } } as const

test('the snapshot is written at the first look, on a change of its text, and on the heartbeat while a job is open, never otherwise', async ($, on) => {
  const session = world(on, { seed: liveBook(SESSION, START_MS - WRITTEN_AT) })
  await $.session.start(START)
  const times = (): unknown[] => session.liveWrites.map(write => writtenAt(write.text))
  const at = (ms: number): string => new Date(START_MS + ms).toISOString()

  await session.clock.advance(CHECK_EVERY_MS)
  expect(session.liveWrites.map(write => write.path)).toEqual([PATH])
  expect(times()).toEqual([at(CHECK_EVERY_MS)])

  // The prompt mode is not in the snapshot: the book changed, its text did not.
  await $.command.run({ command: 'rex', args: 'prompts improve', ...PERSON })
  await session.clock.advance(3 * CHECK_EVERY_MS)
  expect(times()).toEqual([at(CHECK_EVERY_MS)])

  await session.clock.advance(2 * CHECK_EVERY_MS)
  expect(times()).toEqual([at(CHECK_EVERY_MS), at(CHECK_EVERY_MS + HEARTBEAT_MS)])

  await $.command.run({ command: 'rex', args: 'answer q1-553d90ce69d1 Merge', ...PERSON })
  await session.clock.advance(CHECK_EVERY_MS)
  expect(times()).toEqual([at(CHECK_EVERY_MS), at(CHECK_EVERY_MS + HEARTBEAT_MS), at(2 * CHECK_EVERY_MS + HEARTBEAT_MS)])
  expect(session.liveWrites[2]?.text).toContain('"source": "command"')
})

test('a tree spends what its master and every job of it spent, against its cap', () => {
  const tree = snapshotOf(liveBook(), WRITTEN_AT)['trees']

  expect(Array.isArray(tree) ? Reflect.get(tree[0] ?? {}, 'spent_nano') : undefined).toBe(3_650_000_000)
})

test('a charter or a request of many pages is cut, so the file a reader polls stays small', () => {
  const book = liveBook()
  const long = 'x'.repeat(50_000)
  const text = JSON.stringify(snapshotOf({ ...book, trees: book.trees.map(tree => ({ ...tree, charter: long })), jobs: book.jobs.map(job => ({ ...job, request: long })) }, WRITTEN_AT))

  expect(text.length).toBeLessThan(20_000)
})

test('the heartbeat stops once the last job and tree are closed: one write, then none', async ($, on) => {
  const book = liveBook(SESSION, START_MS - WRITTEN_AT)
  const closed = { ...book, trees: book.trees.map(tree => ({ ...tree, state: 'done' as const })), jobs: book.jobs.map(job => ({ ...job, status: 'delivered' as const })) }
  const session = world(on, { seed: closed })
  await $.session.start(START)

  await session.clock.advance(3 * HEARTBEAT_MS)
  expect(session.liveWrites).toHaveLength(1)
})

test('the file goes to the real folder of a session root reached through a symbolic link', async ($, on) => {
  on('fs.stat', { path: /^\/home\/person\/repo$/ }, () => ({ value: { kind: 'dir', size: 0, mtimeMs: 0, isLink: true, realPath: '/srv/real/repo' } }))
  const session = world(on, { seed: liveBook(SESSION, START_MS - WRITTEN_AT) })
  await $.session.start(START)

  await session.clock.advance(CHECK_EVERY_MS)
  expect(session.liveWrites.map(write => write.path)).toEqual([livePath('/srv/real/repo', SESSION)])
})

test('nothing is written while the book holds no job and no tree', async ($, on) => {
  const session = world(on)
  await $.session.start(START)

  await session.clock.advance(3 * HEARTBEAT_MS)
  expect(session.liveWrites).toEqual([])
})

test('a failed write blocks nothing: it is recorded once, and the next look writes again', async ($, on) => {
  const session = world(on, { seed: liveBook(SESSION, START_MS - WRITTEN_AT) })
  await $.session.start(START)
  session.failNextLiveWrite()

  await session.clock.advance(CHECK_EVERY_MS)
  expect(session.liveWrites).toEqual([])
  expect(session.book().jobs[0]?.degraded).toEqual(['observer:clock:live'])

  const status = await $.command.run({ command: 'rex', args: 'status', ...PERSON })
  expect(status.text?.split('\n')[0]).toMatch(/^rex master t0geh4lh3i running/)

  await session.clock.advance(CHECK_EVERY_MS)
  expect(session.liveWrites.map(write => write.path)).toEqual([PATH])
})

// Writes the live snapshot to <main checkout>/.rex/live/<session id>.json in every storage mode, so `rex watch` in another terminal shows the run with no database.
// `$.fs.write` has no rename, so a reader may meet a half-written file; the reader keeps its last good read. A failed write never blocks anything.
import type { EngineInterface, On, Timer } from 'claude-code'

import { bookWrites, errorText, fail, isOpen, readBook } from '../book/index'
import type { RexBook } from '../../types'
import type { Parts } from '../parts'
import { snapshotOf } from './snapshot'

/** How often the mod looks for a change of the book. */
export const CHECK_EVERY_MS = 2_000
/** How old the file may grow while a job or a tree is open, so a reader tells a live session from one that ended. */
export const HEARTBEAT_MS = 10_000

type Written = { readonly path: string; readonly snapshot: ReturnType<typeof snapshotOf>; readonly body: string; readonly isLive: boolean }

/**
 * What this load of the module knows: the count of the book's writes at the last look, the last snapshot it wrote,
 * and the looks since. A look that finds no new write of the book and no heartbeat due calls nothing.
 * A reload starts it over and writes again at its first look.
 */
const writing: { timer: Timer | null; lastVersion: number | null; last: Written | null; quietLooks: number; isFailing: boolean } = {
  timer: null,
  lastVersion: null,
  last: null,
  quietLooks: 0,
  isFailing: false,
}

/** A paused or blocked tree waits to go on, so it keeps the heartbeat as a running one does. */
function isLive(book: RexBook): boolean {
  return book.jobs.some(isOpen) || book.trees.some(tree => tree.state === 'running' || tree.state === 'paused' || tree.state === 'blocked')
}

export function livePath(root: string, session: string): string {
  return `${root}/.rex/live/${session}.json`
}

async function realRoot($: EngineInterface): Promise<string> {
  const root = await $.session.root()
  return (await $.fs.stat(root, { resolve: true })).realPath ?? root
}

/** The snapshot of the book as it stands, or null when the book holds no job and no tree. */
async function current($: EngineInterface): Promise<Written | null> {
  const book = await readBook($)
  if (book.inert !== null || (book.jobs.length === 0 && book.trees.length === 0)) {
    return null
  }
  const snapshot = snapshotOf(book, 0)
  const path = writing.last?.path ?? livePath(await realRoot($), book.session)
  return { path, snapshot, body: JSON.stringify(snapshot), isLive: isLive(book) }
}

/** Writes when the snapshot changed, or when the last one is a heartbeat old while a job or a tree is open. */
async function writeIfDue($: EngineInterface): Promise<void> {
  const version = bookWrites()
  const isChanged = version !== writing.lastVersion
  writing.quietLooks += 1
  const isHeartbeat = writing.last?.isLive === true && writing.quietLooks * CHECK_EVERY_MS >= HEARTBEAT_MS
  if (!isChanged && !isHeartbeat) {
    return
  }
  const next = isChanged ? await current($) : writing.last
  writing.lastVersion = version
  if (next === null || (next.body === writing.last?.body && !isHeartbeat)) {
    return
  }
  const writtenAt = new Date(await $.clock.now()).toISOString()
  await $.fs.write(next.path, `${JSON.stringify({ ...next.snapshot, written_at: writtenAt }, null, 2)}\n`)
  writing.last = next
  writing.quietLooks = 0
  writing.isFailing = false
}

/** A failure is recorded once until a write succeeds again, so a folder that stays unwritable marks the job once, not at every look. */
async function look($: EngineInterface): Promise<void> {
  try {
    await writeIfDue($)
  } catch (error) {
    writing.lastVersion = null
    if (!writing.isFailing) {
      writing.isFailing = true
      await fail($, { kind: 'observer-failed', event: 'clock', module: 'live', agentId: null, error: errorText(error) })
    }
  }
}

export function register(on: On, _parts: Parts): void {
  on('session.start', { cwd: /(?:)/ }, async ($, e, next) => {
    writing.timer?.cancel()
    writing.timer = null
    writing.lastVersion = null
    writing.last = null
    writing.quietLooks = 0
    writing.isFailing = false
    const book = await readBook($)
    if (book.inert === null) {
      writing.timer = $.clock.every(CHECK_EVERY_MS, () => {
        void look($).catch((error: unknown) => {
          $.ui.status(`rex: the live snapshot could not record its own failure: ${errorText(error)}`)
        })
      })
    }
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'session.start', module: 'live', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('session.end', { reason: /(?:)/ }, async ($, e, next) => {
    writing.timer?.cancel()
    writing.timer = null
    await look($)
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'session.end', module: 'live', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })
}

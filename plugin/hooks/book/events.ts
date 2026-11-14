// The empty book, the queue of events for the binary, and the stream fields: pure changes that every other change of the book goes through.
import type { RexBook, RexEvent, RexIngest, RexJob, RexJson, RexTokens } from '../../types'

export const NO_TOKENS: RexTokens = { input: 0, output: 0, cacheRead: 0, cacheCreation: 0 }

/** The binary refuses a batch above these. */
export const BATCH_MAX_EVENTS = 500
export const BATCH_MAX_BYTES = 1024 * 1024
/** Past this many waiting events, new ones are dropped and counted. */
export const QUEUE_LIMIT = 10_000

/**
 * `acked` is -1 until the binary first answered this book: then the events
 * queued so far are numbered again after the last sequence it holds.
 */
export const NOT_ACKED = -1

export type EventData = { readonly [key: string]: RexJson }

export function emptyBook(): RexBook {
  return {
    session: '',
    versions: { mod: 'unknown', binary: null, claudeCode: null },
    permissionAsks: 0,
    inert: null,
    storage: 'off',
    prompts: 'improve',
    jobsFromSdk: true,
    jobStartPrompt: null,
    jobs: [],
    runs: [],
    lintAttempts: [],
    questions: [],
    trees: [],
    pendingSpawns: [],
    ingest: null,
  }
}

export function newIngest(stream: string, secret: string): RexIngest {
  return { stream, secret, nextSequence: 1, acked: NOT_ACKED, failures: 0, resets: 0, halted: null, dropped: 0, queue: [] }
}

/** The store's form of a time: UTC, milliseconds, `Z`. */
export function utcText(ms: number): string {
  return new Date(ms).toISOString()
}

export function isOpen(job: RexJob): boolean {
  return job.status === 'created' || job.status === 'planned'
}

export function openJobs(book: RexBook): readonly RexJob[] {
  return book.jobs.filter(job => isOpen(job) && job.owner.session === book.session)
}

export function findJob(book: RexBook, id: string): RexJob | undefined {
  return book.jobs.find(job => job.id === id)
}

export function replaceJob(book: RexBook, id: string, change: (job: RexJob) => RexJob): RexBook {
  return { ...book, jobs: book.jobs.map(job => (job.id === id ? change(job) : job)) }
}

/** Adds one event for the binary; with storage off it changes nothing. */
export function queued(book: RexBook, at: number, kind: string, agent: string, job: string | null, data: EventData): RexBook {
  const ingest = book.ingest
  if (book.storage === 'off' || ingest === null) {
    return book
  }
  if (ingest.queue.length >= QUEUE_LIMIT) {
    return droppedEvent(book, ingest, at)
  }
  const event: RexEvent = { seq: ingest.nextSequence, at: utcText(at), kind, agent, job, data }
  return { ...book, ingest: { ...ingest, nextSequence: ingest.nextSequence + 1, queue: [...ingest.queue, event] } }
}

function droppedEvent(book: RexBook, ingest: RexIngest, at: number): RexBook {
  const dropped = ingest.dropped + 1
  const mark = `ingest:dropped:${String(dropped)}`
  const jobs = book.jobs.map(job =>
    isOpen(job) ? { ...job, degraded: [...job.degraded.filter(old => !old.startsWith('ingest:dropped:')), mark], updatedAt: at } : job,
  )
  return { ...book, jobs, ingest: { ...ingest, dropped } }
}

/** Adds a mark to a job once, with its event. */
export function withMark(book: RexBook, jobId: string, mark: string, at: number): RexBook {
  const job = findJob(book, jobId)
  if (job === undefined || job.degraded.includes(mark)) {
    return book
  }
  const marked = replaceJob(book, jobId, old => ({ ...old, degraded: [...old.degraded, mark], updatedAt: at }))
  return queued(marked, at, 'job.degraded', job.owner.builder, jobId, { mark })
}

export function withMarkOnOpenJobs(book: RexBook, mark: string, at: number): RexBook {
  return openJobs(book).reduce((changed, job) => withMark(changed, job.id, mark, at), book)
}

/** The waiting events that fit in one batch, oldest first. */
export function nextBatch(book: RexBook, max: number): readonly RexEvent[] {
  const queue = book.ingest?.queue ?? []
  const batch: RexEvent[] = []
  let bytes = 0
  for (const event of queue.slice(0, Math.min(max, BATCH_MAX_EVENTS))) {
    bytes += JSON.stringify(event).length
    if (batch.length > 0 && bytes > BATCH_MAX_BYTES) {
      break
    }
    batch.push(event)
  }
  return batch
}

/** The binary holds every event up to `lastSequence`: those leave the queue. */
export function acked(book: RexBook, lastSequence: number): RexBook {
  const ingest = book.ingest
  if (ingest === null) {
    return book
  }
  const settled = { ...ingest, failures: 0, resets: 0 }
  if (ingest.acked === NOT_ACKED) {
    const queue = ingest.queue.map((event, index) => ({ ...event, seq: lastSequence + 1 + index }))
    return { ...book, ingest: { ...settled, acked: lastSequence, queue, nextSequence: lastSequence + 1 + queue.length } }
  }
  const queue = ingest.queue.filter(event => event.seq > lastSequence)
  const nextSequence = Math.max(ingest.nextSequence, lastSequence + 1)
  return { ...book, ingest: { ...settled, acked: Math.max(ingest.acked, lastSequence), queue, nextSequence } }
}

/**
 * Puts the session's transcript path into its waiting `session.start` event.
 * SMELL: a sent event changed and sent again is a conflict that stops ingest.
 * Only this one event is changed, and only before the first event batch has
 * been answered; the classic SessionStart that gives the path comes at start.
 */
export function withTranscript(book: RexBook, path: string): RexBook {
  const ingest = book.ingest
  if (ingest === null || ingest.acked > 0) {
    return book
  }
  const queue = ingest.queue.map(event =>
    event.kind === 'session.start' && !('transcript_path' in event.data)
      ? { ...event, data: { ...event.data, transcript_path: path } }
      : event,
  )
  return { ...book, ingest: { ...ingest, queue } }
}

export function tokensPlus(sum: RexTokens, add: RexTokens): RexTokens {
  return {
    input: sum.input + add.input,
    output: sum.output + add.output,
    cacheRead: sum.cacheRead + add.cacheRead,
    cacheCreation: sum.cacheCreation + add.cacheCreation,
  }
}

export function tokensData(tokens: RexTokens): EventData {
  return { input: tokens.input, output: tokens.output, cache_read: tokens.cacheRead, cache_creation: tokens.cacheCreation }
}

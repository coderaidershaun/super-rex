// Sends the live record's events to the binary in batches, so a session with storage keeps its record after it ends.
import type { EngineInterface, On, Timer } from 'claude-code'

import { ackEvents, assertNever, errorText, fail, isOpen, nextEvents, readBook } from '../book/index'
import type { RexBook } from '../../types'
import type { Parts } from '../parts'
import { batchText, INGEST_ARGV, INGEST_TIMEOUT_MS, outcomeOf, RETRY_DELAYS_MS, RETRY_JITTER_MS } from './batch'
import type { IngestOutcome } from './batch'

const FLUSH_EVERY_MS = 10_000
/** Events that are sent at once: a run's end, and a job's change of status or merge answer. */
const SEND_AT_ONCE = new Set(['run.end', 'job.open', 'job.plan', 'job.close', 'job.merge_answer'])

/**
 * What this load of the module is doing with the binary. A reload starts it
 * over and opens the stream again, which the binary allows.
 */
const sending: {
  openedStream: string | null
  running: Promise<void> | null
  isAgainWanted: boolean
  /** Set at session end: no timer is left to send later, so nothing waits for one. */
  isEnding: boolean
  retry: Timer | null
  every: Timer | null
  check: Timer | null
  isInteractive: boolean
} = { openedStream: null, running: null, isAgainWanted: false, isEnding: false, retry: null, every: null, check: null, isInteractive: true }

async function send($: EngineInterface, text: string): Promise<IngestOutcome> {
  try {
    const result = await $.process.run(INGEST_ARGV, { cwd: await $.session.root(), stdin: text, timeoutMs: INGEST_TIMEOUT_MS })
    return outcomeOf(result)
  } catch (error) {
    return { kind: 'failed', reason: error instanceof Error ? error.message : String(error) }
  }
}

function canSend(book: RexBook): boolean {
  return book.inert === null && book.storage === 'on' && book.ingest !== null && book.ingest.halted === null
}

function flushLater($: EngineInterface, ms: number): void {
  if (sending.isEnding) {
    return
  }
  const jitter = crypto.getRandomValues(new Uint32Array(1))[0] ?? 0
  sending.retry = $.clock.after(ms + (jitter % (RETRY_JITTER_MS + 1)), () => {
    sending.retry = null
    void flush($).catch((error: unknown) => {
      reportTimerError($, error)
    })
  })
}

/** Acts on a failed call by its class; answers true while sending may go on now. */
async function onFailed($: EngineInterface, outcome: Exclude<IngestOutcome, { kind: 'stored' }>): Promise<boolean> {
  const before = await readBook($)
  switch (outcome.kind) {
    case 'conflict':
      await fail($, { kind: 'ingest-conflict', seq: outcome.seq, stderr: outcome.stderr })
      return false
    case 'no-store':
      await fail($, { kind: 'ingest-no-store', stderr: outcome.stderr })
      return false
    case 'wrong-secret':
      await fail($, { kind: 'ingest-wrong-secret', stream: before.ingest?.stream ?? '', stderr: outcome.stderr })
      return canSend(await readBook($))
    case 'failed': {
      await fail($, { kind: 'ingest-failed', reason: outcome.reason })
      const after = await readBook($)
      if (!canSend(after) || after.ingest === null) {
        return false
      }
      const isRenewed = after.ingest.stream !== before.ingest?.stream
      const delay = isRenewed ? RETRY_DELAYS_MS[2] : RETRY_DELAYS_MS[after.ingest.failures - 1]
      flushLater($, delay ?? FLUSH_EVERY_MS)
      return false
    }
    default:
      return assertNever(outcome)
  }
}

/** Sends what waits; with `isOpenOnly`, only the opening batch of a stream not yet opened in this load. */
async function sendAll($: EngineInterface, isOpenOnly: boolean): Promise<void> {
  for (;;) {
    const batch = await nextEvents($, 500)
    if (batch === null || !canSend(await readBook($))) {
      return
    }
    const isOpening = sending.openedStream !== batch.stream
    if (!isOpening && (isOpenOnly || batch.events.length === 0)) {
      return
    }
    const outcome = await send($, batchText(isOpening ? { ...batch, events: [] } : batch))
    if (outcome.kind !== 'stored') {
      if (!(await onFailed($, outcome))) {
        return
      }
      continue
    }
    sending.openedStream = batch.stream
    await ackEvents($, outcome.lastSequence)
    if (isOpenOnly) {
      return
    }
  }
}

/**
 * One flush at a time. A flush asked for while one runs makes that one send
 * again when it ends, and waits for it; while a retry waits, the retry sends.
 */
function flush($: EngineInterface, isOpenOnly = false): Promise<void> {
  if (sending.running !== null) {
    sending.isAgainWanted = true
    return sending.running
  }
  if (sending.retry !== null) {
    return Promise.resolve()
  }
  sending.running = flushNow($, isOpenOnly).finally(() => {
    sending.running = null
  })
  return sending.running
}

async function flushNow($: EngineInterface, isOpenOnly: boolean): Promise<void> {
  await sendAll($, isOpenOnly)
  while (sending.isAgainWanted && sending.retry === null) {
    sending.isAgainWanted = false
    await sendAll($, false)
  }
  sending.isAgainWanted = false
}

/**
 * Sends at once when a run ended or a job changed status or merge answer.
 * The check runs from a timer after the whole dispatch, so it also sees what
 * a module above this one recorded after the event passed down.
 */
function flushSoon($: EngineInterface): void {
  if (sending.check !== null) {
    return
  }
  sending.check = $.clock.after(0, () => {
    sending.check = null
    void flushIfUrgent($).catch((error: unknown) => {
      reportTimerError($, error)
    })
  })
}

async function flushIfUrgent($: EngineInterface): Promise<void> {
  const queue = (await readBook($)).ingest?.queue ?? []
  if (queue.some(event => SEND_AT_ONCE.has(event.kind))) {
    await flush($)
  }
}

function startFlushing($: EngineInterface): void {
  sending.every?.cancel()
  sending.every = $.clock.every(FLUSH_EVERY_MS, () => {
    void flush($).catch((error: unknown) => {
      reportTimerError($, error)
    })
  })
}

/** At session end no timer is left to send later, so the last flush sends everything queued, a failed batch first. */
function stopFlushing(): void {
  sending.isEnding = true
  sending.retry?.cancel()
  sending.every?.cancel()
  sending.check?.cancel()
  sending.retry = null
  sending.every = null
  sending.check = null
}

function reportTimerError($: EngineInterface, error: unknown): void {
  void fail($, { kind: 'observer-failed', event: 'clock', module: 'ingest', agentId: null, error: errorText(error) }).catch((again: unknown) => {
    $.ui.status(`rex: ingest could not record its own failure: ${errorText(again)}`)
  })
}

/** At the end of a headless session nobody can answer a parked question, so its job is blocked before the last flush. */
async function blockUnanswered($: EngineInterface): Promise<void> {
  const book = await readBook($)
  for (const question of book.questions) {
    const jobId = question.mergeFor ?? question.job
    const job = book.jobs.find(each => each.id === jobId)
    if (question.answer === null && job !== undefined && isOpen(job)) {
      await fail($, { kind: 'question-unanswered-headless', job: job.id, question: question.id })
    }
  }
}

export function register(on: On, _parts: Parts): void {
  on('session.start', { cwd: /(?:)/ }, async ($, e, next) => {
    sending.isInteractive = e.isInteractive
    sending.isEnding = false
    const book = await readBook($)
    if (book.inert === null && book.storage === 'on') {
      await flush($, true)
      startFlushing($)
    }
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'session.start', module: 'ingest', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('session.end', async ($, e, next) => {
    stopFlushing()
    if (!sending.isInteractive) {
      await blockUnanswered($)
    }
    await flush($)
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'session.end', module: 'ingest', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('turn.complete', { reason: /(?:)/ }, async ($, e, next) => {
    const result = await next(e)
    flushSoon($)
    return result
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'turn.complete', module: 'ingest', agentId: e.agentId ?? null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('tool.call', { tool: /(?:)/ }, async ($, e, next) => {
    const result = await next(e)
    flushSoon($)
    return result
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'tool.call', module: 'ingest', agentId: e.agentId ?? null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('prompt.submit', { text: /(?:)/ }, async ($, e, next) => {
    const result = await next(e)
    flushSoon($)
    return result
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'prompt.submit', module: 'ingest', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })
}

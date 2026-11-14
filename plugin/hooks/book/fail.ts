// The book's part of each failure: marks, the halt of ingest or a tree, a blocked job, and the one status line the person reads.
import type { RexBook } from '../../types'
import { findJob, isOpen, openJobs, withMark, withMarkOnOpenJobs } from './events'
import type { RexFailure } from './failure'
import { assertNever } from './failure'
import type { StreamKeys } from './ids'
import { closeJobIn } from './jobs'
import { findRun } from './runs'
import { treeStateIn } from './trees'

/** Three failed flushes in a row renew the stream; two failed renewals stop ingest. */
export const FLUSH_TRIES = 3
export const RESET_TRIES = 2

export type Failed = { readonly book: RexBook; readonly status: string | null }

export function failIn(book: RexBook, failure: RexFailure, at: number, keys: StreamKeys): Failed {
  switch (failure.kind) {
    case 'policy-changed':
      return {
        book: treeStateIn(book, failure.tree, 'paused', null, at),
        status: `rex: policy changed since master ${failure.tree} started; /rex policy confirm, then /rex master resume`,
      }
    case 'ingest-conflict': {
      const seq = failure.seq === null ? 'unknown' : String(failure.seq)
      return {
        book: halted(withMarkOnOpenJobs(book, `ingest:conflict:${seq}`, at), `conflict:${seq}`),
        status: `rex: ingest stopped: conflict at seq ${seq}; run rex doctor`,
      }
    }
    case 'ingest-no-store': {
      const line = firstLine(failure.stderr)
      return {
        book: halted(withMarkOnOpenJobs(book, 'ingest:no-store', at), 'no-store'),
        status: line.includes('rex init --with-storage') ? `rex: ingest stopped: ${line}` : `rex: ingest stopped: ${line}; run rex init --with-storage`,
      }
    }
    case 'config-unreadable':
      return { book: { ...book, inert: failure.reason }, status: `rex: .rex/config.json unreadable: ${failure.reason}` }
    case 'merge-outside-rex':
      return {
        book: blocked(withMark(book, failure.job, 'merge:outside-rex_merge', at), failure.job, `commit ${failure.commit} reached ${failure.base} outside rex_merge`, at),
        status: `rex: job ${failure.job} blocked: commit ${failure.commit} reached ${failure.base} outside rex_merge`,
      }
    case 'ingest-wrong-secret':
      return renewed(book, keys, at)
    case 'ingest-halted':
      return haltedIngest(book, at)
    case 'ingest-failed':
      return flushFailed(book, failure.reason, keys, at)
    case 'builder-stuck':
    case 'builder-api-error':
      return failure.isLimit
        ? { book: blocked(book, failure.job, `builder ${failure.agentId} did not recover`, at), status: `rex: job ${failure.job} blocked: builder ${failure.agentId} did not recover` }
        : { book, status: null }
    case 'observer-failed':
      return { book: observerMarked(book, failure.agentId, `observer:${failure.event}:${failure.module}`, at), status: null }
    case 'restatement-failed':
      return { book, status: `rex: restatement skipped (${failure.reason})` }
    case 'gate-check-failed':
      return { book: failure.job === null ? book : withMark(book, failure.job, `gate:${failure.check}:failed`, at), status: null }
    case 'question-unanswered-headless':
      return {
        book: blocked(book, failure.job, `question ${failure.question} was not answered before the headless session ended`, at),
        status: `rex: job ${failure.job} blocked: question ${failure.question} unanswered`,
      }
    case 'migrate-unknown-version':
    case 'builder-dead-on-resume':
    case 'merge-answer-missing':
    case 'authority-check-failed':
    case 'origin-not-person':
    case 'caller-not-allowed':
    case 'bad-argument':
    case 'spend-limit-reached':
    case 'depth':
    case 'fork':
    case 'tree-ledger':
    case 'rate-limit-pause':
    case 'lint-findings':
    case 'merge-push-rejected':
    case 'merge-lock-held':
    case 'merge-not-fast-forward':
    case 'merge-path-asks':
    case 'binary-read-failed':
    case 'preflight-failed':
    case 'policy-missing':
      return { book, status: null }
    default:
      return assertNever(failure)
  }
}

function firstLine(text: string): string {
  return text.split('\n')[0]?.replace(/^ingest refused: /, '') ?? ''
}

function halted(book: RexBook, reason: string): RexBook {
  return book.ingest === null ? book : { ...book, ingest: { ...book.ingest, halted: reason } }
}

function haltedIngest(book: RexBook, at: number): Failed {
  return { book: halted(withMarkOnOpenJobs(book, 'ingest:halted', at), 'halted'), status: 'rex: ingest stopped; run rex doctor' }
}

function blocked(book: RexBook, jobId: string, outcome: string, at: number): RexBook {
  const job = findJob(book, jobId)
  return job === undefined || !isOpen(job) ? book : closeJobIn(book, jobId, { status: 'blocked', outcome, deliveries: job.deliveries }, at)
}

/** A new stream and secret; the old secret is never sent again. */
function renewed(book: RexBook, keys: StreamKeys, at: number): Failed {
  const ingest = book.ingest
  if (ingest === null) {
    return { book, status: null }
  }
  if (ingest.resets >= RESET_TRIES) {
    return haltedIngest(book, at)
  }
  return { book: { ...book, ingest: { ...ingest, stream: keys.stream, secret: keys.secret, failures: 0, resets: ingest.resets + 1 } }, status: null }
}

function flushFailed(book: RexBook, reason: string, keys: StreamKeys, at: number): Failed {
  const ingest = book.ingest
  if (ingest === null) {
    return { book, status: null }
  }
  if (ingest.resets > 0) {
    return renewed(book, keys, at)
  }
  const failures = ingest.failures + 1
  const counted = { ...book, ingest: { ...ingest, failures } }
  if (failures < FLUSH_TRIES) {
    return { book: counted, status: null }
  }
  return renewed(withMarkOnOpenJobs(counted, `ingest:${reason}`, at), keys, at)
}

/** The mark goes to the job of the event's run, else to the session's newest open job. */
function observerMarked(book: RexBook, agentId: string | null, mark: string, at: number): RexBook {
  const runJob = agentId === null ? null : (findRun(book, agentId)?.job ?? null)
  const newest = [...openJobs(book)].sort((a, b) => b.createdAt - a.createdAt)[0]
  const jobId = runJob ?? newest?.id ?? null
  return jobId === null ? book : withMark(book, jobId, mark, at)
}

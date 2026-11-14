// What one call of `rex ingest` sends and what its answer means, kept apart from the timing of the calls.
import type { ProcessRunResult } from 'claude-code'

import type { Batch } from '../book/index'

export const INGEST_ARGV: readonly string[] = ['rex', 'ingest']
export const INGEST_TIMEOUT_MS = 30_000
/** The waits after the first, second and third failed flush in a row; the third ends in a new stream. */
export const RETRY_DELAYS_MS: readonly number[] = [10_000, 20_000, 40_000]
export const RETRY_JITTER_MS = 2_000

/** The batch as the binary reads it on stdin. */
export function batchText(batch: Batch): string {
  return JSON.stringify({ version: 1, session: batch.session, stream: batch.stream, secret: batch.secret, events: batch.events })
}

export type IngestOutcome =
  | { readonly kind: 'stored'; readonly lastSequence: number }
  | { readonly kind: 'conflict'; readonly seq: number | null; readonly stderr: string }
  | { readonly kind: 'wrong-secret'; readonly stderr: string }
  | { readonly kind: 'no-store'; readonly stderr: string }
  | { readonly kind: 'failed'; readonly reason: string }

const LAST_SEQUENCE = /^last_sequence=(\d+)\n?$/
const CONFLICT_SEQ = /\bseq (\d+)/

/** Reads the binary's answer by exit code: 2 conflict or bad batch, 3 wrong secret, 4 no store. */
export function outcomeOf(result: ProcessRunResult): IngestOutcome {
  switch (result.exitCode) {
    case 0: {
      const found = LAST_SEQUENCE.exec(result.stdout)?.[1]
      const lastSequence = found === undefined ? Number.NaN : Number(found)
      return Number.isSafeInteger(lastSequence)
        ? { kind: 'stored', lastSequence }
        : { kind: 'failed', reason: `unreadable answer ${JSON.stringify(result.stdout.slice(0, 80))}` }
    }
    case 2: {
      const seq = CONFLICT_SEQ.exec(result.stderr)?.[1]
      return { kind: 'conflict', seq: seq === undefined ? null : Number(seq), stderr: result.stderr }
    }
    case 3:
      return { kind: 'wrong-secret', stderr: result.stderr }
    case 4:
      return { kind: 'no-store', stderr: result.stderr }
    default:
      return { kind: 'failed', reason: String(result.exitCode) }
  }
}

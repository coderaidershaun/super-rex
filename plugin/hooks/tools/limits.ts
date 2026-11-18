// The spend limit a job gets when no person and no policy set one, one high amount for every job, and the read of a limit a person set at a shell.
import type { EngineInterface } from 'claude-code'

import type { RexBook, RexJob } from '../../types'
import { MAX_LIMIT_USD } from '../book/index'

/** Nanodollars: $100,000. Equal to the shared limits table, which a test checks. */
export const DEFAULT_LIMIT_NANO = 100_000_000_000_000

export function defaultLimitNano(): number {
  return DEFAULT_LIMIT_NANO
}

/** What `rex limit <job> --json` prints, or null when the binary fails or prints no JSON: a limit that cannot be read is no higher limit. */
async function storedLimit($: EngineInterface, job: string): Promise<unknown> {
  try {
    const read = await $.process.run(['rex', 'limit', job, '--json'], { cwd: await $.session.root() })
    const stored: unknown = read.exitCode === 0 ? JSON.parse(read.stdout) : null
    return stored
  } catch (error) {
    if (error instanceof Error) {
      return null
    }
    throw error
  }
}

/**
 * With storage on, a person may have raised a job's limit at a shell with
 * `rex limit <job> --set`. The limit they set there, in nanodollars, when it
 * is higher than the book's and one `/rex limit` would take; else null.
 */
export async function higherLimitInStore($: EngineInterface, book: RexBook, job: RexJob): Promise<number | null> {
  if (book.storage !== 'on') {
    return null
  }
  const stored = await storedLimit($, job.id)
  const usd: unknown = typeof stored === 'object' && stored !== null ? Reflect.get(stored, 'limit_usd') : undefined
  const source: unknown = typeof stored === 'object' && stored !== null ? Reflect.get(stored, 'source') : undefined
  const isInRange = typeof usd === 'number' && Number.isFinite(usd) && usd >= 0 && usd <= MAX_LIMIT_USD
  const nano = isInRange ? Math.round(usd * 1e9) : 0
  return source === 'set' && nano > job.limit.nano ? nano : null
}

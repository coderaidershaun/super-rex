// Reads a merge answer and a spend limit out of the person's own words; these are authority, so they run whatever the prompt mode.

const MERGE_ANSWER = /\bmerge(?: to main)?\s*[:=]\s*(yes|no)\b/i
const SPEND_LIMIT = /\bspend limit(?: of this job)? (?:to|=|:)\s*\$?(\d+(?:\.\d+)?)\s*(?:dollars|usd)?/i

/** Nanodollars stay safe integers up to this many dollars. */
export const MAX_LIMIT_USD = 9_000_000

export type SpendLimitWords =
  | { readonly kind: 'none' }
  | { readonly kind: 'amount'; readonly nano: number }
  | { readonly kind: 'ignored'; readonly text: string }

export type PersonWords = {
  readonly merge: 'yes' | 'no' | null
  readonly limit: SpendLimitWords
}

export function readPersonWords(text: string): PersonWords {
  const merge = MERGE_ANSWER.exec(text)?.[1]?.toLowerCase()
  const amount = SPEND_LIMIT.exec(text)?.[1]
  return {
    merge: merge === 'yes' || merge === 'no' ? merge : null,
    limit: amount === undefined ? { kind: 'none' } : limitOf(amount),
  }
}

function limitOf(amount: string): SpendLimitWords {
  const usd = Number(amount)
  if (Number.isNaN(usd) || usd < 0 || usd > MAX_LIMIT_USD) {
    return { kind: 'ignored', text: amount }
  }
  return { kind: 'amount', nano: Math.round(usd * 1e9) }
}

export function limitIgnoredLine(text: string): string {
  return `rex: limit ${text} ignored: not an amount from 0 to ${String(MAX_LIMIT_USD)} dollars`
}

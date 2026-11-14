// The spend limit a job gets when no person set one: the agents its size allows, plus the main session, times the dollars of one agent at its budget.
import type { RexJob } from '../../types'

/** Nanodollars, by size and stated budget; `none` is a job that stated no budget. Equal to the shared limits table, which a test checks. */
export const DEFAULT_LIMIT_NANO = {
  small: { low: 10_500_000_000, medium: 28_000_000_000, high: 56_000_000_000, none: 28_000_000_000 },
  medium: { low: 12_000_000_000, medium: 32_000_000_000, high: 64_000_000_000, none: 32_000_000_000 },
  large: { low: 18_000_000_000, medium: 48_000_000_000, high: 96_000_000_000, none: 48_000_000_000 },
} as const

/** A job with no size counts as medium, the middle of the caps, as the binary counts it. */
export function defaultLimitNano(size: RexJob['size'], budget: RexJob['prefs']['budget']): number {
  return DEFAULT_LIMIT_NANO[size ?? 'medium'][budget ?? 'none']
}

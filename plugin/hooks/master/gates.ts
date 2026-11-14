// The gate checks of rex_build as pure decisions over the book and the policy: spend of the tree, the plan's rate limit, and the counts and consequence the policy allows.
import type { SessionRateLimit } from 'claude-code'

import type { RexBook, RexJob, RexLevel, RexTree } from '../../types'
import { isOpen } from '../book/index'
import { usd } from '../tools/index'
import { levelRank, parallelLimit } from './policy'
import type { Policy } from './policy'

export function treeJobs(book: RexBook, tree: string): readonly RexJob[] {
  return book.jobs.filter(job => job.tree === tree)
}

/** What the tree has spent: the master's own requests and every job's spend. */
export function treeSpentNano(book: RexBook, tree: RexTree): number {
  return treeJobs(book, tree.id).reduce((sum, job) => sum + job.spentNano, tree.ledgerNano)
}

export function treeLedgerRefusal(book: RexBook, tree: RexTree): string | null {
  const spent = treeSpentNano(book, tree)
  return spent >= tree.capNano ? `rex gate: master tree ${tree.id} has spent $${usd(spent)} of its $${usd(tree.capNano)}` : null
}

export function rateLimitRefusal(limits: readonly SessionRateLimit[], policy: Policy): string | null {
  const over = limits.find(limit => limit.percentUsed >= policy.pause_at_rate_limit_pct)
  return over === undefined ? null : `rex gate: plan usage at ${String(over.percentUsed)}% (${over.kind}); new builders wait until it falls`
}

export type NewJob = { readonly consequence: RexLevel; readonly size: RexJob['size'] }

export function maxJobsRefusal(book: RexBook, tree: RexTree, policy: Policy, job: NewJob): string | null {
  if (levelRank(job.consequence) > levelRank(policy.consequence_ceiling)) {
    return `rex gate: consequence_ceiling of the policy, got ${job.consequence}`
  }
  const jobs = treeJobs(book, tree.id).filter(each => each.status !== 'cancelled')
  if (jobs.length >= policy.max_jobs) {
    return `rex gate: max_jobs of the policy, got ${String(jobs.length)}`
  }
  const open = jobs.filter(isOpen).length
  return open >= parallelLimit(policy, job.size) ? `rex gate: max_parallel_builders of the policy, got ${String(open)} open jobs` : null
}

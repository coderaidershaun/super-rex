// The one wake each tree waits on after the rate limit refused a build, so a master that ended its turn with no builder running still goes on.
import type { EngineInterface, Timer } from 'claude-code'

import type { RexTree } from '../../types'
import type { Policy } from './policy'
import { wakeMain } from './wake'

const MINUTE_MS = 60_000

/** A reload clears it: a build refused after one sets the wake again. */
const rateWakes = new Map<string, Timer>()

export function rateWakeText(minutes: number): string {
  return `rex: plan usage was over the pause point ${String(minutes)} minutes ago. Call rex_build again for the steps that wait.`
}

/** One wake for each tree, `retry.then_minutes` after rex_build was refused for the rate limit. */
export function wakeAfterRateLimit($: EngineInterface, tree: RexTree, policy: Policy): void {
  if (rateWakes.has(tree.id)) {
    return
  }
  const minutes = policy.retry.then_minutes
  const timer = $.clock.after(minutes * MINUTE_MS, () => {
    rateWakes.delete(tree.id)
    wakeMain($, rateWakeText(minutes))
  })
  rateWakes.set(tree.id, timer)
}

/** The tree closed: nothing more wakes its master. */
export function forgetTree(treeId: string): void {
  rateWakes.get(treeId)?.cancel()
  rateWakes.delete(treeId)
}

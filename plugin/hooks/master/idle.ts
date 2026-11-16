// Keeps count of the wakes that keep a master from stopping with work to do: the idle wakes in a row of each tree, and the one wake each tree waits on after the rate limit refused a build.
import type { EngineInterface, Timer } from 'claude-code'

import type { RexTree } from '../../types'
import type { Policy } from './policy'
import { wakeLater } from './wake'

const MINUTE_MS = 60_000

/** A reload clears both: a master woken after one starts its count again. */
const idleWakes = new Map<string, number>()
const rateWakes = new Map<string, Timer>()

/** The master did something that moves its tree, so its next stop starts the count again. */
export function masterMoved(treeId: string): void {
  idleWakes.delete(treeId)
}

export function idleWakesOf(treeId: string): number {
  return idleWakes.get(treeId) ?? 0
}

export function countIdleWake(treeId: string): void {
  idleWakes.set(treeId, idleWakesOf(treeId) + 1)
}

export function rateWakeText(minutes: number): string {
  return `rex: plan usage was over the pause point ${String(minutes)} minutes ago. Call rex_build again for the steps that wait.`
}

export function hasRateWake(treeId: string): boolean {
  return rateWakes.has(treeId)
}

/** One wake for each tree, `retry.then_minutes` after rex_build was refused for the rate limit. */
export function wakeAfterRateLimit($: EngineInterface, tree: RexTree, policy: Policy): void {
  const master = tree.master
  if (master === null || rateWakes.has(tree.id)) {
    return
  }
  const minutes = policy.retry.then_minutes
  const timer = $.clock.after(minutes * MINUTE_MS, () => {
    rateWakes.delete(tree.id)
    wakeLater($, master, rateWakeText(minutes))
  })
  rateWakes.set(tree.id, timer)
}

/** The tree closed: nothing more wakes its master. */
export function forgetTree(treeId: string): void {
  rateWakes.get(treeId)?.cancel()
  rateWakes.delete(treeId)
  idleWakes.delete(treeId)
}

// Keeps a tree moving with no person watching: a builder whose spend stays flat or whose run died on an API error is woken on a schedule, and the master hears each builder's end.
import type { EngineInterface, Timer } from 'claude-code'

import type { RexBook, RexJob, RexTree } from '../../types'
import { addQuestion, countWake, errorText, fail, isOpen, readBook, tickTree } from '../book/index'
import type { StatusSection, Table } from '../parts'
import { statusLines } from '../tools/index'
import { runDoneCheck } from './done'
import { treeJobs } from './gates'
import { DEFAULT_POLICY } from './policy'
import type { Policy } from './policy'
import { readPolicy } from './store'
import { raiseAtLimit } from './limits'
import { CONTINUE_TEXT, wakeLater } from './wake'

const MINUTE_MS = 60_000
/** Two wakes that brought no spend mean waking again will not help. */
const STUCK_WAKES = 2

/** The watchdog of each live tree and the retry each job waits on; a reload cancels both, and session start sets the watchdogs again. */
const watchdogs = new Map<string, Timer>()
const retries = new Map<string, Timer>()

async function policyOrDefault($: EngineInterface): Promise<Policy> {
  try {
    return (await readPolicy($))?.policy ?? DEFAULT_POLICY
  } catch (error) {
    $.ui.status(`rex: the policy could not be read for the watchdog, so it runs on the defaults: ${errorText(error)}`)
    return DEFAULT_POLICY
  }
}

function isLive(tree: RexTree | undefined): tree is RexTree {
  return tree !== undefined && (tree.state === 'running' || tree.state === 'paused')
}

function builderOf(tree: RexTree, job: string): string | undefined {
  return tree.builders.find(builder => builder.job === job)?.agentId
}

/** The ticks at the end of the job's record that all show the same spend. */
function flatTicks(tree: RexTree, job: string): number {
  const ticks = tree.ticks.filter(tick => tick.job === job)
  const last = ticks.at(-1)
  if (last === undefined) {
    return 0
  }
  let count = 0
  for (let index = ticks.length - 1; index >= 0 && ticks[index]?.spentNano === last.spentNano; index -= 1) {
    count += 1
  }
  return count
}

/** A job no wake helped waits on the person, and its builder is not woken again. */
async function blockAndAsk($: EngineInterface, tree: RexTree, job: RexJob, why: string): Promise<void> {
  await addQuestion($, {
    tree: tree.id,
    job: job.id,
    question: `Job ${job.id} is blocked: ${why}. What should the master do?`,
    options: ['Build it again', 'Leave it blocked'],
    recommendation: 'Leave it blocked',
    category: 'other',
    mergeFor: null,
  })
}

/** The status digest a master is woken with: what rex_status would tell it now. */
function digest(book: RexBook, tree: RexTree, sections: Table<StatusSection>): string {
  return statusLines(book, null, { kind: 'master', agentId: tree.master, job: null }, sections).join('\n')
}

export function wakeMaster($: EngineInterface, book: RexBook, tree: RexTree, why: string, sections: Table<StatusSection>): void {
  if (tree.master !== null) {
    wakeLater($, tree.master, `rex: ${why}. The tree now:\n${digest(book, tree, sections)}`)
  }
}

async function tickJob($: EngineInterface, tree: RexTree, job: RexJob, policy: Policy): Promise<void> {
  const builder = builderOf(tree, job.id)
  if (builder === undefined) {
    return
  }
  await tickTree($, tree.id, { job: job.id, spentNano: job.spentNano })
  const book = await readBook($)
  const ticked = book.trees.find(each => each.id === tree.id) ?? tree
  const run = book.runs.find(each => each.agentId === builder)
  const isStuck = run?.state === 'open' && flatTicks(ticked, job.id) >= policy.stuck_ticks * (job.stuckWakes + 1)
  if (!isStuck) {
    return
  }
  const isLimit = job.stuckWakes >= STUCK_WAKES
  await fail($, { kind: 'builder-stuck', job: job.id, agentId: builder, wakes: job.stuckWakes, isLimit })
  if (isLimit) {
    await blockAndAsk($, tree, job, `its builder ${builder} made no progress after ${String(STUCK_WAKES)} wakes`)
    return
  }
  await countWake($, job.id, 'stuck')
  wakeLater($, builder, CONTINUE_TEXT(job.id))
}

export async function watchdogTick($: EngineInterface, treeId: string, sections: Table<StatusSection>): Promise<void> {
  const book = await readBook($)
  const tree = book.trees.find(each => each.id === treeId)
  if (!isLive(tree)) {
    stopWatchdog(treeId)
    return
  }
  const policy = await policyOrDefault($)
  const open = treeJobs(book, tree.id).filter(isOpen)
  for (const job of open) {
    await tickJob($, tree, job, policy)
    await raiseAtLimit($, await readBook($), tree, job.id, policy)
  }
  const master = book.runs.find(run => run.agentId === tree.master)
  if (master?.state === 'done' && open.length > 0) {
    wakeMaster($, await readBook($), tree, 'the watchdog found open jobs while you were stopped', sections)
  }
}

export function stopWatchdog(treeId: string): void {
  watchdogs.get(treeId)?.cancel()
  watchdogs.delete(treeId)
}

/** The tick of each tree that runs now: a tick starts only after the one before it ended, so two ticks never read and write one job's record at once. */
const ticking = new Map<string, Promise<void>>()

export function startWatchdog($: EngineInterface, treeId: string, minutes: number, sections: Table<StatusSection>): void {
  stopWatchdog(treeId)
  const timer = $.clock.every(minutes * MINUTE_MS, () => {
    const before = ticking.get(treeId) ?? Promise.resolve()
    const tick = before.then(() => watchdogTick($, treeId, sections)).catch((error: unknown) => {
      void fail($, { kind: 'observer-failed', event: 'clock.every', module: 'master', agentId: null, error: errorText(error) }).catch((again: unknown) => {
        $.ui.status(`rex: the watchdog of tree ${treeId} failed and could not record it: ${errorText(again)}`)
      })
    })
    ticking.set(treeId, tick)
  })
  watchdogs.set(treeId, timer)
}

/** The wait before the next try: the first tries come soon, the later ones further apart. */
export function retryDelayMinutes(policy: Policy, tryNumber: number): number {
  return tryNumber <= policy.retry.first_tries ? policy.retry.first_minutes : policy.retry.then_minutes
}

/** A builder whose run ended on an API error is woken on the policy's schedule, once for each error, until its last try blocks the job. */
async function retryBuilder($: EngineInterface, tree: RexTree, job: RexJob, builder: string): Promise<void> {
  if (retries.has(job.id)) {
    return
  }
  const policy = await policyOrDefault($)
  const tryNumber = job.retries + 1
  const isLimit = tryNumber >= policy.retry.max_tries
  await fail($, { kind: 'builder-api-error', job: job.id, agentId: builder, tries: job.retries, isLimit })
  if (isLimit) {
    await blockAndAsk($, tree, job, `its builder ${builder} ended on an API error ${String(tryNumber)} times`)
    return
  }
  await countWake($, job.id, 'retry')
  const timer = $.clock.after(retryDelayMinutes(policy, tryNumber) * MINUTE_MS, () => {
    retries.delete(job.id)
    wakeLater($, builder, CONTINUE_TEXT(job.id))
  })
  retries.set(job.id, timer)
}

/** The done check of a delivered job, then the master hears of it; a check that fails is recorded and the master still hears. */
async function afterDelivery($: EngineInterface, job: RexJob, tree: RexTree, sections: Table<StatusSection>): Promise<void> {
  try {
    await runDoneCheck($, job, await policyOrDefault($))
  } catch (error) {
    await fail($, { kind: 'observer-failed', event: 'clock.after', module: 'master', agentId: null, error: errorText(error) })
  }
  wakeMaster($, await readBook($), tree, `job ${job.id} was delivered and its done check ran`, sections)
}

/** A builder's run ended: retry it after an API error, else check a delivered job and tell the master. */
export async function builderEnded($: EngineInterface, agentId: string, reason: string, sections: Table<StatusSection>): Promise<void> {
  const book = await readBook($)
  const tree = book.trees.find(each => isLive(each) && each.builders.some(builder => builder.agentId === agentId))
  const jobId = tree?.builders.find(builder => builder.agentId === agentId)?.job
  const job = book.jobs.find(each => each.id === jobId)
  if (tree === undefined || job === undefined) {
    return
  }
  if (reason === 'error' && isOpen(job)) {
    await retryBuilder($, tree, job, agentId)
    return
  }
  if (job.status === 'delivered' && job.doneChecks.length === 0) {
    $.clock.after(0, () => {
      void afterDelivery($, job, tree, sections).catch((error: unknown) => {
        $.ui.status(`rex: the done check of job ${job.id} failed and could not be recorded: ${errorText(error)}`)
      })
    })
    return
  }
  wakeMaster($, book, tree, `the builder of job ${job.id} ended its turn (${reason})`, sections)
}

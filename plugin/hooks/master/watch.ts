// Keeps a tree moving with no person watching: a builder whose job's spend stays flat, whoever holds a run open in it and sooner when nothing does, or whose run died on an API error is woken on a schedule, and the master hears each builder that stopped with nothing of its own running.
import type { EngineInterface, Timer } from 'claude-code'

import type { RexBook, RexJob, RexTree, RexTreeTick } from '../../types'
import { bookWrites, countWake, errorText, fail, isOpen, readBook, tickTree } from '../book/index'
import type { StatusSection, Table } from '../parts'
import { parkQuestion } from './ask'
import { runDoneCheck } from './done'
import { treeJobs } from './gates'
import { DEFAULT_POLICY } from './policy'
import type { Policy } from './policy'
import { readPolicy } from './store'
import { raiseAtLimit } from './limits'
import { CONTINUE_TEXT, wakeLater, wakeMaster } from './wake'

const MINUTE_MS = 60_000
/** Two wakes that brought no spend mean waking again will not help. */
const STUCK_WAKES = 2
/**
 * The quiet ticks after which an idle job's builder is woken, against
 * `stuck_ticks` for a builder at work. A builder at work can sit in one long
 * tool call, but a job with no run open and no question waiting moves only
 * when it is woken. Two ticks, not one, so that a builder whose agent ended
 * just before a tick has a whole period to be resumed by the engine first.
 */
const IDLE_TICKS = 2

/** The watchdog of each live tree and the retry each job waits on; a reload cancels both, and session start sets the watchdogs again. */
const watchdogs = new Map<string, Timer>()
const retries = new Map<string, Timer>()

/** The last reason the policy could not be read, so a tick every few minutes shows each new one once and not again. */
let policyFailure: string | null = null

async function policyOrDefault($: EngineInterface): Promise<Policy> {
  try {
    const policy = (await readPolicy($))?.policy ?? DEFAULT_POLICY
    policyFailure = null
    return policy
  } catch (error) {
    const why = errorText(error)
    if (why !== policyFailure) {
      $.ui.status(`rex: the policy could not be read for the watchdog, so it runs on the defaults: ${why}`)
    }
    policyFailure = why
    return DEFAULT_POLICY
  }
}

function isLive(tree: RexTree | undefined): tree is RexTree {
  return tree !== undefined && (tree.state === 'running' || tree.state === 'paused')
}

function builderOf(tree: RexTree, job: string): string | undefined {
  return tree.builders.find(builder => builder.job === job)?.agentId
}

/**
 * The ticks since the first tick that saw the job's spend as it is now. They
 * come from the time, so a quiet tick writes nothing; rounding absorbs the few
 * milliseconds a timer runs late.
 */
function quietTicks(tick: RexTreeTick, now: number, policy: Policy): number {
  return Math.round((now - tick.at) / (policy.watchdog_minutes * MINUTE_MS))
}

/** A job no wake helped waits on the person, and its builder is not woken again. */
async function blockAndAsk($: EngineInterface, tree: RexTree, job: RexJob, why: string): Promise<void> {
  await parkQuestion($, {
    tree: tree.id,
    job: job.id,
    question: `Job ${job.id} is blocked: ${why}. What should the master do?`,
    options: ['Build it again', 'Leave it blocked'],
    recommendation: 'Leave it blocked',
    category: 'other',
    mergeFor: null,
  })
}

/** True while an agent that the builder or one of its leads started, at any depth, still runs: the builder waits for it, and the engine resumes the builder when it ends. */
function hasOpenAgent(book: RexBook, job: string, builder: string): boolean {
  return book.runs.some(run => run.job === job && run.agentId !== builder && run.state === 'open')
}

/**
 * The quiet ticks after which a job is stuck, or null while it waits on the
 * person or on the retry of an API error, which wakes its builder on its own
 * schedule. What the watchdog measures is the job's spend, and a live agent
 * spends: a job with any run open, the builder's or one below it whose end
 * never reached the book, is stuck after `stuck_ticks`. A job with nothing
 * open moves only when it is woken, so it is stuck after `IDLE_TICKS`.
 */
function stuckPeriod(book: RexBook, job: string, policy: Policy): { readonly ticks: number; readonly isRunning: boolean } | null {
  const isAsking = book.questions.some(question => question.job === job && question.answer === null)
  if (isAsking || retries.has(job)) {
    return null
  }
  const isRunning = book.runs.some(run => run.job === job && run.state === 'open')
  return { ticks: isRunning ? policy.stuck_ticks : IDLE_TICKS, isRunning }
}

/** What one tick did to a job: the first wake of a stopped builder is the only one its stopped master hears of. */
type Ticked = { readonly kind: 'none' } | { readonly kind: 'woke-stopped-first'; readonly minutes: number; readonly isAgentOpen: boolean }

const NONE: Ticked = { kind: 'none' }

/**
 * A stuck builder is one whose job's spend stayed flat for the job's stuck
 * period. A wake stamps the job's tick again, so each next step, the second
 * wake and then the blocked job, waits one more such period.
 */
async function tickJob($: EngineInterface, book: RexBook, tree: RexTree, job: RexJob, policy: Policy, now: number): Promise<Ticked> {
  const builder = builderOf(tree, job.id)
  if (builder === undefined) {
    return NONE
  }
  const seen = tree.ticks.find(tick => tick.job === job.id)
  if (seen?.spentNano !== job.spentNano) {
    await tickTree($, tree.id, { job: job.id, spentNano: job.spentNano })
    return NONE
  }
  const isWorking = book.runs.find(run => run.agentId === builder)?.state === 'open'
  const period = stuckPeriod(book, job.id, policy)
  const quiet = quietTicks(seen, now, policy)
  if (period === null || quiet < period.ticks) {
    return NONE
  }
  const isLimit = job.stuckWakes >= STUCK_WAKES
  await fail($, { kind: 'builder-stuck', job: job.id, agentId: builder, wakes: job.stuckWakes, isLimit })
  if (isLimit) {
    await blockAndAsk($, tree, job, `its builder ${builder} made no progress after ${String(STUCK_WAKES)} wakes`)
    return NONE
  }
  await countWake($, job.id, 'stuck')
  await tickTree($, tree.id, { job: job.id, spentNano: job.spentNano })
  wakeLater($, builder, CONTINUE_TEXT(job.id))
  return !isWorking && job.stuckWakes === 0 ? { kind: 'woke-stopped-first', minutes: quiet * policy.watchdog_minutes, isAgentOpen: period.isRunning } : NONE
}

/** Why the watchdog woke the builder of a stopped master's job. */
function stoppedWakeText(job: string, ticked: Extract<Ticked, { kind: 'woke-stopped-first' }>): string {
  const held = ticked.isAgentOpen ? 'an agent open' : 'no agent running'
  return `the watchdog found job ${job} with ${held} and no spend for ${String(ticked.minutes)} minutes while you were stopped; the mod woke its builder`
}

/** The book as it is now, read again only after a write of this load of the mod, so a quiet tick reads it once. */
function bookReader($: EngineInterface): () => Promise<RexBook> {
  let held: { readonly book: RexBook; readonly writes: number } | null = null
  return async () => {
    if (held === null || held.writes !== bookWrites()) {
      const writes = bookWrites()
      held = { book: await readBook($), writes }
    }
    return held.book
  }
}

/** Ticks each open job; a stopped master hears only of the first wake of a stopped builder, never of builders at work. A quiet tick writes nothing. */
export async function watchdogTick($: EngineInterface, treeId: string, sections: Table<StatusSection>): Promise<void> {
  const current = bookReader($)
  const book = await current()
  const tree = book.trees.find(each => each.id === treeId)
  if (!isLive(tree)) {
    stopWatchdog(treeId)
    return
  }
  const open = treeJobs(book, tree.id).filter(isOpen)
  if (open.length === 0) {
    return
  }
  const policy = await policyOrDefault($)
  const now = await $.clock.now()
  const master = book.runs.find(run => run.agentId === tree.master)
  for (const job of open) {
    const ticked = await tickJob($, await current(), tree, job, policy, now)
    await raiseAtLimit($, await current(), tree, job.id, policy)
    if (ticked.kind === 'woke-stopped-first' && master?.state === 'done') {
      wakeMaster($, await current(), tree, stoppedWakeText(job.id, ticked), sections)
    }
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

/** The end of a builder's last message that its master is woken with: room for the state and the open items it gives. */
const LAST_MESSAGE_CHARS = 1_500

/** The end of one turn of an agent, as `turn.complete` gives it. */
export type TurnEnd = { readonly agentId: string; readonly reason: string; readonly answer: string }

/** A builder's turn ended: retry it after an API error, check a delivered job, leave a builder that waits for its own agents alone, else tell the master. */
export async function builderEnded($: EngineInterface, end: TurnEnd, sections: Table<StatusSection>): Promise<void> {
  const { agentId, reason } = end
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
  const lastMessage = end.answer.slice(-LAST_MESSAGE_CHARS)
  if (!isOpen(job)) {
    wakeMaster($, book, tree, `the builder of job ${job.id} closed it ${job.status} and ended its turn (${reason})`, sections, lastMessage)
    return
  }
  if (hasOpenAgent(book, job.id, agentId)) {
    return
  }
  wakeMaster($, book, tree, `the builder of job ${job.id} ended its turn (${reason}) with no agent of its own running`, sections, lastMessage)
}

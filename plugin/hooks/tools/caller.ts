// Who called a mod tool, read from the engine's agent id and the book, never from the call's arguments.
import type { RexBook, RexJob } from '../../types'
import { isOpen } from '../book/index'
import type { Caller, CallerKind, ToolEntry } from '../parts'
import { seenText } from './args'

function ownOpenJob(book: RexBook, builder: string): RexJob | undefined {
  return book.jobs.find(job => isOpen(job) && job.owner.session === book.session && job.owner.builder === builder)
}

/** `agentId` absent is the main loop, which is a builder while it owns an open job. */
export function callerIn(book: RexBook, agentId: string | undefined): Caller {
  if (agentId === undefined) {
    const job = ownOpenJob(book, 'main')
    return job === undefined ? { kind: 'main', agentId: null, job: null } : { kind: 'builder', agentId: null, job }
  }
  const job = ownOpenJob(book, agentId)
  if (job !== undefined) {
    return { kind: 'builder', agentId, job }
  }
  const isMaster = book.trees.some(tree => tree.master === agentId && (tree.state === 'running' || tree.state === 'paused'))
  if (isMaster) {
    return { kind: 'master', agentId, job: null }
  }
  const runJob = book.runs.find(run => run.agentId === agentId)?.job ?? null
  return { kind: 'agent', agentId, job: runJob === null ? null : (book.jobs.find(each => each.id === runJob) ?? null) }
}

/**
 * The newest job this caller owns, closed ones included, so a call that is
 * safe to repeat still finds the job after it closed.
 */
export function ownJobAnyStatus(book: RexBook, agentId: string | undefined): RexJob | undefined {
  const builder = agentId ?? 'main'
  const owned = book.jobs.filter(job => job.owner.session === book.session && job.owner.builder === builder)
  return owned.find(isOpen) ?? [...owned].sort((a, b) => b.updatedAt - a.updatedAt)[0]
}

const WHO: { readonly [kind in CallerKind]: string } = { main: 'the main loop', builder: 'a builder', master: 'the master', agent: 'an agent' }

/** Null when the entry lets this caller call it, else the refusal the model reads. */
export function callerRefusal(entry: ToolEntry, caller: Caller): string | null {
  if (entry.callers.includes(caller.kind)) {
    return null
  }
  const who = entry.callers.map(kind => WHO[kind]).join(' or ')
  return `${entry.name}: only ${who} may call it, got ${caller.kind}`
}

/** A `job` argument must name the caller's own job; null when it does, or when none is given. */
export function jobArgRefusal(tool: string, given: unknown, own: RexJob): string | null {
  if (given === undefined || given === null || given === own.id) {
    return null
  }
  return `${tool}: job ${seenText(given)} is not your job, got ${seenText(given)}; call it for ${own.id}`
}

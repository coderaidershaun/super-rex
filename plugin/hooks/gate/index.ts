// Refuses an agent start or a shell command that a gate check refuses: spend over the job's limit, a master tree at its budget or its time, depth outside a master tree, a fork in a job, an agent of a job that would not start as its sub-agent, a merge around rex_merge. An inert mod refuses nothing.
import type { AgentSpawnInput, AgentSpawnResult, EngineInterface, HookFailure, On } from 'claude-code'

import type { RexBook, RexJob } from '../../types'
import { fail, markDegraded, newestOpenJob, readBook, setLimit } from '../book/index'
import type { GateCheck, Parts } from '../parts'
import { treeAtLimit } from '../master/budget'
import { treeSpentNano } from '../master/gates'
import { treeLimitRefusal } from '../master/tree-limits'
import { failureName, higherLimitInStore } from '../tools/index'
import { agentArgRefusal, agentKindRefusal, depthVerdict, forkRefusal, isMergeCommand, isOverLimit, isOwnSpawn, jobOfLoop, refusedAgentArg, spendRefusal, tripwireRefusal } from './checks'
import { LIMIT_SUBCOMMAND, serveLimit } from './limit'

export { isMergeCommand, isOwnSpawn, jobOfLoop, refusedAgentArg } from './checks'

export const GATE_CHECKS: readonly GateCheck[] = [
  { name: 'spend', site: { kind: 'agent', at: 'both' }, failure: 'open' },
  { name: 'depth', site: { kind: 'agent', at: 'agent.spawn' }, failure: 'closed' },
  { name: 'fork', site: { kind: 'agent', at: 'agent.spawn' }, failure: 'closed' },
  { name: 'job-agent', site: { kind: 'agent', at: 'both' }, failure: 'closed' },
  { name: 'merge-tripwire', site: { kind: 'tool', tool: 'Bash' }, failure: 'closed' },
]

/** A job of a master tree starts no agent while its tree stands at its budget or its time; the mod asks the person at once, and tells the master. */
async function treeAtLimitRefusal($: EngineInterface, book: RexBook, job: RexJob): Promise<string | null> {
  const tree = book.trees.find(each => each.id === job.tree && each.session === book.session && (each.state === 'running' || each.state === 'paused'))
  const refusal = tree === undefined ? null : treeLimitRefusal(book, tree, await $.clock.now())
  if (tree !== undefined && refusal !== null) {
    await fail($, { kind: 'tree-ledger', tree: tree.id, spentNano: treeSpentNano(book, tree), capNano: tree.capNano })
    await treeAtLimit($, tree.id, true)
  }
  return refusal
}

/** A person may have raised the limit at a shell; a higher limit they set there is taken before refusing. */
async function overLimitAfterReread($: EngineInterface, agentId: string | undefined): Promise<string | null> {
  const book = await readBook($)
  const job = jobOfLoop(book, agentId)
  if (book.inert !== null || job === undefined) {
    return null
  }
  const tree = await treeAtLimitRefusal($, book, job)
  if (tree !== null) {
    return tree
  }
  if (!isOverLimit(job)) {
    return null
  }
  const nano = await higherLimitInStore($, book, job)
  if (nano !== null) {
    await setLimit($, job.id, { nano, source: 'set' })
    if (job.spentNano < nano) {
      return null
    }
  }
  await fail($, { kind: 'spend-limit-reached', job: job.id, spentNano: job.spentNano, limitNano: job.limit.nano })
  return spendRefusal((await readBook($)).jobs.find(each => each.id === job.id) ?? job)
}

async function spendAtSpawn($: EngineInterface, event: AgentSpawnInput): Promise<string | null> {
  return overLimitAfterReread($, event.parentAgentId)
}

async function depthAtSpawn($: EngineInterface, event: AgentSpawnInput): Promise<string | null> {
  if (event.resume !== undefined) {
    return null
  }
  const book = await readBook($)
  if (book.inert !== null) {
    return null
  }
  const verdict = depthVerdict(book, event.parentAgentId)
  if (verdict.depth === null) {
    const newest = newestOpenJob(book)
    if (newest !== undefined) {
      await markDegraded($, newest.id, 'gate:depth:unknown-parent')
    }
    return null
  }
  if (verdict.refusal !== null) {
    await fail($, { kind: 'depth', depth: verdict.depth })
  }
  return verdict.refusal
}

async function forkAtSpawn($: EngineInterface, event: AgentSpawnInput): Promise<string | null> {
  if (event.resume !== undefined || !event.fork) {
    return null
  }
  const book = await readBook($)
  const job = jobOfLoop(book, event.parentAgentId)
  if (book.inert !== null || job === undefined) {
    return null
  }
  await fail($, { kind: 'fork', job: job.id })
  return forkRefusal(book, job, event.parentAgentId)
}

/** The book is read only for a call that gives a refused argument, so every other Agent call costs no read. */
async function jobAgentAtCall($: EngineInterface, call: object, agentId: string | undefined): Promise<string | null> {
  const seen = refusedAgentArg(call)
  if (seen === undefined) {
    return null
  }
  const book = await readBook($)
  if (book.inert !== null || jobOfLoop(book, agentId) === undefined) {
    return null
  }
  await fail($, { kind: 'bad-argument', tool: 'Agent', argument: seen.argument, value: seen.value })
  return agentArgRefusal(seen)
}

/** A teammate or a workflow's agent can start with no refused argument at the Agent call, or from another tool, so the spawn is checked too. */
async function jobAgentAtSpawn($: EngineInterface, event: AgentSpawnInput, origin: string | undefined): Promise<string | null> {
  const refusal = event.resume === undefined ? agentKindRefusal(event) : null
  if (refusal === null) {
    return null
  }
  const book = await readBook($)
  if (book.inert !== null || isOwnSpawn(book, event, origin) || jobOfLoop(book, event.parentAgentId) === undefined) {
    return null
  }
  await fail($, { kind: 'bad-argument', tool: 'Agent', argument: event.isTeammate === true ? 'teammate' : 'workflow', value: event.description })
  return refusal
}

async function tripwireAtBash($: EngineInterface, command: string, agentId: string | undefined): Promise<string | null> {
  const book = await readBook($)
  if (book.inert !== null) {
    return null
  }
  const job = jobOfLoop(book, agentId)
  const base = job?.base ?? 'main'
  return isMergeCommand(command, base, job?.branch ?? null) ? tripwireRefusal(command, base) : null
}

/** An open check that fails refuses nothing: the agent starts, and its job is marked. */
async function openFailed($: EngineInterface, check: string, agentId: string | undefined, error: HookFailure): Promise<void> {
  const job = jobOfLoop(await readBook($), agentId)
  await fail($, { kind: 'gate-check-failed', check, job: job?.id ?? null, error: error.message ?? error.kind })
}

/** A closed check that fails refuses, as authority must. */
async function closedFailed($: EngineInterface, check: string, error: HookFailure): Promise<string> {
  await fail($, { kind: 'authority-check-failed', check, error: error.message ?? error.kind })
  return `rex gate: ${check} failed (${failureName(error)}); refused`
}

/**
 * The same refusal for an error the check's own code threw. In the test kit a
 * registration's catch did not refuse a spawn, so a closed check catches its
 * own errors and refuses in its body; its catch stays for a timeout.
 */
async function thrownRefusal($: EngineInterface, check: string, error: unknown): Promise<string> {
  const name = error instanceof Error ? error.name : 'Error'
  const message = error instanceof Error ? error.message : String(error)
  await fail($, { kind: 'authority-check-failed', check, error: `${name}: ${message}` })
  return `rex gate: ${check} failed (${name}); refused`
}

async function depthGuard($: EngineInterface, event: AgentSpawnInput, origin: string | undefined): Promise<string | null> {
  try {
    if (isOwnSpawn(await readBook($), event, origin)) {
      return null
    }
    return await depthAtSpawn($, event)
  } catch (error) {
    return thrownRefusal($, 'depth', error)
  }
}

async function forkGuard($: EngineInterface, event: AgentSpawnInput, origin: string | undefined): Promise<string | null> {
  try {
    if (isOwnSpawn(await readBook($), event, origin)) {
      return null
    }
    return await forkAtSpawn($, event)
  } catch (error) {
    return thrownRefusal($, 'fork', error)
  }
}

async function jobAgentCallGuard($: EngineInterface, call: object, agentId: string | undefined): Promise<string | null> {
  try {
    return await jobAgentAtCall($, call, agentId)
  } catch (error) {
    return thrownRefusal($, 'job-agent', error)
  }
}

async function jobAgentSpawnGuard($: EngineInterface, event: AgentSpawnInput, origin: string | undefined): Promise<string | null> {
  try {
    return await jobAgentAtSpawn($, event, origin)
  } catch (error) {
    return thrownRefusal($, 'job-agent', error)
  }
}

async function tripwireGuard($: EngineInterface, command: string, agentId: string | undefined): Promise<string | null> {
  try {
    return await tripwireAtBash($, command, agentId)
  } catch (error) {
    return thrownRefusal($, 'merge-tripwire', error)
  }
}

function spawnDeny(refusal: string | null): AgentSpawnResult | null {
  return refusal === null ? null : { deny: refusal }
}

export function register(on: On, parts: Parts): void {
  for (const check of GATE_CHECKS) {
    parts.gateChecks.add(check)
  }
  parts.subcommands.add(LIMIT_SUBCOMMAND)

  on('tool.call', { tool: 'Agent' }, async ($, e, next) => {
    if (next.origin.plugin === 'super-rex' && e.agentId === undefined) {
      return next(e)
    }
    const refusal = await jobAgentCallGuard($, e, e.agentId)
    return refusal === null ? next(e) : { deny: refusal }
  }).catch(async ($, e, next) => (next.called ? next(e) : { deny: await closedFailed($, 'job-agent', next.error) }))

  on('agent.spawn', { subagentType: /(?:)/ }, async ($, e, next) => {
    const refusal = await jobAgentSpawnGuard($, e, next.origin.plugin)
    return spawnDeny(refusal) ?? next(e)
  }).catch(async ($, e, next) => (next.called ? next(e) : { deny: await closedFailed($, 'job-agent', next.error) }))

  on('tool.call', { tool: 'Agent' }, async ($, e, next) => {
    if (next.origin.plugin === 'super-rex' && e.agentId === undefined) {
      return next(e)
    }
    const refusal = await overLimitAfterReread($, e.agentId)
    return refusal === null ? next(e) : { deny: refusal }
  }).catch(async ($, e, next) => {
    if (!next.called) {
      await openFailed($, 'spend', e.agentId, next.error)
    }
    return next(e)
  })

  on('agent.spawn', { subagentType: /(?:)/ }, async ($, e, next) => {
    const refusal = isOwnSpawn(await readBook($), e, next.origin.plugin) ? null : await spendAtSpawn($, e)
    return spawnDeny(refusal) ?? next(e)
  }).catch(async ($, e, next) => {
    if (!next.called) {
      await openFailed($, 'spend', e.parentAgentId, next.error)
    }
    return next(e)
  })

  on('agent.spawn', { subagentType: /(?:)/ }, async ($, e, next) => {
    const refusal = await depthGuard($, e, next.origin.plugin)
    return spawnDeny(refusal) ?? next(e)
  }).catch(async ($, e, next) => (next.called ? next(e) : { deny: await closedFailed($, 'depth', next.error) }))

  on('agent.spawn', { subagentType: /(?:)/ }, async ($, e, next) => {
    const refusal = await forkGuard($, e, next.origin.plugin)
    return spawnDeny(refusal) ?? next(e)
  }).catch(async ($, e, next) => (next.called ? next(e) : { deny: await closedFailed($, 'fork', next.error) }))

  on('tool.call', { tool: 'Bash' }, async ($, e, next) => {
    const refusal = await tripwireGuard($, e.command, e.agentId)
    return refusal === null ? next(e) : { deny: refusal }
  }).catch(async ($, e, next) => (next.called ? next(e) : { deny: await closedFailed($, 'merge-tripwire', next.error) }))

  // Monitor runs a shell command too, so the same tripwire stands in front of it.
  on('tool.call', { tool: 'Monitor' }, async ($, e, next) => {
    const refusal = e.command === undefined ? null : await tripwireGuard($, e.command, e.agentId)
    return refusal === null ? next(e) : { deny: refusal }
  }).catch(async ($, e, next) => (next.called ? next(e) : { deny: await closedFailed($, 'merge-tripwire', next.error) }))

  on('command.run', { command: 'rex', args: /^limit(\s|$)/ }, ($, e) => serveLimit($, e)).catch((_$, _e, next) => ({
    text: `rex limit: failed (${failureName(next.error)}); nothing was changed after the failure`,
  }))
}

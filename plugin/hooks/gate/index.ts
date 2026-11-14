// Refuses an agent start or a shell command that a gate check refuses: spend over the limit, depth outside a master tree, a fork in a job, a merge around rex_merge. An inert mod refuses nothing.
import type { AgentSpawnInput, AgentSpawnResult, EngineInterface, HookFailure, On } from 'claude-code'

import { fail, markDegraded, MAX_LIMIT_USD, openJobs, readBook, setLimit } from '../book/index'
import type { GateCheck, Parts } from '../parts'
import { failureName } from '../tools/index'
import { depthVerdict, forkRefusal, isMergeCommand, isOverLimit, isOwnSpawn, jobOfLoop, spendRefusal, tripwireRefusal } from './checks'
import { LIMIT_SUBCOMMAND, serveLimit } from './limit'

export { isMergeCommand, isOwnSpawn, jobOfLoop } from './checks'

export const GATE_CHECKS: readonly GateCheck[] = [
  { name: 'spend', site: { kind: 'agent', at: 'both' }, failure: 'open' },
  { name: 'depth', site: { kind: 'agent', at: 'agent.spawn' }, failure: 'closed' },
  { name: 'fork', site: { kind: 'agent', at: 'agent.spawn' }, failure: 'closed' },
  { name: 'merge-tripwire', site: { kind: 'tool', tool: 'Bash' }, failure: 'closed' },
]

/** With storage on, a person may have raised the limit at a shell; a higher limit they set is taken before refusing, if it is one /rex limit would take. */
async function overLimitAfterReread($: EngineInterface, agentId: string | undefined): Promise<string | null> {
  const book = await readBook($)
  const job = jobOfLoop(book, agentId)
  if (book.inert !== null || job === undefined || !isOverLimit(job)) {
    return null
  }
  if (book.storage === 'on') {
    const read = await $.process.run(['rex', 'limit', job.id, '--json'], { cwd: await $.session.root() })
    const stored: unknown = read.exitCode === 0 ? JSON.parse(read.stdout) : null
    const usd: unknown = typeof stored === 'object' && stored !== null ? Reflect.get(stored, 'limit_usd') : undefined
    const source: unknown = typeof stored === 'object' && stored !== null ? Reflect.get(stored, 'source') : undefined
    const isInRange = typeof usd === 'number' && Number.isFinite(usd) && usd >= 0 && usd <= MAX_LIMIT_USD
    const nano = isInRange ? Math.round(usd * 1e9) : 0
    if (source === 'set' && nano > job.limit.nano) {
      await setLimit($, job.id, { nano, source: 'set' })
      if (job.spentNano < nano) {
        return null
      }
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
    const newest = [...openJobs(book)].sort((a, b) => b.createdAt - a.createdAt)[0]
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

async function tripwireAtBash($: EngineInterface, command: string, agentId: string | undefined): Promise<string | null> {
  const book = await readBook($)
  if (book.inert !== null) {
    return null
  }
  const base = jobOfLoop(book, agentId)?.base ?? 'main'
  return isMergeCommand(command, base) ? tripwireRefusal(command, base) : null
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

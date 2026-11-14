// The mod's own spawns as the engine raises them: pre-flight 5 at the master's spawn, where the session's permission mode is first known, and the link of a builder to its job.
import type { AgentSpawnInput, AgentSpawnResult, EngineInterface } from 'claude-code'

import type { RexPendingSpawn } from '../../types'
import { fail, linkBuilder, readBook, startRun } from '../book/index'
import { DEFAULT_POLICY } from './policy'
import { readPolicy } from './store'

/** The pending spawn this event is, when this plugin's own `$.agent.spawn` raised it. */
export async function ownPending($: EngineInterface, event: AgentSpawnInput, origin: string | undefined): Promise<RexPendingSpawn | undefined> {
  if (origin !== 'super-rex' || event.parentAgentId !== undefined) {
    return undefined
  }
  return (await readBook($)).pendingSpawns.find(spawn => spawn.description === event.description)
}

/** The allow rules of the merged settings, as plain text. */
async function allowRules($: EngineInterface): Promise<readonly string[]> {
  const permissions: unknown = await $.settings.read({ key: 'permissions' })
  const allow: unknown = typeof permissions === 'object' && permissions !== null ? Reflect.get(permissions, 'allow') : undefined
  return Array.isArray(allow) ? allow.filter((rule: unknown): rule is string => typeof rule === 'string') : []
}

/**
 * Headless, an ask is a deny, so a master runs only where no ask comes up:
 * bypass, or every allow rule the policy requires already in the settings.
 */
export async function permissionRefusal($: EngineInterface, event: AgentSpawnInput, isInteractive: boolean): Promise<string | null> {
  if (isInteractive) {
    return null
  }
  const mode = event.permissionMode ?? 'unknown'
  if (mode === 'bypassPermissions') {
    return null
  }
  const required = ((await readPolicy($))?.policy ?? DEFAULT_POLICY).required_allow
  const allowed = await allowRules($)
  const missing = required.filter(rule => !allowed.includes(rule))
  if (required.length > 0 && missing.length === 0) {
    return null
  }
  const seen = required.length === 0 ? 'required_allow is empty' : missing.join(', ')
  return `rex master: headless mode ${mode} asks no one, so each ask is a deny; run with bypassPermissions or list allow rules in policy required_allow, got ${seen}`
}

/** A check that throws refuses the master's spawn, as authority must. */
async function guardedRefusal($: EngineInterface, event: AgentSpawnInput, isInteractive: boolean): Promise<string | null> {
  try {
    return await permissionRefusal($, event, isInteractive)
  } catch (error) {
    const name = error instanceof Error ? error.name : 'Error'
    await fail($, { kind: 'authority-check-failed', check: 'master-permission-mode', error: error instanceof Error ? `${name}: ${error.message}` : String(error) })
    return `rex master: the permission check failed (${name}); refused`
  }
}

/** What one `$.agent.spawn` of the mod's own carries between the call and its spawn hook, by the spawn's fixed description. */
const inFlight = new Map<string, { readonly subagentType: string; agentId: string | null }>()

/**
 * Spawns one of the mod's own agents. The spawn hook records the agent id it
 * saw; the answer takes it from there when the call's own answer lacks it, as
 * the test kit's answer does.
 */
export async function spawnOwn($: EngineInterface, request: { readonly subagentType: string; readonly description: string; readonly prompt: string; readonly cwd?: string }): Promise<AgentSpawnResult> {
  const flight = { subagentType: request.subagentType, agentId: null }
  inFlight.set(request.description, flight)
  try {
    const result = await $.agent.spawn(request)
    if (result.deny !== undefined || result.agentId !== undefined || flight.agentId === null) {
      return result
    }
    return { ...result, agentId: flight.agentId }
  } finally {
    inFlight.delete(request.description)
  }
}

/**
 * The run of the mod's own agent is recorded here, before the observer
 * records it, so the master's link to its tree and a builder's link to its job
 * never rest on an observer, which may fail and let the spawn go on.
 */
async function recordOwnRun($: EngineInterface, event: AgentSpawnInput, pending: RexPendingSpawn, agentId: string): Promise<void> {
  const flight = inFlight.get(pending.description)
  if (flight !== undefined) {
    flight.agentId = agentId
  }
  const agentType = flight?.subagentType ?? event.subagentType
  await startRun($, { agentId, toolUseId: event.tool_use_id ?? null, parentAgentId: null, agentType, description: pending.description, isOwnSpawn: true })
  if (pending.kind === 'builder' && pending.job !== null) {
    await linkBuilder($, pending.tree, pending.job, agentId)
  }
}

export async function ownSpawn($: EngineInterface, event: AgentSpawnInput, origin: string | undefined, isInteractive: boolean, next: (e: AgentSpawnInput) => Promise<AgentSpawnResult>): Promise<AgentSpawnResult> {
  const pending = await ownPending($, event, origin)
  if (pending === undefined) {
    return next(event)
  }
  if (pending.kind === 'master') {
    const refusal = await guardedRefusal($, event, isInteractive)
    if (refusal !== null) {
      await fail($, { kind: 'preflight-failed', step: 5, text: refusal })
      return { deny: refusal }
    }
  }
  const result = await next(event)
  if (result.agentId !== undefined) {
    await recordOwnRun($, event, pending, result.agentId)
  }
  return result
}

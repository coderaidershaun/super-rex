// Registers the mod's tools with the engine and serves its own; the dispatcher, last of all tool hooks, answers every super-rex tool call no owner answered.
import type { EngineInterface, HookFailure, On, ToolCallResult, ToolSpec } from 'claude-code'

import { fail, readBook } from '../book/index'
import type { RexBook } from '../../types'
import type { Caller, Parts, StatusSection, Table, ToolEntry } from '../parts'
import { argsOf, field } from './args'
import { callerIn, callerRefusal } from './caller'
import { serveJobClose, serveRecall } from './close'
import { JOB_CLOSE_TOOL, JOB_PLAN_TOOL, OWN_TOOLS, RECALL_TOOL } from './entries'
import { serveJobOpen } from './open'
import { serveJobPlan } from './plan'
import { OWN_SECTIONS, statusLines } from './status'
import { answer, deny, failureName } from './text'

export { callerRefusal, jobArgRefusal, ownJobAnyStatus } from './caller'
export { statusLines } from './status'
export { runOms } from './plan'
export type { OmsAnswer, OmsInput } from './plan'
export { answer, deny, failureName, linesText, usd } from './text'

/** The full name of a tool of this plugin: what a `tool.call` matcher names. */
export const TOOL_PREFIX = 'mcp__super-rex__'

export async function callerOf($: EngineInterface, agentId: string | undefined): Promise<Caller> {
  return callerIn(await readBook($), agentId)
}

export function toolSpecs(table: Table<ToolEntry>): readonly ToolSpec[] {
  return table.entries().map(entry => ({ name: entry.name, description: entry.description, inputSchema: { ...entry.inputSchema }, isDeferred: entry.isDeferred }))
}

/** A tool hook that threw refuses its call: a tool that fails never acts by halves. */
export async function toolFailed($: EngineInterface, tool: string, error: HookFailure): Promise<ToolCallResult> {
  const name = failureName(error)
  await fail($, { kind: 'authority-check-failed', check: tool, error: error.message ?? error.kind })
  return deny(`rex: ${tool} failed (${name}); refused`)
}

async function serveStatus($: EngineInterface, event: { readonly agentId?: string }, sections: Table<StatusSection>): Promise<ToolCallResult> {
  const book = await readBook($)
  const caller = callerIn(book, event.agentId)
  const asked = field(argsOf(event), 'job')
  if (asked !== undefined && asked !== null) {
    const job = book.jobs.find(each => each.id === asked)
    if (job === undefined) {
      return deny(`rex_status: no job ${String(asked)} in this session, got ${String(asked)}; call it with no job for your own`)
    }
    return answer(statusLines(book, job, caller, sections))
  }
  return answer(statusLines(book, caller.job, caller, sections))
}

/** What the dispatcher answers a call of `prefix` that no owner answered: the caller rule of a table name, else no such tool. */
export function dispatchAnswer(book: RexBook, event: { readonly tool: string; readonly agentId?: string }, table: Table<ToolEntry>, prefix: string): string {
  const name = event.tool.slice(prefix.length)
  const entry = table.get(name)
  if (entry === undefined) {
    return `${event.tool}: no such rex tool, got ${name}`
  }
  const caller = callerIn(book, event.agentId)
  return callerRefusal(entry, caller) ?? `${name}: only ${entry.callers.join(' or ')} may call it, got ${caller.kind}`
}

async function dispatch($: EngineInterface, event: { readonly tool: string; readonly agentId?: string }, table: Table<ToolEntry>, prefix: string): Promise<ToolCallResult> {
  const text = dispatchAnswer(await readBook($), event, table, prefix)
  await fail($, { kind: 'caller-not-allowed', tool: event.tool, caller: event.agentId ?? 'main' })
  return deny(text)
}

/**
 * Registers every tool of the table at session start, and the dispatcher on
 * `prefix`. It must be the last `tool.call` registration of the plugin.
 */
export function registerToolTable(on: On, parts: Parts, prefix: string): void {
  on('session.start', { cwd: /(?:)/ }, async ($, e, next) => {
    const book = await readBook($)
    if (book.inert === null) {
      for (const spec of toolSpecs(parts.tools)) {
        await $.tool.register(spec)
      }
    }
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'session.start', module: 'tools', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('tool.call', { tool: new RegExp(`^${prefix}`) }, ($, e) => dispatch($, e, parts.tools, prefix)).catch(($, _e, next) => toolFailed($, 'dispatcher', next.error))
}

export function register(on: On, parts: Parts): void {
  for (const entry of OWN_TOOLS) {
    parts.tools.add(entry)
  }
  for (const section of OWN_SECTIONS) {
    parts.statusSections.add(section)
  }

  on('tool.call', { tool: 'mcp__super-rex__rex_status' }, ($, e) => serveStatus($, e, parts.statusSections)).catch(($, _e, next) => toolFailed($, 'rex_status', next.error))
  on('tool.call', { tool: 'mcp__super-rex__rex_job_open' }, ($, e) => serveJobOpen($, e)).catch(($, _e, next) => toolFailed($, 'rex_job_open', next.error))
  on('tool.call', { tool: 'mcp__super-rex__rex_recall' }, ($, e) => serveRecall($, e, RECALL_TOOL)).catch(($, _e, next) => toolFailed($, 'rex_recall', next.error))
  on('tool.call', { tool: 'mcp__super-rex__rex_job_plan' }, ($, e) => serveJobPlan($, e, JOB_PLAN_TOOL)).catch(($, _e, next) => toolFailed($, 'rex_job_plan', next.error))
  on('tool.call', { tool: 'mcp__super-rex__rex_job_close' }, ($, e) => serveJobClose($, e, JOB_CLOSE_TOOL)).catch(($, _e, next) => toolFailed($, 'rex_job_close', next.error))

  registerToolTable(on, parts, TOOL_PREFIX)
}

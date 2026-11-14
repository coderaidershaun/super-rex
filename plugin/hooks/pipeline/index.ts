// Assembles each new agent's brief at its spawn: lints the author's text once, then wraps it in the brief parts; agents of a resume, a team, no job or an inert mod pass as they came.
import type { AgentSpawnInput, AgentSpawnResult, EngineInterface, On, ToolCallResult } from 'claude-code'

import type { RexJob, RexRun } from '../../types'
import { fail, readBook, recordLintAttempt, recordSkillOpen, setRunOutput, startRun } from '../book/index'
import { jobOfLoop } from '../gate/index'
import type { BriefContext, BriefFile, BriefPart, Parts, Table } from '../parts'
import { hashText } from '../skills/index'
import { briefFiles, lateFacts, lintKey, roleFor, spawnCase } from './brief'
import { lintFindings, lintRefusal, taskLine } from './lint'
import { assembled, outputFileOf, OWN_PARTS } from './parts'
import { gridOf } from './tiers'

export { assembled, OWN_PARTS } from './parts'
export { lintFindings } from './lint'

type Assembly = {
  readonly prompt: string
  readonly job: RexJob
  readonly task: string | null
  readonly planned: string | null
  readonly lint: NonNullable<RexRun['lint']>
  readonly findings: readonly string[]
  readonly files: readonly BriefFile[]
  readonly outputFile: string
}

type Decision = { readonly kind: 'pass' } | { readonly kind: 'deny'; readonly text: string } | { readonly kind: 'assemble'; readonly assembly: Assembly }

const PLUGIN_PREFIX = 'super-rex:'

async function decide($: EngineInterface, event: AgentSpawnInput, origin: string | undefined, parts: Table<BriefPart>): Promise<Decision> {
  const book = await readBook($)
  const job = jobOfLoop(book, event.parentAgentId)
  if (book.inert !== null || spawnCase(book, event, origin, job) !== 'new' || job === undefined) {
    return { kind: 'pass' }
  }
  if (book.runs.some(run => run.toolUseId === event.tool_use_id && run.assembled === 'yes')) {
    return { kind: 'pass' }
  }
  const task = taskLine(event.prompt)
  const findings = lintFindings(event.prompt, job.plan === null ? null : job.plan.tasks.map(each => each.id))
  if (findings.length > 0) {
    const stored = await recordLintAttempt($, { key: lintKey(event.parentAgentId, job, task), findings })
    if (stored === null) {
      await fail($, { kind: 'lint-findings', findings })
      return { kind: 'deny', text: lintRefusal(findings) }
    }
  }
  const role = roleFor(book, job, task, event.subagentType, event.parentAgentId)
  const files = await briefFiles($, job, task, role, event.subagentType)
  const outputFile = outputFileOf(book, job, role)
  const ctx: BriefContext = { book, job, run: null, spawn: event, role, task, files }
  const grid = gridOf(event.subagentType)
  const planned = grid === null ? null : event.subagentType.replace(PLUGIN_PREFIX, '')
  return {
    kind: 'assemble',
    assembly: { prompt: assembled(parts.entries(), ctx, event.prompt), job, task, planned, lint: findings.length > 0 ? 'passed-marked' : 'pass', findings, files, outputFile },
  }
}

async function recordAssembly($: EngineInterface, event: AgentSpawnInput, agentId: string, assembly: Assembly): Promise<void> {
  await startRun($, {
    agentId,
    toolUseId: event.tool_use_id,
    parentAgentId: event.parentAgentId ?? null,
    agentType: event.subagentType,
    description: event.description,
    isOwnSpawn: false,
    planned: assembly.planned,
    task: assembly.task,
    briefHash: await hashText(assembly.prompt),
    assembled: 'yes',
    lint: assembly.lint,
    lintFindings: assembly.findings,
    skillsAttached: assembly.files.filter(file => file.file === 'SKILL.md').map(file => file.skill),
  })
  await setRunOutput($, agentId, assembly.outputFile)
  for (const file of assembly.files) {
    await recordSkillOpen($, agentId, { skill: file.skill, file: file.file, hash: file.hash, how: 'attached' })
  }
}

async function assembleAtSpawn($: EngineInterface, event: AgentSpawnInput, origin: string | undefined, parts: Table<BriefPart>, next: (e: AgentSpawnInput) => Promise<AgentSpawnResult>): Promise<AgentSpawnResult> {
  const decision = await decide($, event, origin, parts)
  switch (decision.kind) {
    case 'pass':
      return next(event)
    case 'deny':
      return { deny: decision.text }
    case 'assemble': {
      const result = await next({ ...event, prompt: decision.assembly.prompt })
      if (result.agentId !== undefined) {
        await recordAssembly($, event, result.agentId, decision.assembly)
      }
      return result
    }
    default:
      return assertNoDecision(decision)
  }
}

function assertNoDecision(decision: never): never {
  throw new Error(`unhandled spawn decision: ${JSON.stringify(decision)}`)
}

/** The parts a late assembly hands an agent that started without them. */
const LATE_PARTS = new Set(['skills', 'tier', 'return'])

async function lateAssembly($: EngineInterface, agentId: string | undefined, parts: Table<BriefPart>): Promise<ToolCallResult> {
  const book = await readBook($)
  const run = agentId === undefined ? undefined : book.runs.find(each => each.agentId === agentId)
  const job = run?.job === null || run === undefined ? undefined : book.jobs.find(each => each.id === run.job)
  if (run === undefined || job === undefined) {
    return { deny: `rex_skill: with no name it gives a job's agent the brief parts it is missing, got ${agentId ?? 'main'} with no job; call it with a skill name` }
  }
  const facts = lateFacts(run)
  const files = await briefFiles($, job, facts.task, facts.role, facts.agentType)
  const ctx: BriefContext = { book, job, run, spawn: null, role: facts.role, task: facts.task, files }
  const text = assembled(parts.entries().filter(part => LATE_PARTS.has(part.name)), ctx, null)
  await startRun($, { agentId: run.agentId, toolUseId: run.toolUseId, parentAgentId: null, agentType: run.agentType, description: '', isOwnSpawn: false, assembled: 'late' })
  if (run.outputFile === null) {
    await setRunOutput($, run.agentId, outputFileOf(book, job, facts.role))
  }
  return { result: `assembly=late\n\n${text}` }
}

function hasName(event: { readonly [key: string]: unknown }): boolean {
  const name = event['name']
  return typeof name === 'string' && name !== ''
}

export function register(on: On, parts: Parts): void {
  for (const part of OWN_PARTS) {
    parts.briefParts.add(part)
  }

  on('agent.spawn', { subagentType: /(?:)/ }, ($, e, next) => assembleAtSpawn($, e, next.origin.plugin, parts.briefParts, next)).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'agent.spawn', module: 'pipeline', agentId: e.parentAgentId ?? null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('tool.call', { tool: 'mcp__super-rex__rex_skill' }, async ($, e, next) => (hasName(e) ? next(e) : lateAssembly($, e.agentId, parts.briefParts))).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'tool.call', module: 'pipeline', agentId: e.agentId ?? null, error: next.error.message ?? next.error.kind })
    return next.called ? next(e) : { deny: 'rex: rex_skill failed while it assembled the brief; call it with a skill name' }
  })
}

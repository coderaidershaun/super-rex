// Turns what the engine reports about spawns, model requests, agent ends, permission asks and the merge dialog into changes of the live record.
import type { On, TurnUsage } from 'claude-code'

import { countPermissionAsk, endRun, errorText, fail, noteTranscript, readBook, setMergeAnswer, startRun, stepRun } from '../book/index'
import type { Parts } from '../parts'

/** The builder skill asks the merge question in exactly these words; any other dialog records nothing. */
export const MERGE_QUESTION = 'Merge this job to main when its checks pass?'
export const MERGE_YES = 'Yes, merge'
export const MERGE_NO = 'No, keep the branch'

export class UnsafeCountError extends Error {
  readonly field: string
  readonly value: number

  constructor(field: string, value: number) {
    super(`usage ${field} is ${String(value)}, not a whole count the tally can hold`)
    this.name = 'UnsafeCountError'
    this.field = field
    this.value = value
  }
}

/** Token counts arrive from outside the mod: each must be a safe, non-negative integer before it is added up. */
function checkedUsage(usage: TurnUsage | null): TurnUsage | null {
  if (usage === null) {
    return null
  }
  const counts = {
    input_tokens: usage.input_tokens,
    output_tokens: usage.output_tokens,
    cache_read_input_tokens: usage.cache_read_input_tokens,
    cache_creation_input_tokens: usage.cache_creation_input_tokens,
  }
  for (const [field, value] of Object.entries(counts)) {
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new UnsafeCountError(field, value)
    }
  }
  return usage
}

/** One field of a value from outside the mod, or undefined when the value has no such field. */
function fieldOf(value: unknown, key: string): unknown {
  return typeof value === 'object' && value !== null ? Reflect.get(value, key) : undefined
}

export function register(on: On, _parts: Parts): void {
  on('agent.spawn', async ($, e, next) => {
    const result = await next(e)
    if (result.agentId !== undefined) {
      await startRun($, {
        agentId: result.agentId,
        toolUseId: e.tool_use_id,
        parentAgentId: e.parentAgentId ?? null,
        agentType: e.subagentType,
        description: e.description,
        isOwnSpawn: next.origin.plugin === 'super-rex',
      })
    }
    return result
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'agent.spawn', module: 'observe', agentId: e.parentAgentId ?? null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('turn.step', async function* ($, e, next) {
    const result = yield* next(e)
    if (e.agentId !== undefined) {
      try {
        const usage = checkedUsage(result.usage)
        await stepRun($, e.agentId, { model: e.model, effort: e.effort === undefined ? null : String(e.effort), usage })
      } catch (error) {
        await fail($, { kind: 'observer-failed', event: 'turn.step', module: 'observe', agentId: e.agentId, error: errorText(error) })
      }
    }
    return result
  })

  on('turn.complete', async ($, e, next) => {
    if (e.agentId !== undefined) {
      await endRun($, e.agentId, { reason: e.reason, answer: e.answer })
    }
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'turn.complete', module: 'observe', agentId: e.agentId ?? null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('tool.call', { tool: 'AskUserQuestion' }, async ($, e, next) => {
    const result = await next(e)
    const asked = e.questions.find(question => question.question === MERGE_QUESTION)
    const labels = asked?.options.map(option => option.label) ?? []
    const isMergeDialog = e.agentId === undefined && labels.length === 2 && labels.includes(MERGE_YES) && labels.includes(MERGE_NO)
    const output: unknown = result.result
    if (!isMergeDialog || fieldOf(output, 'afkTimeoutMs') !== undefined) {
      return result
    }
    const answer = fieldOf(fieldOf(output, 'answers'), MERGE_QUESTION)
    const book = await readBook($)
    const job = book.jobs.find(each => each.owner.builder === 'main' && each.owner.session === book.session && (each.status === 'created' || each.status === 'planned'))
    if (job !== undefined && (answer === MERGE_YES || answer === MERGE_NO)) {
      await setMergeAnswer($, job.id, { answer: answer === MERGE_YES ? 'yes' : 'no', source: 'dialog', question: null, policyHash: null })
    }
    return result
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'tool.call', module: 'observe', agentId: e.agentId ?? null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('tool.check', async ($, e, next) => {
    const verdict = await next(e)
    if (verdict.decision === 'ask') {
      await countPermissionAsk($)
    }
    return verdict
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'tool.check', module: 'observe', agentId: e.agentId ?? null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('classic.SessionStart', async ($, e, next) => {
    if (e.transcript_path !== '') {
      await noteTranscript($, e.transcript_path)
    }
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'classic.SessionStart', module: 'observe', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })
}

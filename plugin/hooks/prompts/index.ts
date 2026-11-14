// Records the person's prompts unchanged and adds a restatement of a job's request beside it, so the person's words stay the request.
import type { EngineInterface, On, PromptOrigin, PromptSubmitInput } from 'claude-code'

import { fail, isOpen, limitIgnoredLine, readBook, readPersonWords, recordPrompt, setLimit, setMergeAnswer } from '../book/index'
import type { RexBook, RexOriginKind } from '../../types'
import type { Parts } from '../parts'
import { missingItems, restate, restatementContext } from './restate'

export { restate } from './restate'
export type { Restated } from './restate'

/** The command names a job-start prompt runs: the plugin's skill, and its bare name, which the engine may also give. */
const JOB_START_COMMANDS = new Set(['super-rex:orchestration-builder', 'orchestration-builder'])

/** Only the person's own words are acted on; an `sdk` prompt is the person's when the config lets jobs start from it. */
function personOrigin(origin: PromptOrigin, book: RexBook): RexOriginKind | null {
  if (origin.kind === 'composer' || origin.kind === 'bridge') {
    return origin.kind
  }
  return origin.kind === 'sdk' && book.jobsFromSdk ? 'sdk' : null
}

function withContext(e: PromptSubmitInput, lines: readonly string[]): PromptSubmitInput {
  return lines.length === 0 ? e : { ...e, context: [...(e.context ?? []), ...lines] }
}

/** The job-start prompt: recorded, restated in mode `improve`, and kept so the job that opens next takes its words. */
async function jobStart($: EngineInterface, e: PromptSubmitInput, origin: RexOriginKind, book: RexBook): Promise<readonly string[]> {
  const request = e.command?.args ?? e.text
  const restated = book.prompts === 'improve' ? await restate($, request) : null
  const restatement = restated?.isRestated === true ? restated.text : null
  const skipped = restated?.isRestated === false ? restated.reason : null
  await recordPrompt($, {
    text: e.text,
    origin,
    command: e.command ?? null,
    restatement,
    isRecorded: book.prompts !== 'off',
    jobStart: { origin, text: request, restatement, restatementSkipped: skipped },
  })
  if (skipped !== null) {
    await fail($, { kind: 'restatement-failed', reason: skipped })
  }
  const lines: string[] = []
  if (restatement !== null) {
    $.ui.status(`rex: added restatement (${missingItems(restatement)})`)
    lines.push(restatementContext(restatement))
  }
  const words = readPersonWords(request)
  if (words.limit.kind === 'ignored') {
    lines.push(limitIgnoredLine(words.limit.text))
  }
  return lines
}

/** Any other prompt of the person while the main loop owns a job: rules only, no model call. */
async function insideJob($: EngineInterface, e: PromptSubmitInput, origin: RexOriginKind, book: RexBook): Promise<readonly string[]> {
  await recordPrompt($, { text: e.text, origin, command: e.command ?? null, restatement: null, isRecorded: book.prompts !== 'off', jobStart: null })
  const job = book.jobs.find(each => isOpen(each) && each.owner.session === book.session && each.owner.builder === 'main')
  if (job === undefined) {
    return []
  }
  const words = readPersonWords(e.text)
  const lines: string[] = []
  if (words.merge !== null) {
    await setMergeAnswer($, job.id, { answer: words.merge, source: origin === 'sdk' ? 'sdk-prompt' : 'person-prompt', question: null, policyHash: null })
  }
  if (words.limit.kind === 'amount') {
    await setLimit($, job.id, { nano: words.limit.nano, source: 'set' })
  }
  if (words.limit.kind === 'ignored') {
    lines.push(limitIgnoredLine(words.limit.text))
  }
  if (words.merge === null && job.merge === null) {
    lines.push('rex: this job has no merge answer yet')
  }
  return lines
}

export function register(on: On, _parts: Parts): void {
  on('prompt.submit', async ($, e, next) => {
    const book = await readBook($)
    const origin = book.inert === null ? personOrigin(e.origin, book) : null
    if (origin === null) {
      return next(e)
    }
    const isJobStart = e.command !== undefined && JOB_START_COMMANDS.has(e.command.name)
    const lines = isJobStart ? await jobStart($, e, origin, book) : await insideJob($, e, origin, book)
    return next(withContext(e, lines))
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'prompt.submit', module: 'prompts', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })
}

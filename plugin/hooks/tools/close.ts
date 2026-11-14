// rex_recall and rex_job_close: the store's lessons for the builder's job, and the close of that job with its report.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexDelivery, RexJob } from '../../types'
import { closeJob, fail, isOpen, readBook } from '../book/index'
import type { JobClose } from '../book/index'
import type { ToolEntry } from '../parts'
import { argsOf, field, isRefused, objectList, optionalText, textArg, word } from './args'
import type { Args, Refused } from './args'
import { jsonField, parsedJson, readBinary } from './binary'
import { callerIn, callerRefusal, jobArgRefusal, ownJobAnyStatus } from './caller'
import { answer, deny, usd } from './text'

const STORAGE_OFF = 'reason=storage is off; run rex init --with-storage'
/** A final message above this many characters is named in the close report: the brief asks for at most 2,000. */
const LONG_ANSWER = 2000

export async function serveRecall($: EngineInterface, event: { readonly agentId?: string }, entry: ToolEntry): Promise<ToolCallResult> {
  const book = await readBook($)
  const caller = callerIn(book, event.agentId)
  const refusal = callerRefusal(entry, caller)
  if (refusal !== null || caller.job === null) {
    await fail($, { kind: 'caller-not-allowed', tool: entry.name, caller: caller.kind })
    return deny(refusal ?? `${entry.name}: only a builder may call it, got ${caller.kind}`)
  }
  const jobText = jobArgRefusal(entry.name, field(argsOf(event), 'job'), caller.job)
  if (jobText !== null) {
    return deny(jobText)
  }
  if (book.storage === 'off') {
    return answer(['recall=off', STORAGE_OFF])
  }
  const read = await readBinary($, ['rex', 'recall', caller.job.id])
  return read.ok ? { result: read.stdout } : deny(read.line)
}

const OUTCOMES: readonly RexDelivery['outcome'][] = ['merged', 'pr-open', 'kept-local', 'blocked']
const STATUSES: readonly JobClose['status'][] = ['delivered', 'blocked', 'failed', 'cancelled']

function deliveryOf(tool: string, item: Args): RexDelivery | Refused {
  const todo = 'give each delivery repo and outcome'
  const repo = textArg(tool, item, 'repo', todo)
  const outcome = word(tool, item, 'outcome', OUTCOMES)
  const branch = optionalText(tool, item, 'branch', todo)
  const pr = optionalText(tool, item, 'pr', todo)
  const mergeCommit = optionalText(tool, item, 'merge_commit', todo)
  const note = optionalText(tool, item, 'note', todo)
  if (isRefused(repo)) return repo
  if (isRefused(outcome)) return outcome
  if (isRefused(branch)) return branch
  if (isRefused(pr)) return pr
  if (isRefused(mergeCommit)) return mergeCommit
  if (isRefused(note)) return note
  return { repo, branch, pr, outcome, mergeCommit, note }
}

function closeOf(tool: string, args: Args): JobClose | Refused {
  const status = word(tool, args, 'status', STATUSES)
  const outcome = textArg(tool, args, 'outcome', 'give the outcome in one line')
  const items = objectList(tool, args, 'deliveries', 'give a list of deliveries, empty when none')
  if (isRefused(status)) return status
  if (isRefused(outcome)) return outcome
  if (isRefused(items)) return items
  const deliveries: RexDelivery[] = []
  for (const item of items) {
    const delivery = deliveryOf(tool, item)
    if (isRefused(delivery)) return delivery
    deliveries.push(delivery)
  }
  return { status, outcome, deliveries }
}

/** A delivery the caller calls merged takes the commit rex_merge recorded; with none recorded it is refused. */
function withMerges(tool: string, job: RexJob, close: JobClose): JobClose | Refused {
  const deliveries: RexDelivery[] = []
  for (const delivery of close.deliveries) {
    if (delivery.outcome !== 'merged') {
      deliveries.push(delivery)
      continue
    }
    const done = job.deliveries.find(old => old.outcome === 'merged' && old.repo === delivery.repo && old.mergeCommit !== null)
    if (done === undefined) {
      return { refused: `${tool}: delivery to ${delivery.repo} is merged, got no merge by rex_merge for job ${job.id}; merge it with rex_merge, or close it kept-local` }
    }
    deliveries.push({ ...delivery, branch: delivery.branch ?? done.branch, mergeCommit: done.mergeCommit })
  }
  return { ...close, deliveries }
}

/** The close report: what the job spent and ran, and where its runs left the plan's skills. */
export function closeLines(book: RexBook, job: RexJob): readonly string[] {
  const runs = book.runs.filter(run => run.job === job.id)
  const rows = [`job=${job.id}`, `status=${job.status}`, `spent_usd=${usd(job.spentNano)}`, `runs=${String(runs.length)}`]
  for (const role of job.plan?.roles ?? []) {
    const planned = new Set((job.plan?.tasks ?? []).filter(task => task.role === role.role).flatMap(task => task.skills))
    const roleRuns = runs.filter(run => run.role === role.role)
    const seen = new Set(roleRuns.flatMap(run => [...run.skillsAttached, ...run.skillsOpened.map(open => open.split('/')[0] ?? open)]))
    if (roleRuns.length > 0) {
      rows.push(...[...planned].filter(skill => !seen.has(skill)).map(skill => `missed.${role.role}=${skill}`))
      rows.push(...[...seen].filter(skill => !planned.has(skill)).map(skill => `extra.${role.role}=${skill}`))
    }
  }
  rows.push(...runs.filter(run => (run.answerChars ?? 0) > LONG_ANSWER).map(run => `long.${run.agentId}=${String(run.answerChars ?? 0)}`))
  rows.push(`not_assembled=${String(runs.filter(run => run.assembled === 'no').length)}`)
  rows.push(`degraded=${job.degraded.length === 0 ? 'none' : job.degraded.join(',')}`)
  return rows
}

function isSameClose(job: RexJob, close: JobClose): boolean {
  return job.status === close.status && job.outcome === close.outcome
}

export async function serveJobClose($: EngineInterface, event: { readonly agentId?: string }, entry: ToolEntry): Promise<ToolCallResult> {
  const tool = entry.name
  const book = await readBook($)
  const caller = callerIn(book, event.agentId)
  const job = caller.job ?? ownJobAnyStatus(book, event.agentId)
  const isClosedRepeat = caller.job === null && job !== undefined && !isOpen(job)
  const refusal = isClosedRepeat ? null : callerRefusal(entry, caller)
  if (refusal !== null || job === undefined) {
    await fail($, { kind: 'caller-not-allowed', tool, caller: caller.kind })
    return deny(refusal ?? `${tool}: only a builder may call it, got ${caller.kind}`)
  }
  const args = argsOf(event)
  const parsed = closeOf(tool, args)
  const jobText = jobArgRefusal(tool, field(args, 'job'), job)
  if (jobText !== null || isRefused(parsed)) {
    await fail($, { kind: 'bad-argument', tool, argument: 'arguments', value: jobText ?? (isRefused(parsed) ? parsed.refused : '') })
    return deny(jobText ?? (isRefused(parsed) ? parsed.refused : ''))
  }
  if (isClosedRepeat && !isSameClose(job, parsed)) {
    return deny(`${tool}: job ${job.id} is closed ${job.status}, got ${parsed.status}; call it again with the same status and outcome, or open a new job`)
  }
  const close = isClosedRepeat ? { ...parsed, deliveries: job.deliveries } : withMerges(tool, job, parsed)
  if (isRefused(close)) {
    return deny(close.refused)
  }
  await closeJob($, job.id, close)
  const closed = (await readBook($)).jobs.find(each => each.id === job.id) ?? job
  const rows = closeLines(await readBook($), closed)
  if (book.storage === 'off') {
    return answer(rows)
  }
  const shown = await readBinary($, ['rex', 'job', 'show', job.id, '--json'])
  const cost = shown.ok ? jsonField(jsonField(parsedJson(shown.stdout), 'cost'), 'usd') : undefined
  return answer([...rows, `cost_of_record_usd=${typeof cost === 'number' ? cost.toFixed(2) : 'unknown'}`])
}

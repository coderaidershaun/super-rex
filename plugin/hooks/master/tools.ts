// The master's tools besides rex_build and rex_report: a message to one of its builders, the end of a job it gives up, a note of its own decision, and a question parked for the person.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob, RexTree } from '../../types'
import { closeJob, fail, isOpen, noteTree, readBook } from '../book/index'
import type { Caller, ToolEntry } from '../parts'
import { answer, callerOf, callerRefusal, deny } from '../tools/index'
import { parkQuestion } from './ask'
import { field, isRefused, optionalText, refused, text, word } from './args'
import { treeOfMaster } from './pin'
import { DEFAULT_POLICY } from './policy'
import { reviveBuilders } from './resume'
import { readPolicy } from './store'
import { wakeLater } from './wake'

type MasterCall = { readonly tree: RexTree; readonly book: RexBook } | { readonly refused: string; readonly caller: Caller }

/** The caller's live tree when it is that tree's master, else the refusal the model reads and who was refused. */
export async function masterCall($: EngineInterface, entry: ToolEntry, agentId: string | undefined): Promise<MasterCall> {
  const caller = await callerOf($, agentId)
  const book = await readBook($)
  const tree = caller.kind === 'master' ? treeOfMaster(book, caller.agentId) : undefined
  if (tree === undefined) {
    await fail($, { kind: 'caller-not-allowed', tool: entry.name, caller: caller.kind })
    return { refused: callerRefusal(entry, caller) ?? `${entry.name}: only the master of a live tree may call it, got a caller with no live tree; the person starts a tree with /super-rex:orchestration-master <goal>`, caller }
  }
  return { tree, book }
}

/** A job of the caller's own tree, named by its `job` argument. */
function treeJob(tool: string, book: RexBook, tree: RexTree, id: string): RexJob | { readonly refused: string } {
  const job = book.jobs.find(each => each.id === id)
  if (job?.tree !== tree.id) {
    return refused(tool, `job ${id} is not a job of your tree ${tree.id}`, id, `call it with one of ${book.jobs.filter(each => each.tree === tree.id).map(each => each.id).join(', ') || 'your jobs, once rex_build opened one'}`)
  }
  return job
}

export async function serveContinue($: EngineInterface, event: { readonly agentId?: string }, entry: ToolEntry): Promise<ToolCallResult> {
  const call = await masterCall($, entry, event.agentId)
  if ('refused' in call) {
    return deny(call.refused)
  }
  const id = text(entry.name, event, 'job', 'call it again with the job id')
  const message = text(entry.name, event, 'message', 'call it again with the text the builder reads')
  const job = isRefused(id) ? id : treeJob(entry.name, call.book, call.tree, id)
  const problem = isRefused(job) ? job : isRefused(message) ? message : null
  if (problem !== null || isRefused(job) || isRefused(message)) {
    await fail($, { kind: 'bad-argument', tool: entry.name, argument: 'arguments', value: problem?.refused ?? '' })
    return deny(problem?.refused ?? `${entry.name}: bad arguments`)
  }
  if (!isOpen(job)) {
    return deny(`${entry.name}: job ${job.id} is closed, got status ${job.status}; a closed job reads no message: start a fix job with rex_build, or mark its step not built`)
  }
  const builder = call.tree.builders.find(each => each.job === job.id)?.agentId
  if (builder === undefined) {
    return deny(await noBuilder($, entry.name, call.tree, job))
  }
  wakeLater($, builder, message)
  return answer(['sent=yes', `job=${job.id}`])
}

/** An open job with no builder, as after a resume: the step of `on_dead_builder` runs for it, and the master reads what to do next. */
async function noBuilder($: EngineInterface, tool: string, tree: RexTree, job: RexJob): Promise<string> {
  const policy = (await readPolicy($))?.policy ?? DEFAULT_POLICY
  const revived = await reviveBuilders($, tree, [job], policy)
  if (revived.continued.length > 0) {
    return `${tool}: job ${job.id} had no builder, got none; the mod started a new one that goes on from the job's record: call ${tool} again to send your message`
  }
  const asked = revived.asked[0]
  return asked === undefined
    ? `${tool}: job ${job.id} has no builder and none started, got none; give the job up with rex_cancel, then build its step again with rex_build`
    : `${tool}: job ${job.id} has no builder, got none; question ${asked} asks the person whether to start one: tell them, and end your turn`
}

/**
 * The master gives one open job of its tree up: the job closes `cancelled`
 * with the master's reason, its builder is told to stop, and its worktree and
 * branch stay as they are. This is how a tree the person stopped, or a job
 * whose builder is gone, comes to an end that rex_report accepts.
 */
export async function serveCancel($: EngineInterface, event: { readonly agentId?: string }, entry: ToolEntry): Promise<ToolCallResult> {
  const call = await masterCall($, entry, event.agentId)
  if ('refused' in call) {
    return deny(call.refused)
  }
  const id = text(entry.name, event, 'job', 'call it again with the job id')
  const reason = text(entry.name, event, 'reason', 'call it again with why the job ends')
  const job = isRefused(id) ? id : treeJob(entry.name, call.book, call.tree, id)
  const problem = isRefused(job) ? job : isRefused(reason) ? reason : null
  if (problem !== null || isRefused(job) || isRefused(reason)) {
    await fail($, { kind: 'bad-argument', tool: entry.name, argument: 'arguments', value: problem?.refused ?? '' })
    return deny(problem?.refused ?? `${entry.name}: bad arguments`)
  }
  if (!isOpen(job)) {
    return deny(`${entry.name}: job ${job.id} is closed already, got status ${job.status}; nothing to cancel`)
  }
  await closeJob($, job.id, { status: 'cancelled', outcome: `cancelled by the master: ${reason}`, deliveries: job.deliveries })
  await noteTree($, call.tree.id, { job: job.id, text: `rex: the master cancelled this job: ${reason}` })
  const builder = call.tree.builders.find(each => each.job === job.id)?.agentId
  if (builder !== undefined) {
    wakeLater($, builder, `rex: the master cancelled job ${job.id}: ${reason}. The job is closed. Stop work, start no agent, and end your turn.`)
  }
  return answer([`job=${job.id}`, 'status=cancelled'])
}

/** A builder or another agent of a job keeps its own decisions in its report file, which the master and the person read. */
function noteRefusal(refused: string, caller: Caller): string {
  const isOfJob = caller.kind === 'builder' || (caller.kind === 'agent' && caller.job !== null)
  return isOfJob ? `${refused}; write the decision in your report file instead` : refused
}

export async function serveNote($: EngineInterface, event: { readonly agentId?: string }, entry: ToolEntry): Promise<ToolCallResult> {
  const call = await masterCall($, entry, event.agentId)
  if ('refused' in call) {
    return deny(noteRefusal(call.refused, call.caller))
  }
  const note = text(entry.name, event, 'text', 'call it again with what you decided, why, and the option you did not take')
  const id = optionalText(entry.name, event, 'job')
  const job = isRefused(id) || id === null ? id : treeJob(entry.name, call.book, call.tree, id)
  const problem = isRefused(note) ? note : isRefused(job) ? job : null
  if (problem !== null || isRefused(note) || isRefused(job)) {
    await fail($, { kind: 'bad-argument', tool: entry.name, argument: 'arguments', value: problem?.refused ?? '' })
    return deny(problem?.refused ?? `${entry.name}: bad arguments`)
  }
  await noteTree($, call.tree.id, { job: job?.id ?? null, text: note })
  return answer([`noted=${String(call.tree.notes.length + 1)}`])
}

const CATEGORIES = ['merge', 'scope', 'spend', 'other'] as const

function optionsOf(tool: string, event: object): readonly string[] | { readonly refused: string } {
  const value = field(event, 'options')
  const isTextList = Array.isArray(value) && value.every((item: unknown) => typeof item === 'string' && item.trim() !== '')
  if (!isTextList || value.length < 2 || value.length > 4) {
    return refused(tool, 'options must be 2 to 4 texts', value, 'call it again with 2 to 4 options')
  }
  return value.map(String)
}

export async function serveAskOwner($: EngineInterface, event: { readonly agentId?: string }, entry: ToolEntry): Promise<ToolCallResult> {
  const call = await masterCall($, entry, event.agentId)
  if ('refused' in call) {
    return deny(call.refused)
  }
  const question = text(entry.name, event, 'question', 'call it again with the question, ending in a question mark')
  const options = optionsOf(entry.name, event)
  const recommendation = text(entry.name, event, 'recommendation', 'call it again with one of the options')
  const category = word(entry.name, event, 'category', CATEGORIES)
  const id = optionalText(entry.name, event, 'job')
  const job = isRefused(id) || id === null ? id : treeJob(entry.name, call.book, call.tree, id)
  const problem = [question, options, recommendation, category, job].find(isRefused) ?? null
  if (problem !== null || isRefused(question) || isRefused(options) || isRefused(recommendation) || isRefused(category) || isRefused(job)) {
    await fail($, { kind: 'bad-argument', tool: entry.name, argument: 'arguments', value: problem?.refused ?? '' })
    return deny(problem?.refused ?? `${entry.name}: bad arguments`)
  }
  if (!options.includes(recommendation)) {
    return deny(refused(entry.name, 'recommendation must be one of the options', recommendation, `call it again with one of ${options.join(', ')}`).refused)
  }
  const parked = await parkQuestion($, { tree: call.tree.id, job: job?.id ?? null, question, options, recommendation, category, mergeFor: null })
  return answer([`question=${parked.id}`, 'state=parked'])
}

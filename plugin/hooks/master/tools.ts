// The master's tools besides rex_build and rex_report: a message to one of its builders, a note of its own decision, and a question parked for the person.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob, RexTree } from '../../types'
import { fail, noteTree, readBook } from '../book/index'
import type { Caller, ToolEntry } from '../parts'
import { answer, callerOf, callerRefusal, deny } from '../tools/index'
import { parkQuestion } from './ask'
import { field, isRefused, optionalText, refused, text, word } from './args'
import { masterMoved } from './idle'
import { treeOfMaster } from './pin'
import { wakeLater } from './wake'

type MasterCall = { readonly tree: RexTree; readonly book: RexBook } | { readonly refused: string; readonly caller: Caller }

/** The caller's live tree when it is that tree's master, else the refusal the model reads and who was refused. */
export async function masterCall($: EngineInterface, entry: ToolEntry, agentId: string | undefined): Promise<MasterCall> {
  const caller = await callerOf($, agentId)
  const book = await readBook($)
  const tree = caller.kind === 'master' ? treeOfMaster(book, caller.agentId) : undefined
  if (tree === undefined) {
    await fail($, { kind: 'caller-not-allowed', tool: entry.name, caller: caller.kind })
    return { refused: callerRefusal(entry, caller) ?? `${entry.name}: only the master of a live tree may call it, got the master of a closed tree; start a tree with /rex master`, caller }
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
  const builder = call.tree.builders.find(each => each.job === job.id)?.agentId
  if (builder === undefined) {
    return deny(`${entry.name}: job ${job.id} has no builder, got none; call rex_build for it again`)
  }
  wakeLater($, builder, message)
  return answer(['sent=yes', `job=${job.id}`])
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
  masterMoved(call.tree.id)
  return answer([`question=${parked.id}`, 'state=parked'])
}

// How a tree ends: the master calls rex_report once no job is open and no question waits, and a master that stops with nothing open is woken, then its tree closed by the mod after two such wakes.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob, RexTree } from '../../types'
import { closeTree, fail, isOpen, readBook } from '../book/index'
import type { ToolEntry } from '../parts'
import { answer, deny } from '../tools/index'
import { isRefused, text } from './args'
import { handOffFile } from './brief'
import { treeJobs } from './gates'
import { countIdleWake, forgetTree, hasRateWake, idleWakesOf } from './idle'
import { treeOfMaster } from './pin'
import { realRoot } from './store'
import { masterCall } from './tools'
import { isWakeOnTheWay, wakeLater } from './wake'
import { stopWatchdog } from './watch'

/** A master woken this many times in a row with nothing open has stopped for good, so the mod closes its tree. */
export const IDLE_WAKES = 2

export const IDLE_TEXT =
  'rex: your turn ended with no open job and no waiting question. IF the goal of the charter is met, or cannot be met, THEN call rex_report with the final report. ELSE start the next step with rex_build, or park a question with rex_ask_owner.'

export const CLOSED_BY_MOD = "The mod closed this tree because the master stopped twice with nothing open. The master's last message follows."

function jobLine(job: RexJob): string {
  const merged = job.deliveries.find(delivery => delivery.outcome === 'merged')?.mergeCommit ?? 'none'
  const checks = job.doneChecks.length === 0 ? 'not run' : job.doneChecks.map(check => `${check.check} ${check.result}`).join(', ')
  return `- ${job.id} ${job.status}: ${job.request.split('\n')[0] ?? ''} (merge commit ${merged}; report ${handOffFile(job)}; done check: ${checks}; outcome: ${job.outcome ?? 'none'})`
}

/** The mod's list of every job comes first, so the report names each job whatever the master wrote. */
export function finalReport(book: RexBook, tree: RexTree, report: string): string {
  const jobs = treeJobs(book, tree.id)
  const listed = jobs.length === 0 ? ['- no job was built'] : jobs.map(jobLine)
  return [`# Final report of master tree ${tree.id}`, '', '## Jobs', '', ...listed, '', "## The master's report", '', report.trim() === '' ? '(the master wrote no final message)' : report.trim(), ''].join('\n')
}

/** Writes the final report and closes the tree as done; answers the report's path. */
async function closeWithReport($: EngineInterface, tree: RexTree, report: string): Promise<string> {
  const path = `${await realRoot($)}/.rex/plans/${tree.id}/final-report.md`
  await $.fs.write(path, finalReport(await readBook($), tree, report))
  await closeTree($, tree.id, 'done', path)
  stopWatchdog(tree.id)
  forgetTree(tree.id)
  return path
}

/** Why the tree may not close yet: a job of it is open, or a question of it waits; null when it may. */
function openWork(book: RexBook, tree: RexTree): string | null {
  const job = treeJobs(book, tree.id).find(isOpen)
  if (job !== undefined) {
    return `job ${job.id} is still open, got status ${job.status}; wait for it, or tell its builder what to do with rex_continue`
  }
  const question = book.questions.find(each => each.tree === tree.id && each.answer === null)
  return question === undefined ? null : `question ${question.id} still waits for the person, got no answer; end your turn, and report after the answer`
}

export async function serveReport($: EngineInterface, event: { readonly agentId?: string }, entry: ToolEntry): Promise<ToolCallResult> {
  const call = await masterCall($, entry, event.agentId)
  if ('refused' in call) {
    return deny(call.refused)
  }
  const report = text(entry.name, event, 'report', 'call it again with the final report as Markdown')
  if (isRefused(report)) {
    await fail($, { kind: 'bad-argument', tool: entry.name, argument: 'report', value: report.refused })
    return deny(report.refused)
  }
  const open = openWork(call.book, call.tree)
  if (open !== null) {
    return deny(`${entry.name}: ${open}`)
  }
  const path = await closeWithReport($, call.tree, report)
  return answer([`tree=${call.tree.id}`, 'state=done', `report=${path}`])
}

/**
 * The master's turn ended. With its tree live, no job open, no question
 * waiting and no wake of the mod on the way, it is woken to report or go on;
 * after that many wakes in a row, the mod closes the tree with its last message.
 */
export async function masterStopped($: EngineInterface, masterId: string, lastMessage: string): Promise<void> {
  const book = await readBook($)
  const tree = treeOfMaster(book, masterId)
  if (tree === undefined || openWork(book, tree) !== null || hasRateWake(tree.id) || isWakeOnTheWay(masterId)) {
    return
  }
  if (idleWakesOf(tree.id) >= IDLE_WAKES) {
    await closeWithReport($, tree, `${CLOSED_BY_MOD}\n\n${lastMessage}`)
    return
  }
  countIdleWake(tree.id)
  wakeLater($, masterId, IDLE_TEXT)
}

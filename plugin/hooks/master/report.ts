// How a tree ends: the master calls rex_report once no job is open and no question waits, and the mod writes the final report after its own list of the jobs.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob, RexTree } from '../../types'
import { closeTree, fail, isOpen, readBook } from '../book/index'
import type { ToolEntry } from '../parts'
import { answer, deny } from '../tools/index'
import { isRefused, text } from './args'
import { handOffFile } from './brief'
import { treeJobs } from './gates'
import { forgetTree } from './rate'
import { realRoot } from './store'
import { masterCall } from './tools'
import { stopWatchdog } from './watch'

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

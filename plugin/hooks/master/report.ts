// When a master ends its last turn with every job closed, the mod writes its final report beside the jobs' own reports and closes the tree.
import type { EngineInterface } from 'claude-code'

import type { RexBook, RexJob, RexTree } from '../../types'
import { closeTree, isOpen, readBook } from '../book/index'
import { handOffFile } from './brief'
import { treeJobs } from './gates'
import { realRoot } from './store'
import { stopWatchdog } from './watch'

function jobLine(job: RexJob): string {
  const merged = job.deliveries.find(delivery => delivery.outcome === 'merged')?.mergeCommit ?? 'none'
  const checks = job.doneChecks.length === 0 ? 'not run' : job.doneChecks.map(check => `${check.check} ${check.result}`).join(', ')
  return `- ${job.id} ${job.status}: ${job.request.split('\n')[0] ?? ''} (merge commit ${merged}; report ${handOffFile(job)}; done check: ${checks}; outcome: ${job.outcome ?? 'none'})`
}

/** The mod's list of every job comes first, so the report names each job whatever the master wrote. */
export function finalReport(book: RexBook, tree: RexTree, answer: string): string {
  const jobs = treeJobs(book, tree.id)
  const listed = jobs.length === 0 ? ['- no job was built'] : jobs.map(jobLine)
  return [`# Final report of master tree ${tree.id}`, '', '## Jobs', '', ...listed, '', "## The master's report", '', answer.trim() === '' ? '(the master wrote no final message)' : answer.trim(), ''].join('\n')
}

/** Null when the tree is not done yet: it is not live, or a job of it is still open. */
export async function closeIfDone($: EngineInterface, masterId: string, answer: string): Promise<string | null> {
  const book = await readBook($)
  const tree = book.trees.find(each => each.master === masterId && (each.state === 'running' || each.state === 'paused'))
  if (tree === undefined || treeJobs(book, tree.id).some(isOpen)) {
    return null
  }
  const path = `${await realRoot($)}/.rex/plans/${tree.id}/final-report.md`
  await $.fs.write(path, finalReport(book, tree, answer))
  await closeTree($, tree.id, 'done', path)
  stopWatchdog(tree.id)
  return path
}

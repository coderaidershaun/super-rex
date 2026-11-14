// The lines the master module adds to rex_status: parked questions and their answers, the tree and its jobs, and the done check of a delivered job.
import type { RexBook, RexJob, RexQuestion, RexTree } from '../../types'
import type { StatusContext, StatusSection } from '../parts'
import { usd } from '../tools/index'
import { handOffFile } from './brief'
import { treeJobs, treeSpentNano } from './gates'
import { treeOfMaster } from './pin'

function questionLines(questions: readonly RexQuestion[]): readonly string[] {
  return questions.flatMap(question =>
    question.answer === null ? [`question.${question.id}=parked`] : [`question.${question.id}=answered`, `answer.${question.id}=${question.answer.option}`],
  )
}

/** A master reads the questions of its whole tree; anyone asking about a job reads that job's questions. */
function questionsOf({ book, job, caller }: StatusContext): readonly RexQuestion[] {
  if (job !== null) {
    return book.questions.filter(question => question.job === job.id || question.mergeFor === job.id)
  }
  const tree = caller.kind === 'master' ? treeOfMaster(book, caller.agentId) : undefined
  return tree === undefined ? [] : book.questions.filter(question => question.tree === tree.id)
}

function treeLines(book: RexBook, tree: RexTree): readonly string[] {
  const jobs = treeJobs(book, tree.id).flatMap(job => {
    const builder = tree.builders.find(each => each.job === job.id)?.agentId ?? 'none'
    return [`job.${job.id}.status=${job.status}`, `job.${job.id}.builder=${builder}`]
  })
  const notes = tree.notes.map((note, index) => `note.${String(index + 1)}=${note.job === null ? '' : `${note.job}: `}${note.text.replace(/\n/g, ' ')}`)
  return [
    `tree=${tree.id}`,
    `tree_state=${tree.state}`,
    `tree_spent_usd=${usd(treeSpentNano(book, tree))}`,
    `tree_cap_usd=${usd(tree.capNano)}`,
    ...jobs,
    `notes=${String(tree.notes.length)}`,
    ...notes,
  ]
}

function jobTree(book: RexBook, job: RexJob): RexTree | undefined {
  return job.tree === null ? undefined : book.trees.find(tree => tree.id === job.tree)
}

function doneLines(job: RexJob): readonly string[] {
  const merged = job.deliveries.find(delivery => delivery.outcome === 'merged' && delivery.mergeCommit !== null)
  const handOff = merged === undefined ? [] : [`merge_commit=${merged.mergeCommit ?? ''}`, `output=${handOffFile(job)}`]
  return [...handOff, ...job.doneChecks.map(check => `done.${check.check}=${check.result}`)]
}

export const STATUS_SECTIONS: readonly StatusSection[] = [
  { name: 'questions', order: 40, lines: ctx => questionLines(questionsOf(ctx)) },
  {
    name: 'tree',
    order: 50,
    lines: ({ book, job, caller }) => {
      const tree = job === null ? (caller.kind === 'master' ? treeOfMaster(book, caller.agentId) : undefined) : jobTree(book, job)
      if (tree === undefined) {
        return []
      }
      return job === null ? treeLines(book, tree) : [`tree=${tree.id}`]
    },
  },
  { name: 'done', order: 60, lines: ({ job }) => (job === null ? [] : doneLines(job)) },
]

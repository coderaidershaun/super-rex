// The lines the master module adds to rex_status: parked questions and their answers, the tree and its jobs for its master and for the main loop, the policy a master works under, and the report and done check of a tree job.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob, RexQuestion, RexTree } from '../../types'
import type { StatusContext, StatusSection } from '../parts'
import { callerOf, usd } from '../tools/index'
import { handOffFile } from './brief'
import { treeJobs, treeSpentNano } from './gates'
import { liveTrees, treeOfMaster } from './pin'
import { masterPolicyLines, PolicyShapeError } from './policy'
import { readPolicy } from './store'

function questionLines(questions: readonly RexQuestion[]): readonly string[] {
  return questions.flatMap(question =>
    question.answer === null ? [`question.${question.id}=parked`] : [`question.${question.id}=answered`, `answer.${question.id}=${question.answer.option}`],
  )
}

/** The trees a call with no job reads: its own tree for a master, every live tree of the session for the main loop with no job of its own. */
function callerTrees({ book, caller }: StatusContext): readonly RexTree[] {
  if (caller.kind === 'master') {
    const tree = treeOfMaster(book, caller.agentId)
    return tree === undefined ? [] : [tree]
  }
  return caller.kind === 'main' ? liveTrees(book) : []
}

/** Anyone asking about a job reads that job's questions; else the questions of each tree it reads. */
function questionsOf(ctx: StatusContext): readonly RexQuestion[] {
  const job = ctx.job
  if (job !== null) {
    return ctx.book.questions.filter(question => question.job === job.id || question.mergeFor === job.id)
  }
  const trees = callerTrees(ctx).map(tree => tree.id)
  return ctx.book.questions.filter(question => question.tree !== null && trees.includes(question.tree))
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

/** A tree job always names its report, which the master reads whatever the merge; a job of its own gets it once merged. */
function doneLines(job: RexJob): readonly string[] {
  const merged = job.deliveries.find(delivery => delivery.outcome === 'merged' && delivery.mergeCommit !== null)
  const commit = merged === undefined ? [] : [`merge_commit=${merged.mergeCommit ?? ''}`]
  const output = merged === undefined && job.tree === null ? [] : [`output=${handOffFile(job)}`]
  return [...commit, ...output, ...job.doneChecks.map(check => `done.${check.check}=${check.result}`)]
}

export const STATUS_SECTIONS: readonly StatusSection[] = [
  { name: 'questions', order: 40, lines: ctx => questionLines(questionsOf(ctx)) },
  {
    name: 'tree',
    order: 50,
    lines: ctx => {
      if (ctx.job === null) {
        return callerTrees(ctx).flatMap(tree => treeLines(ctx.book, tree))
      }
      const tree = jobTree(ctx.book, ctx.job)
      return tree === undefined ? [] : [`tree=${tree.id}`]
    },
  },
  { name: 'done', order: 60, lines: ({ job }) => (job === null ? [] : doneLines(job)) },
]

/**
 * The master's rex_status ends with the policy it works under, one
 * `policy.<key>=` line each. The policy lives in the store, which a status
 * section cannot read, so this hook adds the lines to the answer.
 */
export async function withPolicyLines<E extends { readonly agentId?: string }>($: EngineInterface, event: E, next: (e: E) => Promise<ToolCallResult>): Promise<ToolCallResult> {
  const result = await next(event)
  if (typeof result.result !== 'string' || (await callerOf($, event.agentId)).kind !== 'master') {
    return result
  }
  let stored
  try {
    stored = await readPolicy($)
  } catch (error) {
    if (error instanceof PolicyShapeError) {
      return { ...result, result: `${result.result}policy=unreadable\n` }
    }
    throw error
  }
  const lines = stored === null ? ['policy=none'] : masterPolicyLines(stored.policy).map(line => `policy.${line}`)
  return { ...result, result: `${result.result}${lines.map(line => `${line}\n`).join('')}` }
}

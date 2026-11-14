// raise_limits of the policy: a tree job at its spend limit gets the limit doubled once by policy, or by the person's answer to one spend question, never past the tree's cap.
import type { EngineInterface } from 'claude-code'

import type { RexBook, RexJob, RexQuestion, RexTree } from '../../types'
import { addQuestion, isOpen, noteTree, readBook, setLimit } from '../book/index'
import type { Policy } from './policy'
import { CONTINUE_TEXT, wakeLater } from './wake'

export const RAISE = 'Raise'
export const KEEP_LIMIT = 'Keep the limit'
/** Every raise is a tree note that starts so; `allowed` raises a job once, so its note is how the mod knows it did. */
const RAISED_NOTE = 'rex: raised the spend limit of this job'

function dollars(nano: number): string {
  return `$${(nano / 1e9).toFixed(2)}`
}

/** The limit after one raise: double, but never past the tree's cap; null when no raise is left under the cap. */
function raisedNano(job: RexJob, tree: RexTree): number | null {
  const raised = Math.min(job.limit.nano * 2, tree.capNano)
  return raised > job.limit.nano ? raised : null
}

/** The question for one limit: its text names the limit, so a raised limit gets its own question. */
export function spendQuestionText(job: RexJob, raised: number): string {
  return `Job ${job.id} reached its spend limit of ${dollars(job.limit.nano)}. Raise it to ${dollars(raised)}?`
}

function wasRaisedByPolicy(tree: RexTree, job: RexJob): boolean {
  return tree.notes.some(note => note.job === job.id && note.text.startsWith(RAISED_NOTE) && note.text.endsWith('by policy raise_limits allowed'))
}

async function raise($: EngineInterface, tree: RexTree, job: RexJob, raised: number, source: 'policy' | 'set', why: string): Promise<void> {
  await setLimit($, job.id, { nano: raised, source })
  await noteTree($, tree.id, { job: job.id, text: `${RAISED_NOTE} from ${dollars(job.limit.nano)} to ${dollars(raised)} ${why}` })
  const builder = tree.builders.find(each => each.job === job.id)?.agentId
  if (builder !== undefined) {
    wakeLater($, builder, `rex: the spend limit of job ${job.id} is now ${dollars(raised)}. ${CONTINUE_TEXT(job.id)}`)
  }
}

/** One open job of a live tree: at its limit, the policy raises it, asks the person once, or leaves it. */
export async function raiseAtLimit($: EngineInterface, book: RexBook, tree: RexTree, jobId: string, policy: Policy): Promise<void> {
  // The job as the book holds it now: an answer of the person can have raised its limit since the tick began.
  const job = book.jobs.find(each => each.id === jobId)
  if (job === undefined || !isOpen(job) || job.spentNano < job.limit.nano) {
    return
  }
  const raised = raisedNano(job, tree)
  if (raised === null) {
    return
  }
  switch (policy.raise_limits) {
    case 'never':
      return
    case 'allowed':
      if (!wasRaisedByPolicy(tree, job)) {
        await raise($, tree, job, raised, 'policy', 'by policy raise_limits allowed')
      }
      return
    case 'ask': {
      const text = spendQuestionText(job, raised)
      const asked = book.questions.find(question => question.job === job.id && question.question === text)
      if (asked === undefined) {
        await addQuestion($, { tree: tree.id, job: job.id, question: text, options: [RAISE, KEEP_LIMIT], recommendation: KEEP_LIMIT, category: 'spend', mergeFor: null })
      } else if (asked.answer?.option === RAISE) {
        // An answer from the pane reaches only the book, so the tick raises for it.
        await raise($, tree, job, raised, 'set', `by the answer Raise to question ${asked.id}`)
      }
      return
    }
    default: {
      const unknown: never = policy.raise_limits
      throw new Error(`raise_limits ${String(unknown)} has no rule`)
    }
  }
}

/** The person answered a spend question with /rex answer: `Raise` raises the limit its text named at once, if the job still has that limit. */
export async function answeredSpend($: EngineInterface, question: RexQuestion, option: string): Promise<void> {
  if (question.category !== 'spend' || option !== RAISE || question.job === null) {
    return
  }
  const book = await readBook($)
  const job = book.jobs.find(each => each.id === question.job)
  const tree = book.trees.find(each => each.id === question.tree)
  const raised = job === undefined || tree === undefined ? null : raisedNano(job, tree)
  if (job === undefined || tree === undefined || raised === null || spendQuestionText(job, raised) !== question.question) {
    return
  }
  await raise($, tree, job, raised, 'set', `by the answer Raise to question ${question.id}`)
}

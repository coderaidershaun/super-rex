// A tree job at its spend limit: raise_limits of the policy doubles the limit once, or the person's answer to one spend question does, never past the tree's cap; a limit the person set themselves is taken as it is, and each raise wakes the job's builder.
import type { EngineInterface } from 'claude-code'

import type { RexBook, RexJob, RexQuestion, RexTree } from '../../types'
import { isOpen, noteTree, readBook, setLimit } from '../book/index'
import { higherLimitInStore } from '../tools/index'
import { parkQuestion, settleMoot } from './ask'
import { isLiveTree } from './pin'
import type { Policy } from './policy'
import { CONTINUE_TEXT, wakeLater } from './wake'

export const RAISE = 'Raise'
export const KEEP_LIMIT = 'Keep the limit'
/** Every raise is a tree note that starts so; `allowed` raises a job once, so its note is how the mod knows it did. */
const RAISED_NOTE = 'rex: raised the spend limit of this job'

function dollars(nano: number): string {
  return `$${(nano / 1e9).toFixed(2)}`
}

/** The limit stays: this note is how the mod knows it told the builder, so it tells it once for each limit. */
const KEPT_NOTE = 'rex: kept the spend limit of this job'

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

/** The tree notes the raise, and the builder that stopped at the old limit goes on. */
async function noteAndWake($: EngineInterface, tree: RexTree, job: RexJob, raised: number, why: string): Promise<void> {
  await noteTree($, tree.id, { job: job.id, text: `${RAISED_NOTE} from ${dollars(job.limit.nano)} to ${dollars(raised)} ${why}` })
  const builder = tree.builders.find(each => each.job === job.id)?.agentId
  if (builder !== undefined) {
    wakeLater($, builder, `rex: the spend limit of job ${job.id} is now ${dollars(raised)}. ${CONTINUE_TEXT(job.id)}`)
  }
}

/**
 * The limit stays where it is, so the job cannot go on: its builder is told
 * once to close it `blocked`, which lets the master rebuild the step or give
 * it up and lets the tree end. The person reads why in one status line.
 */
async function keepLimit($: EngineInterface, tree: RexTree, job: RexJob, why: string): Promise<void> {
  const note = `${KEPT_NOTE} at ${dollars(job.limit.nano)}: ${why}`
  if (tree.notes.some(each => each.job === job.id && each.text === note)) {
    return
  }
  await noteTree($, tree.id, { job: job.id, text: note })
  $.ui.status(`rex: job ${job.id} stops at its spend limit of ${dollars(job.limit.nano)}: ${why}; raise it with /rex limit ${job.id} <usd>`)
  const builder = tree.builders.find(each => each.job === job.id)?.agentId
  if (builder !== undefined) {
    wakeLater($, builder, `rex: the spend limit of job ${job.id} stays at ${dollars(job.limit.nano)}: ${why}. Close your job with rex_job_close, status blocked.`)
  }
}

async function raise($: EngineInterface, tree: RexTree, job: RexJob, raised: number, source: 'policy' | 'set', why: string): Promise<void> {
  await setLimit($, job.id, { nano: raised, source })
  await noteAndWake($, tree, job, raised, why)
}

/**
 * The person set a job's limit themselves, with /rex limit or at a shell:
 * `before` is the job as it was. In a live tree of this session, a spend
 * question the new limit answers is closed, and a builder that stood at the
 * old limit is woken to go on. A limit set on a job under its limit wakes
 * nobody: its builder works.
 */
export async function limitSetByPerson($: EngineInterface, before: RexJob, how: string): Promise<void> {
  const book = await readBook($)
  const job = book.jobs.find(each => each.id === before.id)
  const tree = book.trees.find(each => each.id === before.tree && each.session === book.session && isLiveTree(each))
  if (job === undefined || tree === undefined) {
    return
  }
  await settleMoot($, tree.id)
  if (isOpen(job) && before.spentNano >= before.limit.nano && job.spentNano < job.limit.nano) {
    await noteAndWake($, tree, before, job.limit.nano, how)
  }
}

/** One open job of a live tree: at its limit, the policy raises it, asks the person once, or leaves it. */
export async function raiseAtLimit($: EngineInterface, book: RexBook, tree: RexTree, jobId: string, policy: Policy): Promise<void> {
  // The job as the book holds it now: an answer of the person can have raised its limit since the tick began.
  const job = book.jobs.find(each => each.id === jobId)
  if (job === undefined || !isOpen(job) || job.spentNano < job.limit.nano) {
    return
  }
  // A limit the person set at a shell reaches only the store, so the tick takes it before the policy decides.
  const stored = await higherLimitInStore($, book, job)
  if (stored !== null) {
    await setLimit($, job.id, { nano: stored, source: 'set' })
    await limitSetByPerson($, job, 'by the person, with rex limit at a shell')
    return
  }
  const raised = raisedNano(job, tree)
  if (raised === null) {
    await keepLimit($, tree, job, `the cap of the tree, ${dollars(tree.capNano)}, leaves no room to raise it`)
    return
  }
  switch (policy.raise_limits) {
    case 'never':
      await keepLimit($, tree, job, 'raise_limits of the policy is never')
      return
    case 'allowed':
      if (wasRaisedByPolicy(tree, job)) {
        await keepLimit($, tree, job, 'raise_limits of the policy raises a limit once')
      } else {
        await raise($, tree, job, raised, 'policy', 'by policy raise_limits allowed')
      }
      return
    case 'ask': {
      const text = spendQuestionText(job, raised)
      const asked = book.questions.find(question => question.job === job.id && question.question === text)
      if (asked === undefined) {
        await parkQuestion($, { tree: tree.id, job: job.id, question: text, options: [RAISE, KEEP_LIMIT], recommendation: KEEP_LIMIT, category: 'spend', mergeFor: null })
      } else if (asked.answer?.option === RAISE) {
        // An answer that reached only the book is acted on here.
        await raise($, tree, job, raised, 'set', `by the answer Raise to question ${asked.id}`)
      } else if (asked.answer?.option === KEEP_LIMIT) {
        await keepLimit($, tree, job, `the person answered ${KEEP_LIMIT} to question ${asked.id}`)
      }
      return
    }
    default: {
      const unknown: never = policy.raise_limits
      throw new Error(`raise_limits ${String(unknown)} has no rule`)
    }
  }
}

/** The person answered a spend question, in the dialog or with /rex answer: at once `Raise` raises the limit its text named, and `Keep the limit` has the builder close the job blocked, if the job still has that limit. */
export async function answeredSpend($: EngineInterface, question: RexQuestion, option: string): Promise<void> {
  if (question.category !== 'spend' || question.job === null || (option !== RAISE && option !== KEEP_LIMIT)) {
    return
  }
  const book = await readBook($)
  const job = book.jobs.find(each => each.id === question.job)
  const tree = book.trees.find(each => each.id === question.tree)
  const raised = job === undefined || tree === undefined ? null : raisedNano(job, tree)
  if (job === undefined || tree === undefined || raised === null || spendQuestionText(job, raised) !== question.question) {
    return
  }
  if (option === RAISE) {
    await raise($, tree, job, raised, 'set', `by the answer Raise to question ${question.id}`)
  } else {
    await keepLimit($, tree, job, `the person answered ${KEEP_LIMIT} to question ${question.id}`)
  }
}

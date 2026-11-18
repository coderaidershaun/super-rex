// What keeps a tree job from waiting for ever on a builder that cannot go on: the question the mod asks about it, what each answer of the person sets going (the same from the dialog and from /rex answer), and the builder that a wake could not reach.
import type { EngineInterface } from 'claude-code'

import type { RexJob, RexQuestion, RexTree } from '../../types'
import { closeJob, errorText, isOpen, noteTree, readBook, resetWakes } from '../book/index'
import { parkQuestion, wakeOnAnswer } from './ask'
import { restartBuilder } from './build'
import { answeredSpend } from './limits'
import { isLiveTree } from './pin'
import { DEFAULT_POLICY } from './policy'
import type { Policy } from './policy'
import { readPolicy } from './store'

/** The options of the two questions the mod asks about a job whose builder cannot go on. */
export const START_AGAIN = 'Start a new builder'
export const BUILD_AGAIN = 'Build it again'
export const LEAVE_BLOCKED = 'Leave it blocked'

const GONE_START = 'The builder of job '
const BLOCKED_START = 'Job '

/** The question of a resume, for a job whose builder ended with the last session. */
export function goneQuestionText(job: string): string {
  return `${GONE_START}${job} ended with the last session. Start a new builder for it?`
}

/** A job no wake helped waits on the person, and its builder is not woken again until they answer. A job that already has such a question waiting gets no second one. */
export async function blockAndAsk($: EngineInterface, tree: RexTree, job: RexJob, why: string): Promise<void> {
  const isAsked = (await readBook($)).questions.some(question => question.job === job.id && question.answer === null && isBuilderQuestion(question))
  if (isAsked) {
    return
  }
  await parkQuestion($, {
    tree: tree.id,
    job: job.id,
    question: `${BLOCKED_START}${job.id} is blocked: ${why}. What should the master do?`,
    options: [BUILD_AGAIN, LEAVE_BLOCKED],
    recommendation: LEAVE_BLOCKED,
    category: 'other',
    mergeFor: null,
  })
}

/** True for one of the mod's two questions about a builder, which carry the options the mod acts on. */
function isBuilderQuestion(question: RexQuestion): boolean {
  const [first, second, ...rest] = question.options
  const isAgain = (first === START_AGAIN && question.question.startsWith(GONE_START)) || (first === BUILD_AGAIN && question.question.startsWith(BLOCKED_START))
  return question.category === 'other' && question.job !== null && isAgain && second === LEAVE_BLOCKED && rest.length === 0
}

async function policyNow($: EngineInterface): Promise<Policy> {
  try {
    return (await readPolicy($))?.policy ?? DEFAULT_POLICY
  } catch (error) {
    $.ui.status(`rex: the policy could not be read, so a new builder starts under the defaults: ${errorText(error)}`)
    return DEFAULT_POLICY
  }
}

/**
 * A new builder for an open job, in the job's own worktree, that goes on from
 * the job's record. The job's counts of wakes start from nothing, so the
 * watchdog gives the new builder the same patience as the first. When no
 * builder starts, the job stays as it is, and the person and the tree read why.
 */
export async function startAgain($: EngineInterface, tree: RexTree, job: RexJob, why: string): Promise<boolean> {
  await resetWakes($, job.id, 'stuck')
  await resetWakes($, job.id, 'retry')
  const restarted = await restartBuilder($, job, tree, await policyNow($)).catch((error: unknown) => ({ refused: errorText(error) }))
  if ('refused' in restarted) {
    $.ui.status(`rex: job ${job.id} of tree ${tree.id} got no new builder: ${restarted.refused}`)
    await noteTree($, tree.id, { job: job.id, text: `rex: no new builder started for this job (${why}): ${restarted.refused}` })
    return false
  }
  await noteTree($, tree.id, { job: job.id, text: `rex: started a new builder ${restarted.agentId} for this job (${why})` })
  return true
}

/** The person answered one of the mod's questions about a builder: a new builder goes on with the job, or the job closes blocked so the tree can end. */
async function answeredBuilder($: EngineInterface, question: RexQuestion, option: string): Promise<void> {
  if (!isBuilderQuestion(question)) {
    return
  }
  const book = await readBook($)
  const job = book.jobs.find(each => each.id === question.job)
  const tree = book.trees.find(each => each.id === question.tree && each.session === book.session && isLiveTree(each))
  if (job === undefined || tree === undefined || !isOpen(job)) {
    return
  }
  if (option === LEAVE_BLOCKED) {
    await closeJob($, job.id, { status: 'blocked', outcome: `left blocked by the person's answer to question ${question.id}`, deliveries: job.deliveries })
    return
  }
  await startAgain($, tree, job, `the person's answer ${option} to question ${question.id}`)
}

/** Everything an answer of the person sets going, in one order whatever the way it came: the raise, the builder, then the wakes of whoever waits. */
export async function afterAnswer($: EngineInterface, question: RexQuestion, option: string): Promise<void> {
  await answeredSpend($, question, option)
  await answeredBuilder($, question, option)
  await wakeOnAnswer($, question, option)
}

/**
 * A wake did not reach an agent. When it is the builder of an open job of a
 * live tree of this session, the builder is gone, as after the process ended
 * and came back: by `on_dead_builder` a new builder goes on with the job, or
 * one question asks the person. Any other agent only gets its status line.
 */
export async function wakeUndelivered($: EngineInterface, agentId: string, reason: string): Promise<void> {
  $.ui.status(`rex: ${agentId} was not woken: ${reason}`)
  const book = await readBook($)
  const tree = book.trees.find(each => each.session === book.session && isLiveTree(each) && each.builders.some(builder => builder.agentId === agentId))
  const job = book.jobs.find(each => each.id === tree?.builders.find(builder => builder.agentId === agentId)?.job)
  if (tree === undefined || job === undefined || !isOpen(job)) {
    return
  }
  const why = `its builder ${agentId} could not be woken (${reason})`
  if ((await policyNow($)).on_dead_builder === 'continue') {
    await startAgain($, tree, job, why)
    return
  }
  await blockAndAsk($, tree, job, why)
}

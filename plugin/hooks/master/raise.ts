// rex_raise: the master asks the person to raise the tree's budget, change its time, or raise one job's spend limit; the mod parks the request as a question, and only the person's own answer, in the dialog or with /rex answer, sets the new value.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexQuestion } from '../../types'
import { fail, isOpen, readBook, setLimit, setTreeLimits } from '../book/index'
import type { ToolEntry } from '../parts'
import { answer, deny } from '../tools/index'
import { isRefused, optionalText, refused, text, word } from './args'
import { parkQuestion, personNow } from './ask'
import { afterSetByPerson, timeFrom } from './budget'
import { limitSetByPerson } from './limits'
import { isLiveTree } from './pin'
import { masterCall } from './tools'
import { allowedValue, allowOption, BUDGET_FORMS, budgetOf, KEEP_OPTION, moneyText, requestKind, requestText, spanText, TIME_FORMS, timeOf, whenText } from './tree-limits'
import type { RaiseKind } from './tree-limits'

const KINDS: readonly RaiseKind[] = ['budget', 'time', 'job']

/** What one call asks for: the option that allows it and the question's text. */
type Request = { readonly job: string | null; readonly question: string; readonly allow: string }

export async function serveRaise($: EngineInterface, event: { readonly agentId?: string }, entry: ToolEntry): Promise<ToolCallResult> {
  const call = await masterCall($, entry, event.agentId)
  if ('refused' in call) {
    return deny(call.refused)
  }
  const { tree, book } = call
  const kind = word(entry.name, event, 'limit', KINDS)
  const to = text(entry.name, event, 'to', 'call it again with the new value')
  const reason = text(entry.name, event, 'reason', 'call it again with why, in one sentence')
  const jobId = optionalText(entry.name, event, 'job')
  const problem = [kind, to, reason, jobId].find(isRefused) ?? null
  if (problem !== null || isRefused(kind) || isRefused(to) || isRefused(reason) || isRefused(jobId)) {
    await fail($, { kind: 'bad-argument', tool: entry.name, argument: 'arguments', value: problem?.refused ?? '' })
    return deny(problem?.refused ?? `${entry.name}: bad arguments`)
  }
  if (!personNow().isInteractive) {
    return deny(`${entry.name}: only the person raises a limit, got no person at the terminal; go on within the limits, or end the tree with rex_report`)
  }
  let request: Request
  if (kind === 'time') {
    const span = timeOf(to)
    if (span === null) {
      return deny(refused(entry.name, 'to must be a time', to, `call it again with one of ${TIME_FORMS}`).refused)
    }
    const from = tree.deadlineAt === null ? 'unlimited' : `${spanText(tree.timeMs)}, which ends at ${whenText(tree.deadlineAt)}`
    const target = span.timeMs === null ? 'unlimited' : `${spanText(span.timeMs)} counted from your answer`
    request = { job: null, question: requestText(tree.id, kind, from, target, reason, null), allow: allowOption(kind, span.timeMs) }
  } else {
    const nano = budgetOf(to)
    const job = kind === 'job' ? book.jobs.find(each => each.id === jobId && each.tree === tree.id) : undefined
    if (nano === null) {
      return deny(refused(entry.name, 'to must be an amount', to, `call it again with one of ${BUDGET_FORMS}`).refused)
    }
    if (kind === 'job' && (job === undefined || !isOpen(job))) {
      return deny(refused(entry.name, 'job must be an open job of your tree', jobId, 'call it again with the id of an open job').refused)
    }
    const now = job === undefined ? tree.capNano : job.limit.nano
    if (nano <= now) {
      return deny(refused(entry.name, `to must be more than the ${kind === 'job' ? 'limit' : 'budget'} now, ${moneyText(now)}`, to, 'call it again with a higher amount').refused)
    }
    request = { job: job?.id ?? null, question: requestText(tree.id, kind, moneyText(now), moneyText(nano), reason, job?.id ?? null), allow: allowOption(kind, nano) }
  }
  // One request of a kind waits at a time, and a value the person already kept is not asked for again.
  const same = book.questions.filter(question => question.tree === tree.id && question.job === request.job && requestKind(question) === kind)
  const waiting = same.find(question => question.answer === null)
  if (waiting !== undefined) {
    return answer([`question=${waiting.id}`, 'state=waiting'])
  }
  if (same.some(question => question.options[0] === request.allow && question.answer?.option === KEEP_OPTION[kind])) {
    return deny(`${entry.name}: the person already answered ${KEEP_OPTION[kind]} to this request, got ${to}; do not ask again: go on within the limits`)
  }
  const parked = await parkQuestion($, { tree: tree.id, job: request.job, question: request.question, options: [request.allow, KEEP_OPTION[kind]], recommendation: KEEP_OPTION[kind], category: 'spend', mergeFor: null })
  return answer([`question=${parked.id}`, 'state=parked'])
}

/**
 * The person answered a request of the master, in the dialog or with /rex
 * answer. The allowing option sets the value it names: the tree's budget, its
 * time counted from now, or the job's limit. The keeping option changes
 * nothing; the master reads either answer when it is woken.
 */
export async function answeredRequest($: EngineInterface, question: RexQuestion, option: string): Promise<void> {
  const kind = requestKind(question)
  const allowed = allowedValue(question, option)
  if (kind === null || allowed === null) {
    return
  }
  const book = await readBook($)
  const tree = book.trees.find(each => each.id === question.tree && each.session === book.session && isLiveTree(each))
  if (tree === undefined) {
    return
  }
  const by = `by the answer ${option} to the master's request, question ${question.id}`
  if ('timeMs' in allowed) {
    const time = timeFrom(allowed.timeMs, await $.clock.now())
    await setTreeLimits($, tree.id, { time })
    const what = time.deadlineAt === null ? `the person took the time limit off this tree, ${by}` : `the person gave this tree ${spanText(time.timeMs)} from now, until ${whenText(time.deadlineAt)}, ${by}`
    await afterSetByPerson($, book, tree, what, false)
    return
  }
  if (kind === 'budget') {
    await setTreeLimits($, tree.id, { capNano: allowed.nano })
    await afterSetByPerson($, book, tree, `the person raised the budget of this tree from ${moneyText(tree.capNano)} to ${moneyText(allowed.nano)}, ${by}`, false)
    return
  }
  const job = book.jobs.find(each => each.id === question.job)
  if (job !== undefined && isOpen(job)) {
    await setLimit($, job.id, { nano: allowed.nano, source: 'set' })
    await limitSetByPerson($, job, by)
  }
}

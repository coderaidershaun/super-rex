// The budget and the time of a master tree in action: the two questions the person answers when a tree opens, the one question the mod asks when the tree reaches either, what each answer sets going, and /rex budget and /rex time.
import type { CommandRunInput, CommandRunResult, EngineInterface } from 'claude-code'

import type { RexBook, RexQuestion, RexTree } from '../../types'
import { fail, isOpen, mainTree, noteTree, readBook, setTreeLimits, tickTree } from '../book/index'
import { originRefusal, rexLine } from '../commands/index'
import { usd } from '../tools/index'
import { parkQuestion, personNow, settleMoot } from './ask'
import { BUDGET_SUBCOMMAND, TIME_SUBCOMMAND } from './entries'
import { treeJobs, treeSpentNano } from './gates'
import { isLiveTree } from './pin'
import type { Policy } from './policy'
import {
  BUDGET_FORMS,
  BUDGET_QUESTION,
  BUDGET_TIERS,
  budgetLabel,
  budgetOf,
  goOnOption,
  isTreeLimitQuestion,
  limitCommands,
  limitQuestionText,
  moneyText,
  overWords,
  raisedCapNano,
  spanText,
  STOP_TREE,
  TIME_FORMS,
  TIME_QUESTION,
  TIME_TIERS,
  timeLabel,
  timeOf,
  treeOver,
  whenText,
} from './tree-limits'
import type { TreeOver } from './tree-limits'
import { CONTINUE_TEXT, wakeLater, wakeMaster } from './wake'

const SKILL = '/super-rex:orchestration-master'

/** What the person gave a tree when it opened. */
export type OpenLimits = { readonly capNano: number; readonly timeMs: number | null }

/** The person's choice in one dialog, or null when they dismissed it. */
async function chosen($: EngineInterface, question: string, header: string, options: readonly string[]): Promise<string | null> {
  try {
    return await $.ui.ask(question, { options, header })
  } catch (error) {
    if (error instanceof Error) {
      return null
    }
    throw error
  }
}

/**
 * The budget and the time of a tree that opens. With a person at the
 * terminal, two dialogs ask for them, each with four options and free text
 * for another amount or span. With nobody there, the tree gets `tree_usd` of
 * the policy and no time limit. A text is the refusal: nothing opens with no
 * budget of the person's own choice.
 */
export async function openLimits($: EngineInterface, isInteractive: boolean, policy: Policy): Promise<OpenLimits | string> {
  if (!isInteractive) {
    return { capNano: Math.round(policy.tree_usd * 1e9), timeMs: null }
  }
  const budget = await chosen($, BUDGET_QUESTION, 'Budget', BUDGET_TIERS.map(budgetLabel))
  const capNano = budget === null ? null : budgetOf(budget)
  if (capNano === null) {
    return `rex_tree_open: the person chose no budget for the tree, got ${budget ?? 'a dismissed question'}; call rex_tree_open again, and the person chooses one of ${BUDGET_FORMS}`
  }
  const time = await chosen($, TIME_QUESTION, 'Time', TIME_TIERS.map(timeLabel))
  const span = time === null ? null : timeOf(time)
  if (span === null) {
    return `rex_tree_open: the person chose no time for the tree, got ${time ?? 'a dismissed question'}; call rex_tree_open again, and the person chooses one of ${TIME_FORMS}`
  }
  return { capNano, timeMs: span.timeMs }
}

/** The time of a tree as the book holds it, counted from `now`. */
export function timeFrom(timeMs: number | null, now: number): { readonly timeMs: number | null; readonly deadlineAt: number | null } {
  return { timeMs, deadlineAt: timeMs === null ? null : now + timeMs }
}

/** The `key=value` lines of a tree's budget and time, for the tool that opens it and the two commands. */
export function limitLines(book: RexBook, tree: RexTree): readonly string[] {
  return [`budget_usd=${usd(tree.capNano)}`, `spent_usd=${usd(treeSpentNano(book, tree))}`, `time=${spanText(tree.timeMs)}`, `ends_at=${tree.deadlineAt === null ? 'none' : whenText(tree.deadlineAt)}`]
}

/** One limit by its own name, which stays the same while the tree spends on: `its budget of $10`. */
export function limitName(over: TreeOver): string {
  return over.kind === 'budget' ? `its budget of ${moneyText(over.capNano)}` : `the end of its time of ${spanText(over.timeMs)}, ${whenText(over.deadlineAt)}`
}

/** A tree that stopped at a limit has this note for it; the note is how the mod knows it told the builders and the master once. */
const STOPPED_NOTE = 'rex: this tree stopped at '

export function isStoppedAt(tree: RexTree, over: TreeOver): boolean {
  return tree.notes.some(note => note.job === null && note.text.startsWith(`${STOPPED_NOTE}${limitName(over)}:`))
}

function liveTree(book: RexBook, id: string | null): RexTree | undefined {
  return book.trees.find(each => each.id === id && each.session === book.session && isLiveTree(each))
}

/**
 * The tree stops at a limit: each builder is told once to close its job
 * `blocked`, and the master to end the tree with its report. The quiet time
 * of each open job starts again, so a builder has two watchdog periods to
 * close its job before the mod closes it.
 */
async function stopTree($: EngineInterface, tree: RexTree, over: TreeOver, why: string): Promise<void> {
  if (isStoppedAt(tree, over)) {
    return
  }
  const name = limitName(over)
  await noteTree($, tree.id, { job: null, text: `${STOPPED_NOTE}${name}: ${why}` })
  $.ui.status(`rex: master tree ${tree.id} stops at ${name}: ${why}`)
  const book = await readBook($)
  for (const job of treeJobs(book, tree.id).filter(isOpen)) {
    await tickTree($, tree.id, { job: job.id, spentNano: job.spentNano })
    const builder = tree.builders.find(each => each.job === job.id)?.agentId
    if (builder !== undefined) {
      wakeLater($, builder, `rex: master tree ${tree.id} stops at ${name}. Close your job ${job.id} with rex_job_close, status blocked, and say what is done and what is left.`)
    }
  }
  const { sections } = personNow()
  if (sections !== null) {
    const todo = 'Each builder was told to close its job blocked. Start no new job. When no job is open, end the tree with rex_report, and say what is done and what is left; give up a job that stays open with rex_cancel'
    wakeMaster($, await readBook($), tree, `the tree stops at ${name}: ${why}. ${todo}`, sections)
  }
}

/**
 * The tree may go on: the tree notes why, each builder that had stopped is
 * woken, and the master too when `tellMaster` says nothing else wakes it. A
 * tree that still stands at a limit wakes nobody: the mod asks about that
 * limit next.
 */
async function goOn($: EngineInterface, treeId: string, what: string, tellMaster: boolean): Promise<void> {
  await noteTree($, treeId, { job: null, text: `rex: ${what}` })
  const book = await readBook($)
  const tree = liveTree(book, treeId)
  if (tree === undefined || treeOver(book, tree, await $.clock.now()) !== null) {
    return
  }
  for (const job of treeJobs(book, tree.id).filter(isOpen)) {
    const builder = tree.builders.find(each => each.job === job.id)?.agentId
    const isWorking = book.runs.some(run => run.job === job.id && run.state === 'open')
    if (builder !== undefined && !isWorking) {
      wakeLater($, builder, `rex: ${what}. ${CONTINUE_TEXT(job.id)}`)
    }
  }
  const { sections } = personNow()
  if (tellMaster && sections !== null) {
    wakeMaster($, book, tree, what, sections)
  }
}

/** What an answer of the person to the mod's question about a limit sets going. */
async function actOn($: EngineInterface, tree: RexTree, over: TreeOver, question: RexQuestion, option: string): Promise<void> {
  if (option === STOP_TREE) {
    await stopTree($, tree, over, `the person answered ${STOP_TREE} to question ${question.id}`)
    return
  }
  if (option !== goOnOption(over)) {
    return
  }
  const by = `by the answer ${option} to question ${question.id}`
  if (over.kind === 'budget') {
    const raised = raisedCapNano(over.capNano)
    await setTreeLimits($, tree.id, { capNano: raised })
    await goOn($, tree.id, `the person raised the budget of this tree from ${moneyText(over.capNano)} to ${moneyText(raised)}, ${by}`, false)
    return
  }
  const time = timeFrom(over.timeMs, await $.clock.now())
  await setTreeLimits($, tree.id, { time })
  await goOn($, tree.id, `the person gave this tree ${spanText(over.timeMs)} more, until ${time.deadlineAt === null ? 'no end' : whenText(time.deadlineAt)}, ${by}`, false)
}

/**
 * A live tree of this session that stands at its budget or its time. The
 * first time, the mod asks the person once: raise it, or stop the tree. With
 * nobody at the terminal the tree stops. An answer that reached only the book
 * is acted on here. Answers the limit, or null for a tree under both.
 * `tellMaster` is false when the caller is the master's own refused call,
 * which reads the same in its refusal.
 */
export async function treeAtLimit($: EngineInterface, treeId: string, tellMaster: boolean): Promise<TreeOver | null> {
  const book = await readBook($)
  const tree = liveTree(book, treeId)
  const over = tree === undefined ? null : treeOver(book, tree, await $.clock.now())
  if (tree === undefined || over === null) {
    return null
  }
  const text = limitQuestionText(tree, over)
  // The newest question with this text: one the mod closed as not needed, when the tree was under the limit for a while, is asked again.
  const asked = book.questions.filter(question => question.tree === tree.id && question.question === text).at(-1)
  const person = personNow()
  if (over.kind === 'budget' && raisedCapNano(over.capNano) <= over.capNano) {
    // At the highest amount a budget may be, there is no raise to offer.
    await stopTree($, tree, over, 'no higher budget can be set')
    return over
  }
  if (asked === undefined || asked.answer?.source === 'mod') {
    if (!person.isInteractive) {
      await stopTree($, tree, over, 'no person is at the terminal to raise it')
      return over
    }
    const question = await parkQuestion($, { tree: tree.id, job: null, question: text, options: [goOnOption(over), STOP_TREE], recommendation: STOP_TREE, category: 'spend', mergeFor: null })
    // The person reads in the transcript every command that changes the limit, not only the dialog.
    for (const line of limitCommands(tree, over, question.id)) {
      $.ui.log(line)
    }
    if (tellMaster && person.sections !== null) {
      const todo = 'Nothing new starts until they answer. Start no new job, and end your turn: the mod wakes you with the answer. IF the person tells you in the conversation to raise it, THEN call rex_raise: the mod asks them to confirm'
      wakeMaster($, await readBook($), tree, `the tree ${overWords(over)}, and the mod asked the person to raise it or stop the tree (question ${question.id}). ${todo}`, person.sections)
    }
    return over
  }
  if (asked.answer !== null) {
    await actOn($, tree, over, asked, asked.answer.option)
  }
  return over
}

/** The person answered the mod's question about a tree's budget or time, in the dialog or with /rex answer: the answer acts at once, if the tree still stands at that limit. */
export async function answeredTreeLimit($: EngineInterface, question: RexQuestion, option: string): Promise<void> {
  if (!isTreeLimitQuestion(question)) {
    return
  }
  const book = await readBook($)
  const tree = liveTree(book, question.tree)
  const over = tree === undefined ? null : treeOver(book, tree, await $.clock.now())
  if (tree === undefined || over === null || limitQuestionText(tree, over) !== question.question) {
    return
  }
  await actOn($, tree, over, question, option)
}

/**
 * After the person set a budget or a time, with a command or by allowing a
 * request of the master: `book` and `tree` are as they were before. The mod's
 * question about the old limit is closed, the tree notes the change, and a
 * tree that stood at a limit goes on.
 */
export async function afterSetByPerson($: EngineInterface, book: RexBook, tree: RexTree, what: string, tellMaster: boolean): Promise<void> {
  const wasOver = treeOver(book, tree, await $.clock.now()) !== null
  await settleMoot($, tree.id)
  if (wasOver) {
    await goOn($, tree.id, what, tellMaster)
  } else {
    await noteTree($, tree.id, { job: null, text: `rex: ${what}` })
  }
}

function shown(book: RexBook, tree: RexTree): CommandRunResult {
  return { text: [`tree=${tree.id}`, ...limitLines(book, tree)].map(row => `${row}\n`).join('') }
}

/**
 * `/rex budget [amount]` and `/rex time [span]`: shows the budget and the
 * time of this session's tree, or sets one of them, from the person's own
 * typing only. A time counts from now. A tree that stood at the limit goes
 * on: its question is closed, and its master and its stopped builders are woken.
 */
export async function serveTreeLimit($: EngineInterface, event: CommandRunInput, which: 'budget' | 'time'): Promise<CommandRunResult> {
  const entry = which === 'budget' ? BUDGET_SUBCOMMAND : TIME_SUBCOMMAND
  const refusal = originRefusal(event, entry, false)
  if (refusal !== null) {
    await fail($, { kind: 'origin-not-person', action: `rex ${which}`, origin: event.origin.kind })
    return { text: refusal }
  }
  const typed = rexLine(event).args
  const book = await readBook($)
  const tree = mainTree(book)
  if (tree === undefined) {
    return { text: `rex ${which}: no master tree runs in this session, got none; the mod asks for a budget and a time when a tree opens with ${SKILL} <goal>` }
  }
  if (typed === '') {
    return shown(book, tree)
  }
  const now = await $.clock.now()
  let what: string
  if (which === 'budget') {
    const capNano = budgetOf(typed)
    if (capNano === null) {
      return { text: `rex budget: ${typed} is no budget, got ${typed}; type one of ${BUDGET_FORMS}` }
    }
    await setTreeLimits($, tree.id, { capNano })
    what = `the person set the budget of this tree to ${moneyText(capNano)}, with /rex budget`
  } else {
    const span = timeOf(typed)
    if (span === null) {
      return { text: `rex time: ${typed} is no time, got ${typed}; type one of ${TIME_FORMS}` }
    }
    const time = timeFrom(span.timeMs, now)
    await setTreeLimits($, tree.id, { time })
    what = time.deadlineAt === null ? 'the person took the time limit off this tree, with /rex time' : `the person gave this tree ${spanText(time.timeMs)} from now, until ${whenText(time.deadlineAt)}, with /rex time`
  }
  await afterSetByPerson($, book, tree, what, true)
  const after = await readBook($)
  return shown(after, liveTree(after, tree.id) ?? tree)
}

// Parks a question for the person the same way for the master and the mod, and wakes whoever waits on its answer: a status line names it and the command that answers it until it is answered, with a dialog when someone is at the terminal.
import type { EngineInterface } from 'claude-code'

import type { RexBook, RexQuestion } from '../../types'
import { addQuestion, answerQuestion, errorText, isOpen, readBook } from '../book/index'
import type { QuestionInput } from '../book/index'
import type { StatusSection, Table } from '../parts'
import { afterAnswer } from './answers'
import { isLiveTree } from './pin'
import { isLimitStillReached, isTreeLimitQuestion, requestKind } from './tree-limits'
import { wakeLater, wakeMaster } from './wake'

/** How `$.ui.ask` rejects when the person dismisses the dialog with no answer. */
const DISMISSED = /\$\.ui\.ask: no answer\b/

/** The question text is cut here in the status line, which has one line of the terminal. */
const STATUS_TEXT_MAX = 80

/** Set at each session start: a dialog needs a person at the terminal, and a woken master reads the status sections. */
const asking: { isInteractive: boolean; sections: Table<StatusSection> | null } = { isInteractive: true, sections: null }

export function setAsking(isInteractive: boolean, sections: Table<StatusSection>): void {
  asking.isInteractive = isInteractive
  asking.sections = sections
}

/** Whether a person is at the terminal to answer a dialog, and the status sections a woken master reads. */
export function personNow(): { readonly isInteractive: boolean; readonly sections: Table<StatusSection> | null } {
  return asking
}

export function questionStatus(question: RexQuestion): string {
  const text = question.question.length > STATUS_TEXT_MAX ? `${question.question.slice(0, STATUS_TEXT_MAX)}…` : question.question
  return `rex: question ${question.id} waits: ${text} Answer with /rex answer ${question.id} <1-${String(question.options.length)}>`
}

/** The status line once a question is answered: the oldest question that still waits, else none. */
function statusAfterAnswer(book: RexBook): string | undefined {
  const waiting = book.questions.filter(each => each.answer === null).sort((a, b) => a.askedAt - b.askedAt)[0]
  return waiting === undefined ? undefined : questionStatus(waiting)
}

/**
 * After the person's answer, by the dialog, the pane or /rex answer: the status
 * line stops saying the question waits, the builder of a merge question calls
 * rex_merge again by itself, and the master of a live tree reads the tree as it
 * is now.
 */
export async function wakeOnAnswer($: EngineInterface, question: RexQuestion, option: string): Promise<void> {
  const book = await readBook($)
  $.ui.status(statusAfterAnswer(book))
  const tree = book.trees.find(each => each.id === question.tree && isLiveTree(each))
  if (tree === undefined) {
    return
  }
  const builder = question.mergeFor === null ? undefined : tree.builders.find(each => each.job === question.mergeFor)?.agentId
  if (builder !== undefined) {
    wakeLater($, builder, `rex: the person answered question ${question.id}: ${option}. Call rex_merge again.`)
  }
  if (asking.sections !== null) {
    wakeMaster($, book, tree, `question ${question.id} is answered: ${option}`, asking.sections)
  }
}

/** The dialog runs from a timer, so the call that parked the question returns at once; an answer given elsewhere first wins. */
function askLater($: EngineInterface, question: RexQuestion): void {
  $.clock.after(0, () => {
    void $.ui
      .ask(question.question, question.options)
      .then(async chosen => {
        const option = question.options.find(each => each === chosen)
        const now = (await readBook($)).questions.find(each => each.id === question.id)
        if (option === undefined || now?.answer !== null) {
          return
        }
        await answerQuestion($, question.id, { option, source: 'dialog' })
        await afterAnswer($, question, option)
      })
      .catch((error: unknown) => {
        // A dialog the person dismissed is no failure: the question waits for its command.
        const text = errorText(error)
        $.ui.status(DISMISSED.test(text) ? questionStatus(question) : `${questionStatus(question)} (the dialog failed: ${text})`)
      })
  })
}

/** Shows a question already in the book to the person: its status line, and the dialog when someone is at the terminal. */
export function announceQuestion($: EngineInterface, question: RexQuestion): void {
  $.ui.status(questionStatus(question))
  if (asking.isInteractive) {
    askLater($, question)
  }
}

export async function parkQuestion($: EngineInterface, input: QuestionInput): Promise<RexQuestion> {
  const parked = await addQuestion($, input)
  announceQuestion($, parked)
  return parked
}

/**
 * The mod's question about a tree's budget or time, and a request of the
 * master to raise a limit, hold no tree open once every job of it closed: the
 * master ends the tree with its report, and the question is answered `Not needed`.
 */
export async function settleLimitQuestions($: EngineInterface, treeId: string): Promise<void> {
  const book = await readBook($)
  if (book.jobs.some(job => job.tree === treeId && isOpen(job))) {
    return
  }
  const waiting = book.questions.filter(question => question.tree === treeId && question.answer === null && (isTreeLimitQuestion(question) || requestKind(question) !== null))
  for (const question of waiting) {
    await answerQuestion($, question.id, { option: NOT_NEEDED, source: 'mod' })
  }
  if (waiting.length > 0) {
    $.ui.status(statusAfterAnswer(await readBook($)))
  }
}

/** The answer the mod gives a question that nobody needs to answer any more. */
export const NOT_NEEDED = 'Not needed'

/** True for a question whose answer could still change something: a question about a tree's budget or time while the tree stands at that limit, a spend question while its job is at its limit, any other while its job is open. */
function isStillNeeded(book: RexBook, question: RexQuestion, now: number): boolean {
  if (isTreeLimitQuestion(question)) {
    return isLimitStillReached(book, question, now)
  }
  if (requestKind(question) !== null) {
    // A request of the master to raise a limit waits for the person whether or not the limit is reached yet.
    const asked = book.jobs.find(each => each.id === question.job)
    return asked === undefined || isOpen(asked)
  }
  const job = book.jobs.find(each => each.id === (question.mergeFor ?? question.job))
  if (job === undefined) {
    return true
  }
  return isOpen(job) && (question.category !== 'spend' || job.spentNano >= job.limit.nano)
}

/**
 * Closes each waiting question of a tree that nothing waits on any more: its
 * job closed, its job is under its spend limit again because the person set a
 * higher one, or its tree is under its budget and its time again. The mod answers it `Not needed`, so it stops showing as
 * waiting, stops holding the watchdog off its job, and no longer keeps
 * rex_report from closing the tree. It wakes nobody.
 */
export async function settleMoot($: EngineInterface, treeId: string): Promise<void> {
  const book = await readBook($)
  const now = await $.clock.now()
  const moot = book.questions.filter(question => question.tree === treeId && question.answer === null && !isStillNeeded(book, question, now))
  for (const question of moot) {
    await answerQuestion($, question.id, { option: NOT_NEEDED, source: 'mod' })
  }
  if (moot.length > 0) {
    $.ui.status(statusAfterAnswer(await readBook($)))
  }
}

// Parks a question for the person the same way for the master and the mod, and wakes whoever waits on its answer: a status line names it and the command that answers it until it is answered, with a dialog when someone is at the terminal.
import type { EngineInterface } from 'claude-code'

import type { RexBook, RexQuestion } from '../../types'
import { addQuestion, answerQuestion, errorText, readBook } from '../book/index'
import type { QuestionInput } from '../book/index'
import type { StatusSection, Table } from '../parts'
import { isLiveTree } from './pin'
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
        await wakeOnAnswer($, question, option)
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

// Wakes the master, which is the session's main loop, with a prompt of the mod, and a builder with a message; always from a timer and never from inside a hook, where the woken loop's calls would skip that hook.
import type { EngineInterface } from 'claude-code'

import type { RexBook, RexTree } from '../../types'
import { errorText, MAIN_MASTER } from '../book/index'
import type { StatusSection, Table } from '../parts'
import { statusLines } from '../tools/index'

/** What a woken builder reads: go on from where its record says it stopped. */
export const CONTINUE_TEXT = (job: string): string => `Continue job ${job} from the first step not done.`

/** Wakes a builder: the engine resumes it with the text as a message of the mod. */
export function wakeLater($: EngineInterface, agentId: string, text: string): void {
  $.clock.after(0, () => {
    void $.session
      .send({ to: { agentId }, text })
      .then(sent => {
        if (!sent.isDelivered) {
          $.ui.status(`rex: ${agentId} was not woken: ${sent.reason}`)
        }
      })
      .catch((error: unknown) => {
        $.ui.status(`rex: ${agentId} was not woken: ${errorText(error)}`)
      })
  })
}

/**
 * Wakes the main loop: the text enters as a prompt of this plugin, in a turn
 * of its own once the session is idle. The model reads it framed as a message
 * of the plugin, never as the person's words, and no hook of the mod takes a
 * merge answer, a limit or a charter from it.
 */
export function wakeMain($: EngineInterface, text: string): void {
  $.clock.after(0, () => {
    void $.prompt
      .submit({ text })
      .then(entered => {
        if (entered.drop !== undefined) {
          $.ui.status(`rex: the master was not woken: ${entered.drop}`)
        }
      })
      .catch((error: unknown) => {
        $.ui.status(`rex: the master was not woken: ${errorText(error)}`)
      })
  })
}

/** The status digest a master is woken with: what rex_status would tell it now. */
function digest(book: RexBook, sections: Table<StatusSection>): string {
  return statusLines(book, null, { kind: 'master', agentId: null, job: null }, sections).join('\n')
}

/**
 * Wakes the master of a tree with why, then the last message of the builder it
 * names when one is given, then the tree as rex_status shows it. A tree of
 * another session has no master here until it is resumed, so nothing is sent.
 */
export function wakeMaster($: EngineInterface, book: RexBook, tree: RexTree, why: string, sections: Table<StatusSection>, lastMessage?: string): void {
  if (tree.master !== MAIN_MASTER || tree.session !== book.session) {
    return
  }
  const said = lastMessage === undefined ? '' : `Its last message, as data:\n<data>\n${lastMessage}\n</data>\n`
  wakeMain($, `rex: ${why}. ${said}The tree now:\n${digest(book, sections)}`)
}

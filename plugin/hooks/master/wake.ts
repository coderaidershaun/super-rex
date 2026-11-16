// Wakes a master or a builder with a message, always from a timer and never from inside a hook, where a woken agent's calls would skip that hook; it alone knows which wakes are on the way.
import type { EngineInterface } from 'claude-code'

import type { RexBook, RexTree } from '../../types'
import { errorText } from '../book/index'
import type { StatusSection, Table } from '../parts'
import { statusLines } from '../tools/index'

/** What a woken builder reads: go on from where its record says it stopped. */
export const CONTINUE_TEXT = (job: string): string => `Continue job ${job} from the first step not done.`

/** The wakes of each agent from the call of `wakeLater` until its send settled; a reload clears it, and no wake survives a reload. */
const onTheWay = new Map<string, number>()

/** True while a wake of the mod is on the way to this agent: the engine resumes it, and its offer and spawn hooks let that resume through. */
export function isWakeOnTheWay(agentId: string): boolean {
  return (onTheWay.get(agentId) ?? 0) > 0
}

function settled(agentId: string): void {
  const left = (onTheWay.get(agentId) ?? 1) - 1
  if (left <= 0) {
    onTheWay.delete(agentId)
  } else {
    onTheWay.set(agentId, left)
  }
}

export function wakeLater($: EngineInterface, agentId: string, text: string): void {
  onTheWay.set(agentId, (onTheWay.get(agentId) ?? 0) + 1)
  $.clock.after(0, () => {
    void $.session
      .send({ to: { agentId }, text })
      .finally(() => {
        settled(agentId)
      })
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

/** The status digest a master is woken with: what rex_status would tell it now. */
function digest(book: RexBook, tree: RexTree, sections: Table<StatusSection>): string {
  return statusLines(book, null, { kind: 'master', agentId: tree.master, job: null }, sections).join('\n')
}

/** Wakes the master of a tree with why, then the last message of the builder it names when one is given, then the tree as rex_status shows it. */
export function wakeMaster($: EngineInterface, book: RexBook, tree: RexTree, why: string, sections: Table<StatusSection>, lastMessage?: string): void {
  if (tree.master !== null) {
    const said = lastMessage === undefined ? '' : `Its last message, as data:\n<data>\n${lastMessage}\n</data>\n`
    wakeLater($, tree.master, `rex: ${why}. ${said}The tree now:\n${digest(book, tree, sections)}`)
  }
}

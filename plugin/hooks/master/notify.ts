// The main loop's model does not answer the turns of the mod's own agents: the engine's notice that a master or a tree builder ended a turn reaches the person as one line, and only a closed tree's last notice reaches the model.
import type { PromptSubmitInput } from 'claude-code'

import type { RexBook } from '../../types'
import { isLiveTree } from './pin'

const TASK_ID = /<task-id>([^<]+)<\/task-id>/

/**
 * What the main loop gets for a task notification: dropped with a line for the
 * person while the agent's tree runs, rewritten when the master's tree is
 * closed, and null to let any other notification pass as it came.
 */
export function notificationAnswer(book: RexBook, e: PromptSubmitInput): { readonly drop: string } | { readonly text: string } | null {
  const agentId = TASK_ID.exec(e.text)?.[1]?.trim()
  if (agentId === undefined) {
    return null
  }
  const trees = book.trees.filter(tree => tree.session === book.session)
  const mastered = trees.find(tree => tree.master === agentId)
  if (mastered !== undefined) {
    if (isLiveTree(mastered)) {
      return { drop: `rex: the master of tree ${mastered.id} ended its turn (tree ${mastered.id} runs; /rex watch shows it)` }
    }
    return { text: `rex: master tree ${mastered.id} is closed (${mastered.state}). Its final report is ${mastered.reportFile ?? 'none'}. Read it and tell the person the result in a few lines.` }
  }
  const built = trees.find(tree => isLiveTree(tree) &&tree.builders.some(builder => builder.agentId === agentId))
  const job = built?.builders.find(builder => builder.agentId === agentId)?.job
  return built === undefined || job === undefined ? null : { drop: `rex: the builder of job ${job} ended its turn (tree ${built.id} runs; /rex watch shows it)` }
}

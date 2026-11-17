// The engine's own notice that a tree builder ended a turn does not reach the master: the mod wakes the master itself, after the done check and only when the builder waits on nothing of its own, so the person reads one line and the model one wake.
import type { PromptSubmitInput } from 'claude-code'

import type { RexBook } from '../../types'
import { isLiveTree } from './pin'

const TASK_ID = /<task-id>([^<]+)<\/task-id>/

/**
 * What the main loop gets for a task notification: dropped with a line for the
 * person when a builder of a live tree of this session sent it, and null to
 * let any other notification pass as it came.
 */
export function notificationAnswer(book: RexBook, e: PromptSubmitInput): { readonly drop: string } | null {
  const agentId = TASK_ID.exec(e.text)?.[1]?.trim()
  if (agentId === undefined) {
    return null
  }
  const built = book.trees.find(tree => tree.session === book.session && isLiveTree(tree) && tree.builders.some(builder => builder.agentId === agentId))
  const job = built?.builders.find(builder => builder.agentId === agentId)?.job
  return built === undefined || job === undefined ? null : { drop: `rex: the builder of job ${job} ended its turn (tree ${built.id} runs; /rex watch shows it)` }
}

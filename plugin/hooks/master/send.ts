// Only the mod and the master's own builders may message a master, so no other agent can steer it or pass it an answer.
import type { EngineInterface, SessionSendInput, SessionSendResult } from 'claude-code'

import { fail, readBook } from '../book/index'

export const MASTER_ONLY = "rex: only this master's builders and the mod may message the master"

export async function sendRefusal($: EngineInterface, event: SessionSendInput): Promise<SessionSendResult | null> {
  try {
    const book = await readBook($)
    const tree = book.trees.find(each => each.master === event.to && (each.state === 'running' || each.state === 'paused'))
    if (tree === undefined) {
      return null
    }
    const isMod = event.origin.kind === 'plugin' && event.origin.name === 'super-rex'
    const isBuilder = event.agentId !== undefined && tree.builders.some(builder => builder.agentId === event.agentId)
    return isMod || isBuilder ? null : { isDelivered: false, reason: MASTER_ONLY }
  } catch (error) {
    const name = error instanceof Error ? error.name : 'Error'
    await fail($, { kind: 'authority-check-failed', check: 'master-messages', error: error instanceof Error ? `${name}: ${error.message}` : String(error) })
    return { isDelivered: false, reason: `rex: the check of messages to a master failed (${name}); refused` }
  }
}

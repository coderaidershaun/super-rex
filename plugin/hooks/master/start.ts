// Starts the master of a tree the book already holds, new or resumed: its first prompt is the charter, a marked reading of it, and the digest of a restart.
import type { EngineInterface } from 'claude-code'

import { addPendingSpawn, closeTree } from '../book/index'
import { restate } from '../prompts/index'
import { MASTER_TYPE } from './agent-type'
import { spawnOwn } from './spawn'

/** The master's first prompt: the charter as the person wrote it, then a reading of it that another model made, marked as such. */
export function masterPrompt(tree: string, charter: string, restatement: string | null, digest: string | null): string {
  const parts = [`Rex-Tree: ${tree}`, `The charter of this tree, as the person wrote it:\n<charter>\n${charter}\n</charter>`]
  if (restatement !== null) {
    parts.push(`A reading of the charter by another model (the charter above stays the goal):\n<reading>\n${restatement}\n</reading>`)
  }
  if (digest !== null) {
    parts.push(`This tree started again after a restart. What the store held:\n<digest>\n${digest}\n</digest>`)
  }
  parts.push('Call rex_status first, then follow your procedure.')
  return parts.join('\n\n')
}

export type StartedTree = { readonly tree: string; readonly master: string }

/** Spawns the master of a tree in the book; a refused spawn closes the tree as lost and its text is the answer. */
export async function spawnMaster($: EngineInterface, input: { readonly tree: string; readonly charter: string; readonly digest: string | null }): Promise<StartedTree | { readonly refused: string }> {
  const read = await restate($, input.charter)
  const description = `rex master ${input.tree}`
  await addPendingSpawn($, { description, kind: 'master', tree: input.tree, job: null })
  const prompt = masterPrompt(input.tree, input.charter, read.isRestated ? read.text : null, input.digest)
  const spawned = await spawnOwn($, { subagentType: MASTER_TYPE, description, prompt })
  if (spawned.deny !== undefined || spawned.agentId === undefined) {
    await closeTree($, input.tree, 'lost', null)
    return { refused: spawned.deny ?? `rex master: the master of tree ${input.tree} did not start; run /rex doctor` }
  }
  return { tree: input.tree, master: spawned.agentId }
}

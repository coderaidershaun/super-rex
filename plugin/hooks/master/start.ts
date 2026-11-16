// Starts the master of a tree the book already holds, new or resumed: its first prompt is the charter, a marked reading of it, the policy, the repo's places, the code map, and the digest of a restart.
import type { EngineInterface } from 'claude-code'

import { addPendingSpawn, closeTree } from '../book/index'
import { READ_CHARTER, restate } from '../prompts/index'
import { repoPlaces } from '../tools/index'
import type { RepoPlaces } from '../tools/index'
import { MASTER_TYPE } from './agent-type'
import { masterPolicyLines } from './policy'
import type { Policy } from './policy'
import { spawnOwn } from './spawn'
import { realRoot } from './store'

/** The map of the code is cut here, so a big repo does not fill the master's context. */
export const CODEBASE_MAX_LINES = 150

/** What the mod knows besides the charter, each block in the order the prompt gives it. */
export type MasterData = { readonly restatement: string | null; readonly policy: Policy; readonly repo: RepoPlaces; readonly codebase: string | null; readonly digest: string | null }

/** The master's first prompt: the charter as the person wrote it, then a reading of it that another model made, marked as such, then the mod's data. */
export function masterPrompt(tree: string, charter: string, data: MasterData): string {
  const parts = [`Rex-Tree: ${tree}`, `The charter of this tree, as the person wrote it:\n<charter>\n${charter}\n</charter>`]
  if (data.restatement !== null) {
    parts.push(`A reading of the charter by another model (the charter above stays the goal):\n<reading>\n${data.restatement}\n</reading>`)
  }
  parts.push(`The policy the person confirmed, which the mod enforces on each of your calls:\n<policy>\n${masterPolicyLines(data.policy).join('\n')}\n</policy>`)
  parts.push(`The repo each job builds in by default, as git holds it now:\n<repo>\nrepo=${data.repo.repo}\nbase=${data.repo.base}\nremote=${data.repo.remote}\n</repo>`)
  if (data.codebase !== null) {
    parts.push(`The source files of the repo and what each one is for, from rex codebase:\n<codebase>\n${data.codebase}\n</codebase>`)
  }
  if (data.digest !== null) {
    parts.push(`This tree started again after a restart. What the store held:\n<digest>\n${data.digest}\n</digest>`)
  }
  parts.push('Call rex_status first, then follow your procedure.')
  return parts.join('\n\n')
}

/** The code map of the repo, cut to its first lines; null when `rex codebase` fails, as the master can still read the files. */
export async function codebaseMap($: EngineInterface): Promise<string | null> {
  let ran
  try {
    ran = await $.process.run(['rex', 'codebase', '--only', 'code', '--with-context'], { cwd: await $.session.root() })
  } catch (error) {
    if (error instanceof Error) {
      return null
    }
    throw error
  }
  if (ran.exitCode !== 0) {
    return null
  }
  const rows = ran.stdout.replace(/\n$/, '').split('\n')
  if (rows.length <= CODEBASE_MAX_LINES) {
    return rows.join('\n')
  }
  return [...rows.slice(0, CODEBASE_MAX_LINES), `(${String(rows.length - CODEBASE_MAX_LINES)} more lines cut)`].join('\n')
}

export type StartedTree = { readonly tree: string; readonly master: string }

export type MasterStart = { readonly tree: string; readonly charter: string; readonly digest: string | null; readonly policy: Policy }

/** Spawns the master of a tree in the book; a refused spawn closes the tree as lost and its text is the answer. */
export async function spawnMaster($: EngineInterface, input: MasterStart): Promise<StartedTree | { readonly refused: string }> {
  const read = await restate($, input.charter, READ_CHARTER)
  const repo = await repoPlaces($, await realRoot($))
  const codebase = await codebaseMap($)
  const description = `rex master ${input.tree}`
  await addPendingSpawn($, { description, kind: 'master', tree: input.tree, job: null })
  const prompt = masterPrompt(input.tree, input.charter, { restatement: read.isRestated ? read.text : null, policy: input.policy, repo, codebase, digest: input.digest })
  const spawned = await spawnOwn($, { subagentType: MASTER_TYPE, description, prompt })
  if (spawned.deny !== undefined || spawned.agentId === undefined) {
    await closeTree($, input.tree, 'lost', null)
    return { refused: spawned.deny ?? `rex master: the master of tree ${input.tree} did not start; run /rex doctor` }
  }
  return { tree: input.tree, master: spawned.agentId }
}

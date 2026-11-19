// What the master reads when its tree opens, new or resumed: the charter, a marked reading of it, its budget and time, the policy, the repo's places, the code map, and the digest of a restart.
import type { EngineInterface } from 'claude-code'

import { READ_CHARTER, restate } from '../prompts/index'
import { repoPlaces, usd } from '../tools/index'
import type { RepoPlaces } from '../tools/index'
import type { RexTree } from '../../types'
import { masterPolicyLines } from './policy'
import type { Policy } from './policy'
import { realRoot } from './store'
import { spanText, whenText } from './tree-limits'

/** The map of the code is cut here, so a big repo does not fill the master's context. */
export const CODEBASE_MAX_LINES = 150

/** What the mod knows besides the charter, each block in the order the brief gives it. */
export type TreeData = { readonly restatement: string | null; readonly limits: readonly string[]; readonly policy: Policy; readonly repo: RepoPlaces; readonly codebase: string | null; readonly digest: string | null }

/** The blocks of a tree's brief: the charter as the person wrote it, then a reading of it that another model made, marked as such, then the mod's data. */
export function treeBrief(charter: string, data: TreeData): string {
  const parts = [`The charter of this tree, as the person wrote it:\n<charter>\n${charter}\n</charter>`]
  if (data.restatement !== null) {
    parts.push(`A reading of the charter by another model (the charter above stays the goal):\n<reading>\n${data.restatement}\n</reading>`)
  }
  parts.push(`The budget and the time the person gave this tree. Plan the work to fit both. When the tree reaches one of them, the mod starts nothing new and asks the person to raise it or stop the tree:\n<limits>\n${data.limits.join('\n')}\n</limits>`)
  parts.push(`The policy the person confirmed, which the mod enforces on each of your calls:\n<policy>\n${masterPolicyLines(data.policy).join('\n')}\n</policy>`)
  parts.push(`The repo each job builds in by default, as git holds it now:\n<repo>\nrepo=${data.repo.repo}\nbase=${data.repo.base}\nremote=${data.repo.remote}\n</repo>`)
  if (data.codebase !== null) {
    parts.push(`The source files of the repo and what each one is for, from rex codebase:\n<codebase>\n${data.codebase}\n</codebase>`)
  }
  if (data.digest !== null) {
    parts.push(`This tree started again after a restart. What the store held:\n<digest>\n${data.digest}\n</digest>`)
  }
  return `${parts.join('\n\n')}\n`
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

export type TreeStart = { readonly tree: RexTree; readonly digest: string | null; readonly policy: Policy }

/** The brief of a tree the book already holds: the reading of the charter, the repo and the code map are made here. */
export async function briefOf($: EngineInterface, input: TreeStart): Promise<string> {
  const { tree } = input
  const read = await restate($, tree.charter, READ_CHARTER)
  const repo = await repoPlaces($, await realRoot($))
  const codebase = await codebaseMap($)
  const limits = [`budget_usd=${usd(tree.capNano)}`, `time=${spanText(tree.timeMs)}`, `ends_at=${tree.deadlineAt === null ? 'none' : whenText(tree.deadlineAt)}`]
  return treeBrief(tree.charter, { restatement: read.isRestated ? read.text : null, limits, policy: input.policy, repo, codebase, digest: input.digest })
}

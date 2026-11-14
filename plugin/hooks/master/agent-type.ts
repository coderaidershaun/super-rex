// The master's agent type: the procedure file as its prompt, a fixed tool list with no Agent tool, and the policy's model; no model is ever offered it.
import type { EngineInterface } from 'claude-code'

import { errorText } from '../book/index'
import { DEFAULT_POLICY } from './policy'
import { readPolicy } from './store'

export const MASTER_TYPE = 'super-rex:master'
export const PROCEDURE_FILE = 'skills/orchestration-builder/master/procedure.md'
export const BUILDER_BRIEF_FILE = 'skills/orchestration-builder/master/builder.md'

/** Read reaches the builders' output files; ToolSearch loads the mod tools that wait behind it. */
export const MASTER_TOOLS: readonly string[] = [
  'Read',
  'ToolSearch',
  'mcp__super-rex__rex_build',
  'mcp__super-rex__rex_status',
  'mcp__super-rex__rex_continue',
  'mcp__super-rex__rex_note',
  'mcp__super-rex__rex_ask_owner',
  'mcp__super-rex__rex_skill',
]

export function readPluginFile($: EngineInterface, file: string): Promise<string> {
  return $.fs.read(`${$.plugin.root}/${file}`)
}

/** Whether this load of the mod registered the master type; `/rex master` refuses with the reason when it did not. */
export type MasterTypeState = { readonly kind: 'registered' } | { readonly kind: 'unregistered'; readonly reason: string }

export async function registerMasterType($: EngineInterface): Promise<MasterTypeState> {
  try {
    const procedure = await readPluginFile($, PROCEDURE_FILE)
    const stored = await readPolicy($)
    const model = (stored?.policy ?? DEFAULT_POLICY).master_model
    await $.agent.register({
      name: 'master',
      description: 'SuperRex master: runs a tree of jobs from the charter of the person. Only /rex master starts it.',
      prompt: procedure,
      tools: MASTER_TOOLS,
      model,
      effort: 'high',
    })
    return { kind: 'registered' }
  } catch (error) {
    return { kind: 'unregistered', reason: errorText(error) }
  }
}

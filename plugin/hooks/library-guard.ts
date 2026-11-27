// Refuses the raw tools of the plugin's skill library server to every agent of an open job, so that agents of a job reach the library only through the mod's tools, which record and gate each use.
// A person or another MCP client outside a job still calls the raw tools, because the library must work as an MCP server by itself.
import type { EngineInterface, On } from 'claude-code'

import { errorText, isOpen, readBook } from './book/index'
import { jobOfLoop } from './gate/index'
import type { GateCheck, Parts } from './parts'
import { failureName } from './tools/index'

/** The raw tools of the library server, as an agent sees and calls them. */
export const LIBRARY_TOOLS = ['mcp__plugin_super-rex_library__head', 'mcp__plugin_super-rex_library__search', 'mcp__plugin_super-rex_library__get'] as const

export const LIBRARY_GUARD_CHECKS: readonly GateCheck[] = LIBRARY_TOOLS.map(tool => ({
  name: `library-guard-${tool.slice(tool.lastIndexOf('__') + 2)}`,
  site: { kind: 'tool', tool },
  failure: 'closed',
}))

const INSTRUCTION = [
  'Use the tools of the mod, so that the mod records each use of the library.',
  'To read a skill file, call rex_skill with the name of the skill.',
  'Add the name of the file when it is not SKILL.md.',
  'The tool rex_skill_search will replace the raw search tool.',
  'It does not exist yet, so use the skills that your brief names.',
].join(' ')

/** The refusal that an agent of a job reads in place of the raw tool's answer. */
export function libraryRefusal(): string {
  return `rex library guard: an agent of a job must not call the raw tools of the skill library. ${INSTRUCTION}`
}

/** The refusal when the guard cannot tell whether the caller is an agent of an open job: it fails closed. */
function failedRefusal(tool: string, reason: string): string {
  return `rex library guard: the check of ${tool} failed (${reason}), so the call is refused. An agent of a job must not call the raw tools of the skill library. ${INSTRUCTION}`
}

/** True when the loop that makes the call is the builder or a sub-agent of a job that is still open. An inert mod refuses nothing. */
async function isJobAgent($: EngineInterface, agentId: string | undefined): Promise<boolean> {
  const book = await readBook($)
  if (book.inert !== null) {
    return false
  }
  const job = jobOfLoop(book, agentId)
  return job !== undefined && isOpen(job)
}

const RAW_TOOL = new RegExp(`^(?:${LIBRARY_TOOLS.join('|')})$`)

export function register(on: On, parts: Parts): void {
  for (const check of LIBRARY_GUARD_CHECKS) {
    parts.gateChecks.add(check)
  }

  // Only `tool.call`: the mod's own `$.mcp.call` of the library server is the `mcp.call` event, so this guard never sees it.
  on('tool.call', { tool: RAW_TOOL }, async ($, e, next) => {
    let isRefused: boolean
    try {
      isRefused = await isJobAgent($, e.agentId)
    } catch (error) {
      return { deny: failedRefusal(e.tool, errorText(error)) }
    }
    return isRefused ? { deny: libraryRefusal() } : next(e)
  }).catch((_$, e, next) => (next.called ? next(e) : { deny: failedRefusal(e.tool, failureName(next.error)) }))
}

// Applies the assignment table to a plan's tasks and roles, and asks the decision agent only what needs judgement.
import type { EngineInterface, On, ToolCallResult } from 'claude-code'

import { fail } from '../book/index'
import type { Parts } from '../parts'
import { deny, toolFailed } from '../tools/index'
import { ASSIGN_TOOL, serveAssign, TOOL } from './serve'
import { DECISIONS_SECTION } from './status'

/**
 * A throw inside the hook refuses here: an answer of `deny` from the
 * registration's catch does not stop the call, so only a timeout reaches it.
 */
async function serveOrRefuse($: EngineInterface, event: { readonly agentId?: string }): Promise<ToolCallResult> {
  try {
    return await serveAssign($, event)
  } catch (error) {
    if (!(error instanceof Error)) {
      throw error
    }
    await fail($, { kind: 'authority-check-failed', check: TOOL, error: error.message })
    return deny(`rex: ${TOOL} failed (${error.name}); refused`)
  }
}

export function register(on: On, parts: Parts): void {
  parts.tools.add(ASSIGN_TOOL)
  parts.statusSections.add(DECISIONS_SECTION)
  on('tool.call', { tool: 'mcp__super-rex__rex_assign' }, ($, e) => serveOrRefuse($, e)).catch(($, _e, next) => toolFailed($, TOOL, next.error))
}

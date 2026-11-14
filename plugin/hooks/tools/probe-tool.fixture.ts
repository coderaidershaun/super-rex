// A tool that only a test adds, as another module adds one: a data entry in the table and its own matched hook, which serves the main loop and steps aside for any other caller.
import type { On, ToolCallResult } from 'claude-code'

import type { Parts, ToolEntry } from '../parts'

export const PROBE_TOOL: ToolEntry = { name: 'rex_probe', description: 'Answers probe=yes to a builder.', inputSchema: { type: 'object' }, isDeferred: true, callers: ['builder'] }

function probeAnswer(event: { readonly agentId?: string }): ToolCallResult | null {
  return event.agentId === undefined ? { result: 'probe=yes\n' } : null
}

export function registerProbeTool(on: On, parts: Parts, prefix: string): void {
  parts.tools.add(PROBE_TOOL)
  on('tool.call', { tool: new RegExp(`^${prefix}${PROBE_TOOL.name}$`) }, (_$, e, next) => probeAnswer(e) ?? next(e))
}

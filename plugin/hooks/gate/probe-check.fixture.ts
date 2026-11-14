// A gate check that only a test adds, as another module adds one: a data entry in the table and its own closed hook on one tool, whose catch refuses when it throws.
import type { On } from 'claude-code'

import type { GateCheck, Parts } from '../parts'

export const PROBE_CHECK: GateCheck = { name: 'probe', site: { kind: 'tool', tool: 'mcp__probe__check' }, failure: 'closed' }

export class ProbeBrokeError extends Error {
  constructor() {
    super('the probe check broke')
    this.name = 'ProbeBrokeError'
  }
}

export function registerProbeCheck(on: On, parts: Parts): void {
  parts.gateChecks.add(PROBE_CHECK)
  on('tool.call', { tool: 'mcp__probe__check' }, () => {
    throw new ProbeBrokeError()
  }).catch((_$, e, next) => (next.called ? next(e) : { deny: `rex gate: ${PROBE_CHECK.name} failed (${next.error.message ?? next.error.kind}); refused` }))
}

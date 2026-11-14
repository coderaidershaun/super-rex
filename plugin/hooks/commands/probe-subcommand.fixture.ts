// A sub-command that only a test adds, as another module would add one: a data entry in the table and its own matched hook.
import type { On } from 'claude-code'

import type { Parts, Subcommand } from '../parts'
import { originRefusal, rexLine } from './index'

export const PROBE_SUBCOMMAND: Subcommand = { name: 'probe', summary: 'Answers what it was given', grantsAuthority: true }

export function registerProbe(on: On, parts: Parts, command: string): void {
  parts.subcommands.add(PROBE_SUBCOMMAND)
  on('command.run', { command, args: /^probe(\s|$)/ }, (_$, e) => {
    const refusal = originRefusal(e, PROBE_SUBCOMMAND, false)
    return { text: refusal ?? `probe says ${rexLine(e).args}` }
  })
}

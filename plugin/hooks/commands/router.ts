// Reads a `/rex` line, checks that only the person's own typing grants authority, and runs this module's sub-commands; other modules answer their own names first.
import type { CommandRunInput, CommandRunResult, EngineInterface } from 'claude-code'

import { fail, readBook } from '../book/index'
import type { Subcommand, Table } from '../parts'
import { doctor, jobs, prompts, status, watch } from './subcommands'
import type { Request } from './subcommands'

/** The order the unknown sub-command line lists names in; a name another module adds follows them. */
const KNOWN_ORDER = ['watch', 'status', 'jobs', 'doctor', 'prompts', 'limit', 'master', 'policy', 'answer']

export const OWN_SUBCOMMANDS: readonly Subcommand[] = [
  { name: 'watch', summary: 'Opens the SuperRex pane', grantsAuthority: false },
  { name: 'status', summary: 'Shows where a job stands', grantsAuthority: false },
  { name: 'jobs', summary: 'Lists the jobs', grantsAuthority: false },
  { name: 'doctor', summary: "Runs the binary's check and names the mod's version", grantsAuthority: false },
  { name: 'prompts', summary: 'Sets the prompt mode: off, observe or improve', grantsAuthority: true },
]

export type RexLine = { readonly name: string; readonly args: string }

/** The sub-command's name is the text before the first space; the rest is its arguments. */
export function rexLine(event: CommandRunInput): RexLine {
  const line = event.args.trim()
  const space = line.indexOf(' ')
  return space === -1 ? { name: line, args: '' } : { name: line.slice(0, space), args: line.slice(space + 1).trim() }
}

export function unknownLine(name: string, table: Table<Subcommand>): string {
  const names = table.entries().map(entry => entry.name)
  const known = KNOWN_ORDER.filter(each => names.includes(each))
  const listed = [...known, ...names.filter(each => !KNOWN_ORDER.includes(each))].join(', ')
  return name === '' ? `rex: name a sub-command; one of ${listed}` : `rex: no sub-command ${name}; one of ${listed}`
}

/**
 * The origin rule of every sub-command: one that grants authority runs on the
 * person's own typing, or on another origin its owner allows; null when it may run.
 */
export function originRefusal(event: CommandRunInput, entry: Subcommand, isOtherOriginAllowed: boolean): string | null {
  const kind = event.origin.kind
  if (!entry.grantsAuthority || kind === 'composer' || kind === 'bridge' || isOtherOriginAllowed) {
    return null
  }
  return `rex ${entry.name}: only your own typing may run this, got origin ${kind}`
}

async function runOwn($: EngineInterface, name: string, request: Request): Promise<CommandRunResult> {
  switch (name) {
    case 'watch':
      return watch($)
    case 'status':
      return status(request)
    case 'jobs':
      return jobs($, request)
    case 'doctor':
      return doctor($)
    case 'prompts':
      return prompts($, request)
    default:
      return { text: `rex ${name}: no hook of the module that added it answered; run /rex doctor` }
  }
}

/** The router stands after every other module's `/rex` hook, so a line that reaches it is this module's, or unknown. */
export async function route($: EngineInterface, event: CommandRunInput, table: Table<Subcommand>): Promise<CommandRunResult> {
  const line = rexLine(event)
  const entry = table.get(line.name)
  if (entry === undefined) {
    return { text: unknownLine(line.name, table) }
  }
  const refusal = originRefusal(event, entry, false)
  if (refusal !== null) {
    await fail($, { kind: 'origin-not-person', action: `rex ${entry.name}`, origin: event.origin.kind })
    return { text: refusal }
  }
  return runOwn($, entry.name, { event, args: line.args, book: await readBook($) })
}

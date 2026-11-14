// The six tables through which one module adds sub-commands, tools, gate checks, pane views, status lines and brief parts to the work of another module, so neither edits the other's files.
// No entry holds or takes `$`: the engine refuses `$` inside an object. A sub-command, a tool and a gate check act through their owner's own hook; the other three compute from data.
import type { AgentSpawnInput, Elements } from 'claude-code'

import type { RexBook, RexJob, RexRun, RexSkillSource } from '../types'

export type CallerKind = 'main' | 'builder' | 'master' | 'agent'

/** Who called a tool, read from the engine's `agentId` and the book, never from an argument. */
export type Caller = {
  readonly kind: CallerKind
  readonly agentId: string | null
  readonly job: RexJob | null
}

/**
 * One `/rex <name>` sub-command, as data. Its owner serves it with its own hook
 * `on('command.run', { command: 'rex', args: /^<name>(\s|$)/ }, hook)` and first asks the origin check that `commands` exports.
 */
export type Subcommand = {
  readonly name: string
  readonly summary: string
  readonly grantsAuthority: boolean
}

/**
 * One mod tool, as data, listed as `mcp__super-rex__<name>`. The `tools` module registers it with the engine;
 * its owner serves it with its own hook `on('tool.call', { tool: 'mcp__super-rex__<name>' }, hook)`.
 */
export type ToolEntry = {
  readonly name: string
  readonly description: string
  readonly inputSchema: { readonly [key: string]: unknown }
  readonly isDeferred: boolean
  readonly callers: readonly CallerKind[]
}

/** Where a gate check's own hook runs: on new agents (the Agent call, the spawn, or both), or on calls of one tool. */
export type GateSite =
  | { readonly kind: 'agent'; readonly at: 'tool.call' | 'agent.spawn' | 'both' }
  | { readonly kind: 'tool'; readonly tool: string }

/** One gate check, as data. Its owner's hook at `site` refuses; one with `failure: 'closed'` refuses also when it fails. */
export type GateCheck = {
  readonly name: string
  readonly site: GateSite
  readonly failure: 'open' | 'closed'
}

/** The elements a pane view draws with, as `$.ui.resolve(e)` gave them to the pane. */
export type PaneElements = Pick<Elements['terminal'], 'Box' | 'Text' | 'Button'>

/** What a pane view may make happen; the pane builds each closure, so the view never holds `$`. */
export type PaneActions = {
  readonly answer: (question: string, option: string) => void
}

export type PaneViewContext = {
  readonly elements: PaneElements
  readonly book: RexBook
  readonly columns: number
  readonly actions: PaneActions
}

export type PaneView = {
  readonly name: string
  readonly title: string
  readonly order: number
  readonly draw: (ctx: PaneViewContext) => ReturnType<PaneElements['Box']>
}

export type StatusContext = {
  readonly book: RexBook
  readonly job: RexJob | null
  readonly caller: Caller
}

export type StatusSection = {
  readonly name: string
  readonly order: number
  readonly lines: (ctx: StatusContext) => readonly string[]
}

/** A skill file the pipeline read for this brief before it asked the parts. */
export type BriefFile = {
  readonly skill: string
  readonly file: string
  readonly source: RexSkillSource
  readonly hash: string
  readonly text: string
}

/** `spawn` is absent when `rex_skill` with no name hands an agent its missing assembly. */
export type BriefContext = {
  readonly book: RexBook
  readonly job: RexJob
  readonly run: RexRun | null
  readonly spawn: AgentSpawnInput | null
  readonly role: string | null
  readonly task: string | null
  readonly files: readonly BriefFile[]
}

/** One part of an assembled brief; the author's own text stands at order 30. */
export type BriefPart = {
  readonly name: string
  readonly order: number
  readonly text: (ctx: BriefContext) => string | null
}

export class DuplicateEntryError extends Error {
  readonly table: string
  readonly entry: string

  constructor(table: string, entry: string) {
    super(`table ${table} already holds an entry named ${entry}`)
    this.name = 'DuplicateEntryError'
    this.table = table
    this.entry = entry
  }
}

export class Table<T extends { readonly name: string }> {
  readonly label: string
  readonly #entries = new Map<string, T>()

  constructor(label: string) {
    this.label = label
  }

  add(entry: T): void {
    if (this.#entries.has(entry.name)) {
      throw new DuplicateEntryError(this.label, entry.name)
    }
    this.#entries.set(entry.name, entry)
  }

  get(name: string): T | undefined {
    return this.#entries.get(name)
  }

  entries(): readonly T[] {
    return [...this.#entries.values()]
  }
}

export type Parts = {
  readonly subcommands: Table<Subcommand>
  readonly tools: Table<ToolEntry>
  readonly gateChecks: Table<GateCheck>
  readonly paneViews: Table<PaneView>
  readonly statusSections: Table<StatusSection>
  readonly briefParts: Table<BriefPart>
}

export function createParts(): Parts {
  return {
    subcommands: new Table<Subcommand>('subcommands'),
    tools: new Table<ToolEntry>('tools'),
    gateChecks: new Table<GateCheck>('gateChecks'),
    paneViews: new Table<PaneView>('paneViews'),
    statusSections: new Table<StatusSection>('statusSections'),
    briefParts: new Table<BriefPart>('briefParts'),
  }
}

// Pure changes of agent runs: where a run sits in the tree, which job and role it belongs to, what it spent, and how it ended.
import type { TurnUsage } from 'claude-code'

import type { RexBook, RexJob, RexLintAttempt, RexRun, RexTokens } from '../../types'
import { findJob, isOpen, NO_TOKENS, queued, replaceJob, tokensData, tokensPlus } from './events'
import { priceNano } from './prices'

export type StartRunInput = {
  readonly agentId: string
  readonly toolUseId: string | null
  /** The engine's `parentAgentId`; null when the main loop spawned it. */
  readonly parentAgentId: string | null
  readonly agentType: string
  readonly description: string
  /** True when this plugin's own `$.agent.spawn` raised the spawn. */
  readonly isOwnSpawn: boolean
  readonly planned?: string | null
  readonly task?: string | null
  readonly briefHash?: string | null
  readonly assembled?: RexRun['assembled']
  readonly lint?: RexRun['lint']
  readonly lintFindings?: readonly string[]
  readonly skillsAttached?: readonly string[]
}

const PLUGIN_PREFIX = 'super-rex:'

function bareType(agentType: string): string {
  return agentType.startsWith(PLUGIN_PREFIX) ? agentType.slice(PLUGIN_PREFIX.length) : agentType
}

export function findRun(book: RexBook, agentId: string): RexRun | undefined {
  return book.runs.find(run => run.agentId === agentId)
}

function replaceRun(book: RexBook, agentId: string, change: (run: RexRun) => RexRun): RexBook {
  return { ...book, runs: book.runs.map(run => (run.agentId === agentId ? change(run) : run)) }
}

function mainJob(book: RexBook): RexJob | undefined {
  return book.jobs.find(job => isOpen(job) && job.owner.session === book.session && job.owner.builder === 'main')
}

type Place = { readonly parent: string; readonly parentSource: RexRun['parentSource']; readonly depth: number; readonly job: string | null; readonly tree: string | null }

function placeOf(book: RexBook, input: StartRunInput): { readonly place: Place; readonly book: RexBook } {
  const pending = input.isOwnSpawn && input.parentAgentId === null ? book.pendingSpawns.find(spawn => spawn.description === input.description) : undefined
  if (pending !== undefined) {
    const rest = { ...book, pendingSpawns: book.pendingSpawns.filter(spawn => spawn !== pending) }
    const tree = rest.trees.find(each => each.id === pending.tree)
    if (pending.kind === 'master') {
      const trees = rest.trees.map(each => (each.id === pending.tree ? { ...each, master: input.agentId } : each))
      return { place: { parent: 'main', parentSource: 'mod', depth: 1, job: null, tree: pending.tree }, book: { ...rest, trees } }
    }
    return { place: { parent: tree?.master ?? 'main', parentSource: 'mod', depth: 1, job: pending.job, tree: null }, book: rest }
  }
  const parentRun = input.parentAgentId === null ? undefined : findRun(book, input.parentAgentId)
  const ownJob = book.jobs.find(job => isOpen(job) && job.owner.builder === input.agentId)
  const inherited = input.parentAgentId === null ? (mainJob(book)?.id ?? null) : (parentRun?.job ?? null)
  return {
    place: {
      parent: input.parentAgentId ?? 'main',
      parentSource: 'engine',
      depth: parentRun === undefined ? 1 : parentRun.depth + 1,
      job: ownJob?.id ?? inherited,
      tree: null,
    },
    book,
  }
}

/**
 * The plan role whose grid agent the run is. When several roles share that
 * grid agent, the parent's own role wins, since a lead's workers do its tasks;
 * else the first in plan order.
 */
function roleOf(job: RexJob | undefined, agentType: string, parentRole: string | null): string | null {
  const bare = bareType(agentType)
  const matching = job?.plan?.roles.filter(role => role.gridAgent === bare) ?? []
  const parents = matching.find(role => role.role === parentRole)
  return (parents ?? matching[0])?.role ?? null
}

export function startRunIn(book: RexBook, input: StartRunInput, at: number): RexBook {
  const existing = findRun(book, input.agentId)
  if (existing !== undefined) {
    return replaceRun(book, input.agentId, run => ({ ...run, ...assemblyOf(input, run), state: 'open', endedAt: null, endReason: null }))
  }
  const placed = placeOf(book, input)
  const { place } = placed
  const job = place.job === null ? undefined : findJob(placed.book, place.job)
  const parentRole = input.parentAgentId === null ? null : (findRun(placed.book, input.parentAgentId)?.role ?? null)
  const role = roleOf(job, input.agentType, parentRole)
  const plannedRole = job?.plan?.roles.find(each => each.role === role)
  const fresh: RexRun = {
    agentId: input.agentId,
    toolUseId: input.toolUseId,
    job: place.job,
    tree: place.tree,
    parent: place.parent,
    parentSource: place.parentSource,
    depth: place.depth,
    role,
    task: input.task ?? null,
    agentType: input.agentType,
    planned: input.planned ?? plannedRole?.gridAgent ?? null,
    ran: null,
    marks: [],
    state: 'open',
    endReason: null,
    startedAt: at,
    endedAt: null,
    tokens: NO_TOKENS,
    spentNano: 0,
    briefHash: null,
    assembled: 'no',
    lint: null,
    lintFindings: [],
    skillsAttached: [],
    skillsOpened: [],
    answerChars: null,
    outputFile: null,
  }
  const run: RexRun = { ...fresh, ...assemblyOf(input, fresh) }
  return queued({ ...placed.book, runs: [...placed.book.runs, run] }, at, 'run.start', run.agentId, run.job, {
    agent_id: run.agentId,
    parent: run.parent,
    parent_source: run.parentSource,
    depth: run.depth,
    agent_type: run.agentType,
    planned: run.planned,
    role: run.role,
    task: run.task,
    tool_use_id: run.toolUseId,
    brief_hash: run.briefHash,
    assembled: run.assembled,
    lint: run.lint,
    skills_attached: run.skillsAttached,
    tree: run.tree,
  })
}

/** What the spawn pipeline knows about a run's brief; a run started again keeps its first type. */
function assemblyOf(input: StartRunInput, run: RexRun): Partial<RexRun> {
  return {
    planned: input.planned ?? run.planned,
    task: input.task ?? run.task,
    briefHash: input.briefHash ?? run.briefHash,
    assembled: input.assembled ?? run.assembled,
    lint: input.lint ?? run.lint,
    lintFindings: input.lintFindings ?? run.lintFindings,
    skillsAttached: input.skillsAttached ?? run.skillsAttached,
  }
}

export type StepInput = {
  readonly model: string
  readonly effort: string | null
  readonly usage: TurnUsage | null
}

function tokensOf(usage: TurnUsage | null): RexTokens {
  if (usage === null) {
    return NO_TOKENS
  }
  return { input: usage.input_tokens, output: usage.output_tokens, cacheRead: usage.cache_read_input_tokens, cacheCreation: usage.cache_creation_input_tokens }
}

const TIER_FAMILY: { readonly [tier: string]: string } = { low: 'haiku', medium: 'sonnet', high: 'opus', xhigh: 'fable' }
const GRID_AGENT = /^model-(low|medium|high|xhigh)-effort-(low|medium|high|xhigh|max)$/

/** True when a step ran on another model family or effort than the run's planned grid agent gives. */
function ranOther(planned: string | null, step: StepInput): boolean {
  const grid = planned === null ? null : GRID_AGENT.exec(planned)
  if (grid === null) {
    return false
  }
  const family = TIER_FAMILY[grid[1] ?? ''] ?? ''
  return !step.model.includes(family) || step.effort !== grid[2]
}

export function stepRunIn(book: RexBook, agentId: string, step: StepInput, at: number): RexBook {
  const run = findRun(book, agentId)
  if (run === undefined) {
    return book
  }
  const tokens = tokensOf(step.usage)
  const spent = step.usage === null ? 0 : priceNano(step.usage)
  const planned = run.planned ?? (GRID_AGENT.test(bareType(run.agentType)) ? bareType(run.agentType) : null)
  const mark = `ran-other:${step.model}/${step.effort ?? 'none'}`
  const isNewMark = ranOther(planned, step) && !run.marks.some(old => old.startsWith('ran-other:'))
  let changed = replaceRun(book, agentId, old => ({
    ...old,
    ran: { model: step.model, effort: step.effort },
    tokens: tokensPlus(old.tokens, tokens),
    spentNano: old.spentNano + spent,
    marks: isNewMark ? [...old.marks, mark] : old.marks,
  }))
  if (run.tree !== null) {
    changed = { ...changed, trees: changed.trees.map(tree => (tree.id === run.tree ? { ...tree, ledgerNano: tree.ledgerNano + spent } : tree)) }
  } else if (run.job !== null) {
    changed = replaceJob(changed, run.job, job => ({ ...job, tokens: tokensPlus(job.tokens, tokens), spentNano: job.spentNano + spent }))
  }
  return isNewMark ? queued(changed, at, 'run.mark', agentId, run.job, { agent_id: agentId, mark }) : changed
}

export type EndInput = {
  readonly reason: NonNullable<RexRun['endReason']>
  readonly answer: string
}

export function endRunIn(book: RexBook, agentId: string, end: EndInput, at: number): RexBook {
  const run = findRun(book, agentId)
  if (run === undefined) {
    return book
  }
  const ended = replaceRun(book, agentId, old => ({ ...old, state: 'done', endReason: end.reason, endedAt: at, answerChars: end.answer.length }))
  const data = {
    agent_id: agentId,
    reason: end.reason,
    tokens: tokensData(run.tokens),
    spent_nano: run.spentNano,
    final_message: end.answer,
  }
  return queued(ended, at, 'run.end', agentId, run.job, run.outputFile === null ? data : { ...data, output_file: run.outputFile })
}

export type LintAttemptRecorded = { readonly book: RexBook; readonly stored: RexLintAttempt | null }

/** The first brief of a key is kept; a later one gets the stored attempt back, so lint refuses each key once. */
export function lintAttemptIn(book: RexBook, attempt: RexLintAttempt): LintAttemptRecorded {
  const stored = book.lintAttempts.find(old => old.key === attempt.key)
  if (stored !== undefined) {
    return { book, stored }
  }
  return { book: { ...book, lintAttempts: [...book.lintAttempts, attempt] }, stored: null }
}

export type SkillOpen = {
  readonly skill: string
  readonly file: string
  readonly hash: string
  readonly how: 'attached' | 'rex_skill' | 'read'
}

export function skillOpenIn(book: RexBook, agentId: string, open: SkillOpen, at: number): RexBook {
  const run = findRun(book, agentId)
  const name = `${open.skill}/${open.file}`
  const opened = run === undefined || run.skillsOpened.includes(name) ? book : replaceRun(book, agentId, old => ({ ...old, skillsOpened: [...old.skillsOpened, name] }))
  return queued(opened, at, 'skill.open', agentId, run?.job ?? null, { agent_id: agentId, skill: open.skill, file: open.file, hash: open.hash, how: open.how })
}

export function runOutputIn(book: RexBook, agentId: string, file: string): RexBook {
  return replaceRun(book, agentId, run => ({ ...run, outputFile: file }))
}

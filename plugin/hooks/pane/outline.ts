// The book as one tree of nodes, session, master trees, jobs and their agents, each named by its level, so the pane, the band and /rex tree all agree.
// Pure: the same book and time always give the same outline.
import type { RexBook, RexJob, RexQuestion, RexRun, RexTree } from '../../types'
import { assertNever, isOpen } from '../book/index'
import { treeSpentNano } from '../master/gates'

export type Level = 'session' | 'master' | 'builder' | 'lead' | 'worker' | 'helper'
export type NodeKind = 'session' | 'tree' | 'job' | 'agent'
export type NodeState = 'open' | 'done' | 'lost' | 'blocked' | 'failed' | 'waiting'

export const GLYPH: { readonly [state in NodeState]: string } = { open: '●', done: '✓', lost: '○', blocked: '■', failed: '✗', waiting: '?' }

/** The id of a node is its path from the session, so a node that is gone falls back to the nearest part of its path that exists. */
export type OutlineNode = {
  readonly id: string
  readonly kind: NodeKind
  readonly level: Level
  /** The id of the tree, job or agent, or the session's. */
  readonly ref: string
  /** What a row calls the node: a tree or a job by its id, an agent by its role and task, the session by the start of its id. */
  readonly name: string
  /** What the node is about: a tree's charter or a job's request, cut; empty for an agent and the session. */
  readonly about: string
  readonly status: string
  readonly state: NodeState
  readonly spentNano: number
  readonly capNano: number | null
  readonly openBelow: number
  readonly doneBelow: number
  readonly ranMs: number | null
  /** The questions that wait on the person here or anywhere below, so every level shows what waits. */
  readonly questions: readonly RexQuestion[]
  readonly children: readonly OutlineNode[]
}

export const ROOT_ID = 'session'
const ABOUT_MAX = 60
const SESSION_SHOWN = 8
const LEVELS_OF_TREE: readonly Level[] = ['master', 'builder', 'lead', 'worker', 'helper']
const LEVEL_ORDER: readonly Level[] = ['session', ...LEVELS_OF_TREE]

export function usd(nano: number): string {
  return `$${(nano / 1e9).toFixed(2)}`
}

export function cut(text: string, max: number): string {
  return text.length <= max ? text : `${text.slice(0, Math.max(0, max - 1))}…`
}

export function plural(count: number, one: string, many: string): string {
  return `${String(count)} ${count === 1 ? one : many}`
}

export function duration(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / 1000))
  if (seconds < 60) {
    return `${String(seconds)}s`
  }
  const minutes = Math.floor(seconds / 60)
  return minutes < 60 ? `${String(minutes)}m` : `${String(Math.floor(minutes / 60))}h${String(minutes % 60).padStart(2, '0')}m`
}

function firstLine(text: string): string {
  return (text.split('\n').find(line => line.trim() !== '') ?? '').trim()
}

function firstSentence(text: string): string {
  return firstLine(text).split(/(?<=[.!?])\s/)[0] ?? ''
}

/**
 * The name after an agent's level word: its role, then its task when it says more. A role that is the level word is not said again,
 * so a worker of task T1 is `T1` and a lead with no task has no name. Before a job has a plan its agents have a task and no role,
 * so the name falls back from role to task to grid agent.
 */
export function agentName(run: RexRun, level: Level): string {
  const role = run.role === level ? null : run.role
  const task = run.task === run.role ? null : run.task
  if (role !== null && task !== null) {
    return `${role} ${task}`
  }
  return role ?? task ?? (run.role === null ? run.agentType.replace(/^super-rex:/, '') : '')
}

/** The level word, then the name when there is one: `worker T1`, `lead`. */
export function labelOf(node: OutlineNode): string {
  return node.name === '' ? node.level : `${node.level} ${node.name}`
}

function levelBelow(level: Level): Level {
  switch (level) {
    case 'session':
      return 'master'
    case 'master':
      return 'builder'
    case 'builder':
      return 'lead'
    case 'lead':
      return 'worker'
    case 'worker':
    case 'helper':
      return 'helper'
    default:
      return assertNever(level)
  }
}

function isLive(state: NodeState): boolean {
  return state === 'open' || state === 'waiting'
}

/** Agents below a node: each agent child counts in its own bucket, then what lies below it. */
function agentCounts(children: readonly OutlineNode[]): { readonly openBelow: number; readonly doneBelow: number } {
  return children.reduce(
    (sum, child) => {
      const isAgent = child.kind === 'agent'
      return {
        openBelow: sum.openBelow + child.openBelow + (isAgent && isLive(child.state) ? 1 : 0),
        doneBelow: sum.doneBelow + child.doneBelow + (isAgent && !isLive(child.state) ? 1 : 0),
      }
    },
    { openBelow: 0, doneBelow: 0 },
  )
}

function waiting(book: RexBook, matches: (question: RexQuestion) => boolean): readonly RexQuestion[] {
  return book.questions.filter(question => question.answer === null && matches(question))
}

function runState(run: RexRun): NodeState {
  if (run.state !== 'done') {
    return run.state
  }
  return run.endReason === 'error' || run.endReason === 'refusal' ? 'failed' : 'done'
}

function jobState(job: RexJob, questions: readonly RexQuestion[]): NodeState {
  switch (job.status) {
    case 'created':
    case 'planned':
      return questions.length === 0 ? 'open' : 'waiting'
    case 'blocked':
      return 'blocked'
    case 'failed':
      return 'failed'
    case 'delivered':
      return 'done'
    case 'cancelled':
    case 'lost':
      return 'lost'
    default:
      return assertNever(job.status)
  }
}

function treeState(tree: RexTree, questions: readonly RexQuestion[]): NodeState {
  switch (tree.state) {
    case 'running':
      return questions.length === 0 ? 'open' : 'waiting'
    case 'paused':
    case 'blocked':
      return 'blocked'
    case 'done':
    case 'lost':
      return tree.state
    default:
      return assertNever(tree.state)
  }
}

function agentNode(now: number, members: readonly RexRun[], run: RexRun, parentId: string, level: Level, seen: Set<string>): OutlineNode {
  seen.add(run.agentId)
  const id = `${parentId}/agent:${run.agentId}`
  const children: OutlineNode[] = []
  for (const child of members) {
    if (child.parent === run.agentId && !seen.has(child.agentId)) {
      children.push(agentNode(now, members, child, id, levelBelow(level), seen))
    }
  }
  const status = run.endReason === null ? run.state : `${run.state} (${run.endReason})`
  return { id, kind: 'agent', level, ref: run.agentId, name: agentName(run, level), about: '', status, state: runState(run), spentNano: run.spentNano, capNano: null, ...agentCounts(children), ranMs: (run.endedAt ?? now) - run.startedAt, questions: [], children }
}

function isJobQuestion(job: RexJob, question: RexQuestion): boolean {
  return question.job === job.id || question.mergeFor === job.id
}

/** The agents of a job below its builder; a run whose parent is not among them hangs below the job itself, so none is lost. */
function jobNode(book: RexBook, now: number, job: RexJob, parentId: string): OutlineNode {
  const id = `${parentId}/job:${job.id}`
  const builder = job.owner.builder
  const members = book.runs.filter(run => run.job === job.id && run.agentId !== builder)
  const seen = new Set<string>()
  const memberIds = new Set(members.map(run => run.agentId))
  const tops = members.filter(run => run.parent === builder || !memberIds.has(run.parent))
  const children: OutlineNode[] = []
  for (const run of tops) {
    if (!seen.has(run.agentId)) {
      children.push(agentNode(now, members, run, id, 'lead', seen))
    }
  }
  const questions = waiting(book, question => isJobQuestion(job, question))
  return {
    id,
    kind: 'job',
    level: 'builder',
    ref: job.id,
    name: job.id,
    about: cut(firstSentence(job.request), ABOUT_MAX),
    status: job.status,
    state: jobState(job, questions),
    spentNano: job.spentNano,
    capNano: job.limit.nano,
    ...agentCounts(children),
    ranMs: (isOpen(job) ? now : job.updatedAt) - job.createdAt,
    questions,
    children,
  }
}

function treeNode(book: RexBook, now: number, tree: RexTree): OutlineNode {
  const id = `${ROOT_ID}/tree:${tree.id}`
  const jobs = book.jobs.filter(job => job.tree === tree.id)
  const children = jobs.map(job => jobNode(book, now, job, id))
  const questions = waiting(book, question => question.tree === tree.id || jobs.some(job => isJobQuestion(job, question)))
  return {
    id,
    kind: 'tree',
    level: 'master',
    ref: tree.id,
    name: tree.id,
    about: cut(firstLine(tree.charter), ABOUT_MAX),
    status: tree.state,
    state: treeState(tree, questions),
    spentNano: treeSpentNano(book, tree),
    capNano: tree.capNano,
    ...agentCounts(children),
    ranMs: (tree.closedAt ?? now) - tree.startedAt,
    questions,
    children,
  }
}

export function outlineOf(book: RexBook, now: number): OutlineNode {
  const treeIds = new Set(book.trees.map(tree => tree.id))
  const children = [
    ...book.trees.map(tree => treeNode(book, now, tree)),
    ...book.jobs.filter(job => job.tree === null || !treeIds.has(job.tree)).map(job => jobNode(book, now, job, ROOT_ID)),
  ]
  const questions = waiting(book, () => true)
  const isRunning = children.some(child => isLive(child.state))
  return {
    id: ROOT_ID,
    kind: 'session',
    level: 'session',
    ref: book.session,
    name: book.session.slice(0, SESSION_SHOWN),
    about: '',
    status: book.inert === null ? `storage ${book.storage}` : `off: ${book.inert}`,
    state: questions.length > 0 ? 'waiting' : isRunning ? 'open' : 'done',
    spentNano: children.reduce((sum, child) => sum + child.spentNano, 0),
    capNano: null,
    ...agentCounts(children),
    ranMs: null,
    questions,
    children,
  }
}

/** The nodes from the root down to the node with this id; a missing id gives the path to its nearest parent that exists. */
export function pathTo(root: OutlineNode, id: string): readonly OutlineNode[] {
  const path: OutlineNode[] = [root]
  let node = root
  for (;;) {
    const next = node.children.find(child => id === child.id || id.startsWith(`${child.id}/`))
    if (next === undefined) {
      return path
    }
    path.push(next)
    node = next
  }
}

/** Every node of a subtree, the node first. */
export function flatten(node: OutlineNode): readonly OutlineNode[] {
  return [node, ...node.children.flatMap(flatten)]
}

/** The live trees of this session, in book order: a paused or blocked tree still waits to go on. */
export function liveTrees(book: RexBook, outline: OutlineNode): readonly OutlineNode[] {
  const live = new Set(book.trees.filter(tree => tree.session === book.session && (tree.state === 'running' || tree.state === 'paused' || tree.state === 'blocked')).map(tree => tree.id))
  return outline.children.filter(node => node.kind === 'tree' && live.has(node.ref))
}

/** The levels a run below this node goes through, in order: no master where no tree ran, and a helper only where one ran. */
export function railLevels(top: OutlineNode): readonly Level[] {
  const hasTree = flatten(top).some(node => node.kind === 'tree')
  const hasHelper = flatten(top).some(node => node.level === 'helper')
  return LEVELS_OF_TREE.filter(level => (level !== 'master' || hasTree) && (level !== 'helper' || hasHelper))
}

/** The deepest level among the nodes that `pick` keeps, or null when it keeps none. */
export function deepestLevel(node: OutlineNode, pick: (each: OutlineNode) => boolean): Level | null {
  return flatten(node)
    .filter(pick)
    .reduce<Level | null>((deepest, each) => (deepest === null || LEVEL_ORDER.indexOf(each.level) > LEVEL_ORDER.indexOf(deepest) ? each.level : deepest), null)
}

export function isLiveNode(node: OutlineNode): boolean {
  return isLive(node.state)
}

function agentsPart(node: OutlineNode): readonly string[] {
  if (node.openBelow === 0) {
    return node.doneBelow === 0 ? [] : [`${plural(node.doneBelow, 'agent', 'agents')} done`]
  }
  return [`${plural(node.openBelow, 'agent', 'agents')} open${node.doneBelow === 0 ? '' : `, ${String(node.doneBelow)} done`}`]
}

/** What a session or a tree holds in jobs, said before its agents. */
function jobsPart(node: OutlineNode): readonly string[] {
  if (node.kind === 'tree') {
    return [`${String(node.children.filter(job => job.status === 'delivered').length)}/${String(node.children.length)} jobs delivered`]
  }
  if (node.kind !== 'session') {
    return []
  }
  const trees = node.children.filter(child => child.kind === 'tree').length
  const jobs = flatten(node).filter(each => each.kind === 'job')
  return [...(trees === 0 ? [] : [plural(trees, 'master tree', 'master trees')]), `${plural(jobs.length, 'job', 'jobs')}, ${String(jobs.filter(isLiveNode).length)} open`]
}

/** One node as one line: glyph, level and name, its state and what waits on it, then what it holds, spent and ran, and last what it is about, the part a cut in a narrow pane may lose. */
export function nodeLine(node: OutlineNode): string {
  const spend = node.capNano === null ? usd(node.spentNano) : `${usd(node.spentNano)} of ${usd(node.capNano)}`
  const ran = node.ranMs === null ? [] : [duration(node.ranMs)]
  const asked = node.questions.length === 0 ? [] : [plural(node.questions.length, 'question waits', 'questions wait')]
  const about = node.about === '' ? [] : [node.about]
  return [`${GLYPH[node.state]} ${labelOf(node)}`, node.status, ...asked, ...jobsPart(node), ...agentsPart(node), spend, ...ran, ...about].join(' · ')
}

/** The outline as text lines with tree guides, for the terminal, each cut to `columns` so a line never wraps under the guides. */
export function outlineText(root: OutlineNode, columns: number): readonly string[] {
  const lines = [cut(nodeLine(root), columns)]
  const walk = (node: OutlineNode, prefix: string): void => {
    node.children.forEach((child, index) => {
      const isLast = index === node.children.length - 1
      lines.push(cut(`${prefix}${isLast ? '└─ ' : '├─ '}${nodeLine(child)}`, columns))
      walk(child, `${prefix}${isLast ? '   ' : '│  '}`)
    })
  }
  walk(root, '')
  return lines
}

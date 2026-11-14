// The gate's pure decisions: which job a new agent charges, whether its job is over its limit, whether a spawn is the mod's own, how deep it would run, and whether a shell command merges.
import type { AgentSpawnInput } from 'claude-code'

import type { RexBook, RexJob, RexRun } from '../../types'
import { isOpen } from '../book/index'

/** The job a loop works for: the open job it owns, else its run's job; the main loop's own job when `agentId` is absent. */
export function jobOfLoop(book: RexBook, agentId: string | undefined): RexJob | undefined {
  const builder = agentId ?? 'main'
  const owned = book.jobs.find(job => isOpen(job) && job.owner.session === book.session && job.owner.builder === builder)
  if (owned !== undefined || agentId === undefined) {
    return owned
  }
  const runJob = book.runs.find(run => run.agentId === agentId)?.job
  return book.jobs.find(job => job.id === runJob)
}

/**
 * The mod's own spawn of a master or a builder: from this plugin, with no
 * parent agent, and with the pending entry `/rex master` or `rex_build` left
 * for its description after their own checks. Anything else gets the checks.
 */
export function isOwnSpawn(book: RexBook, event: AgentSpawnInput, origin: string | undefined): boolean {
  return origin === 'super-rex' && event.parentAgentId === undefined && book.pendingSpawns.some(spawn => spawn.description === event.description)
}

function dollars(nano: number): string {
  return (nano / 1e9).toFixed(2)
}

export function isOverLimit(job: RexJob): boolean {
  return job.spentNano >= job.limit.nano
}

export function spendRefusal(job: RexJob): string {
  return `rex gate: job ${job.id} has spent $${dollars(job.spentNano)} of its $${dollars(job.limit.nano)} limit; a person raises it with /rex limit ${job.id} <usd>`
}

export class UnknownParentError extends Error {
  readonly agentId: string

  constructor(agentId: string) {
    super(`the book holds no run of agent ${agentId}, an ancestor of a run it holds, so the chain cannot be followed`)
    this.name = 'UnknownParentError'
    this.agentId = agentId
  }
}

function runOf(book: RexBook, agentId: string): RexRun {
  const run = book.runs.find(each => each.agentId === agentId)
  if (run === undefined) {
    throw new UnknownParentError(agentId)
  }
  return run
}

/** Only a master tree may run agents this deep: the master, its builders at depth 1, then leads, workers and their fan-out. */
export const TREE_DEPTH = 4

/** `depth` is null when the book holds no run of the parent, as when the observer failed to record it. */
export type DepthVerdict = { readonly depth: number | null; readonly refusal: string | null }

/**
 * The engine depth a child of `parentAgentId` gets, and the refusal when it is
 * 4 or more and the chain's depth-1 agent is not a builder of a master tree.
 * A parent the book does not hold gives an unknown depth and no refusal: a
 * failed observer must not block a spawn, and the engine's own depth limit
 * still holds. A gap higher up the chain is a broken book and throws.
 */
export function depthVerdict(book: RexBook, parentAgentId: string | undefined): DepthVerdict {
  if (parentAgentId === undefined) {
    return { depth: 1, refusal: null }
  }
  const parent = book.runs.find(each => each.agentId === parentAgentId)
  if (parent === undefined) {
    return { depth: null, refusal: null }
  }
  const depth = parent.depth + 1
  if (depth < TREE_DEPTH) {
    return { depth, refusal: null }
  }
  let top = parent
  while (top.depth > 1) {
    top = runOf(book, top.parent)
  }
  const topId = top.agentId
  const isTreeBuilder = book.trees.some(tree => tree.builders.some(builder => builder.agentId === topId))
  return { depth, refusal: isTreeBuilder ? null : `rex gate: depth ${String(depth)} is for master trees only; start this work one layer up` }
}

/** The grid agent a fork should have been: the parent's planned one, else the general one. */
export function forkRefusal(book: RexBook, job: RexJob, parentAgentId: string | undefined): string {
  const parent = parentAgentId === undefined ? undefined : book.runs.find(run => run.agentId === parentAgentId)
  const planned = job.plan?.roles.find(role => role.role === parent?.role)?.gridAgent ?? parent?.planned ?? 'model-high-effort-high'
  return `rex: a fork cannot change model; start a grid agent instead, subagent_type super-rex:${planned}`
}

/** Where one command of a line ends and the next begins, command substitution included. */
const SEGMENT_SPLIT = /&&|\|\||;|\||\n|&|\$\(|`|<\(|>\(/
const ASSIGNMENT = /^[A-Za-z_][A-Za-z0-9_]*=/
const COUNT_OR_DURATION = /^\d+(\.\d+)?[smhd]?$/

/** Programs and shell words that run the command after them; the values each one's options take. */
const WRAPPERS: ReadonlyMap<string, ReadonlySet<string>> = new Map([
  ['env', new Set(['-u', '--unset', '-C', '--chdir', '-S', '--split-string'])],
  ['command', new Set<string>()],
  ['builtin', new Set<string>()],
  ['exec', new Set(['-a'])],
  ['nohup', new Set<string>()],
  ['time', new Set<string>()],
  ['nice', new Set(['-n'])],
  ['timeout', new Set(['-s', '--signal', '-k', '--kill-after'])],
  ['xargs', new Set(['-I', '-L', '-n', '-P', '-s', '-d', '-E', '-a'])],
  ['sudo', new Set(['-u', '-g', '-C', '-D', '-h', '-p', '-U'])],
  ['doas', new Set(['-u', '-C'])],
  ['stdbuf', new Set(['-i', '-o', '-e'])],
  ['eval', new Set<string>()],
  ['if', new Set<string>()],
  ['then', new Set<string>()],
  ['else', new Set<string>()],
  ['elif', new Set<string>()],
  ['do', new Set<string>()],
  ['while', new Set<string>()],
  ['until', new Set<string>()],
  ['!', new Set<string>()],
  ['{', new Set<string>()],
])

/** Shells whose `-c` option, alone or among others such as `-lc`, takes the command as its words. */
const SHELLS = new Set(['sh', 'bash', 'zsh', 'dash', 'ksh', 'mksh', 'fish'])

/** Options of git itself, before its sub-command, that take the next word as their value. */
const GIT_VALUES = new Set(['-C', '-c', '--git-dir', '--work-tree', '--namespace', '--config-env', '--super-prefix'])

/** Options of git push that take the next word as their value. */
const PUSH_VALUES = new Set(['--repo', '-o', '--push-option', '--receive-pack', '--exec'])

/** A refspec target whose real name only the shell or git works out: a variable, a glob, a brace list. */
const UNDECIDED_TARGET = /[$*?[{`]/

/** The words of one command as the program sees them: quotes and backslashes gone, a subshell's brackets gone. */
function wordsOf(segment: string): readonly string[] {
  return segment
    .trim()
    .split(/\s+/)
    .map(word => word.replace(/["'\\]/g, '').replace(/^\(+/, '').replace(/\)+$/, ''))
    .filter(word => word.length > 0)
}

function baseName(program: string): string {
  return program.slice(program.lastIndexOf('/') + 1)
}

/** The words after a wrapper's own options and their values. */
function afterOptions(words: readonly string[], valued: ReadonlySet<string>): readonly string[] {
  let index = 0
  while (index < words.length) {
    const word = words[index] ?? ''
    if (word === '--') {
      return words.slice(index + 1)
    }
    if (!word.startsWith('-') && !COUNT_OR_DURATION.test(word)) {
      break
    }
    index += valued.has(word) ? 2 : 1
  }
  return words.slice(index)
}

/**
 * The words of the command a line finally runs: past assignments, wrappers
 * such as `env`, `timeout` or `xargs`, `eval`, and `sh -c`. A shell that runs a
 * script file is left as it is: the tripwire cannot read the script.
 */
function commandWords(words: readonly string[]): readonly string[] {
  let rest = words
  for (;;) {
    const start = rest.findIndex(word => !ASSIGNMENT.test(word))
    rest = start === -1 ? [] : rest.slice(start)
    const program = baseName(rest[0] ?? '')
    const valued = WRAPPERS.get(program)
    if (valued !== undefined) {
      rest = afterOptions(rest.slice(1), valued)
      continue
    }
    const flag = SHELLS.has(program) ? rest.findIndex((word, index) => index > 0 && /^-[a-z]*c[a-z]*$/.test(word)) : -1
    if (flag !== -1) {
      rest = rest.slice(flag + 1)
      continue
    }
    return rest.length === 0 ? [] : [program, ...rest.slice(1)]
  }
}

/** The arguments of `git ... push`, past git's own options such as `-C <dir>`; null when the words are no git push. */
function pushArgs(words: readonly string[]): readonly string[] | null {
  if (words[0] !== 'git') {
    return null
  }
  let index = 1
  while (index < words.length && (words[index] ?? '').startsWith('-')) {
    index += GIT_VALUES.has(words[index] ?? '') ? 2 : 1
  }
  return words[index] === 'push' ? words.slice(index + 1) : null
}

/**
 * True when the refspec may update `base`: it names it, or its target is one
 * that only the shell or git resolves, or it is `HEAD` (the current branch may
 * be the base), or it is empty (`:` pushes every matching branch).
 */
function mayNameBase(refspec: string, base: string): boolean {
  const spec = refspec.startsWith('+') ? refspec.slice(1) : refspec
  const target = spec.slice(spec.lastIndexOf(':') + 1)
  const isCurrent = target === 'HEAD' || target === '@'
  return target === base || target === `refs/heads/${base}` || target === '' || isCurrent || UNDECIDED_TARGET.test(target)
}

/** True when a push may update `base`. A push that names no branch pushes what git's settings pick, which can be the base, so it counts. */
function pushMayReachBase(args: readonly string[], base: string): boolean {
  const plain: string[] = []
  let hasRepoOption = false
  let isTagsOnly = false
  for (let index = 0; index < args.length; index += 1) {
    const word = args[index] ?? ''
    if (word === '--') {
      plain.push(...args.slice(index + 1))
      break
    }
    if (!word.startsWith('-')) {
      plain.push(word)
      continue
    }
    if (word === '--all' || word === '--branches' || word === '--mirror') {
      return true
    }
    hasRepoOption ||= word === '--repo' || word.startsWith('--repo=')
    isTagsOnly ||= word === '--tags'
    index += PUSH_VALUES.has(word) ? 1 : 0
  }
  const refspecs = hasRepoOption ? plain : plain.slice(1)
  if (refspecs.length === 0) {
    return !isTagsOnly
  }
  return refspecs.some(refspec => mayNameBase(refspec, base))
}

/** A pull request merge through the GitHub API, by `gh api` or any other client. */
const API_MERGE = /\/pulls\/[^/\s]+\/merge\b|\/repos\/[^/\s]+\/[^/\s]+\/merges\b/
const GRAPHQL_MERGE = /\bmerge(?:PullRequest|Branch)\b/

function segmentMerges(segment: string, base: string): boolean {
  const words = commandWords(wordsOf(segment))
  const push = pushArgs(words)
  if (push !== null) {
    return pushMayReachBase(push, base)
  }
  const ghCommand = words[0] === 'gh' ? words.slice(1).filter(word => !word.startsWith('-')) : []
  if (ghCommand[0] === 'pr' && ghCommand[1] === 'merge') {
    return true
  }
  if (ghCommand[0] === 'api' && (/\/merges?\b/.test(segment) || GRAPHQL_MERGE.test(segment))) {
    return true
  }
  return words.some(word => API_MERGE.test(word))
}

/** True when one command of the line would merge into `base` some way other than rex_merge, or may do so and the line cannot tell. */
export function isMergeCommand(command: string, base: string): boolean {
  return command.split(SEGMENT_SPLIT).some(segment => segmentMerges(segment, base))
}

export function tripwireRefusal(command: string, base: string): string {
  return `rex gate: merges reach ${base} only through rex_merge, got ${command.slice(0, 80)}`
}

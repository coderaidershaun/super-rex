// Test support for the master module: answers what a master tree asks of the engine (spawns, messages, prompts, settings, environment, store, usage), records each ask, and opens a tree as the person's prompt of the master skill does.
import { mock } from 'claude-code/testing'
import type { MockClock } from 'claude-code/testing'
import type { AgentInfo, AgentSpawnInput, CommandRunInput, On, PromptOrigin, PromptSubmitInput, SessionRateLimit, SessionSendInput, ToolCallResult, TurnStepResult, TurnUsage } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { emptyBook, newIngest } from '../book/events'
import { openJobIn } from '../book/jobs'
import { CONFIG_ON, ROOT, SESSION, START_MS, world } from '../book/kit'
import type { ProcessAnswer, ProcessCall, World, WorldOptions } from '../book/kit'
import { treeIn } from '../book/trees'
import { DEFAULT_POLICY } from './policy'
import type { Policy } from './policy'
import { policyHash } from './store'

export const BUILDER_TEXT = '# Builder brief\nYou are a builder that a master started.\n'
/** The store key of the policy of ROOT: `policy:` and the sha256 of `/home/person/repo`. */
export const POLICY_KEY = 'policy:7b89ea27fe6655eb760c328e5d86092d3940fc708f91ba5edea62aa46516820f'

export type MasterOptions = WorldOptions & {
  /** The variables `$.env.get` answers; depth 4 when not given. */
  readonly env?: Readonly<Record<string, string>>
  /** The plugin store at the start, by key; the policy of ROOT goes under `policyKey`. */
  readonly store?: Readonly<Record<string, unknown>>
  /** The `allow` rules of the merged settings. */
  readonly allow?: readonly string[]
  readonly rateLimits?: readonly SessionRateLimit[]
  /** Makes `$.session.usage()` fail with this text. */
  readonly usageRefusal?: string
  /** Refuses a spawn with this text; null lets it start. */
  readonly spawnRefusal?: (e: AgentSpawnInput) => string | null
  /** Files by absolute path beside the builder's brief file of the master skill. */
  readonly files?: Readonly<Record<string, string>>
}

/** A spawn as the test saw it. The kit hands a plugin's own spawn over in the Agent tool's spelling, so each field is read in either. */
export type SeenSpawn = { readonly type: string; readonly description: string; readonly prompt: string; readonly cwd: string | null }

function textField(e: object, ...keys: readonly string[]): string | null {
  for (const key of keys) {
    const value: unknown = Reflect.get(e, key)
    if (typeof value === 'string') {
      return value
    }
  }
  return null
}

function seenSpawn(e: AgentSpawnInput): SeenSpawn {
  return { type: textField(e, 'subagentType', 'subagent_type') ?? '', description: e.description, prompt: e.prompt, cwd: textField(e, 'cwd') }
}

export type MasterWorld = World & {
  /** The name of each agent type the mod asked the engine to register; the mod registers none. */
  readonly registered: string[]
  readonly spawned: SeenSpawn[]
  /** The messages that woke a builder; the master is never sent one. */
  readonly sent: SessionSendInput[]
  /** Every prompt that reached the session past the plugin, in order. */
  readonly prompts: PromptSubmitInput[]
  /** The text of each prompt of this plugin that reached the session: each wake of the master, which is the main loop. */
  woken: () => readonly string[]
  readonly written: { readonly path: string; readonly text: string }[]
  readonly agents: AgentInfo[]
  /** Makes every later read of the plugin store fail. */
  breakStore: () => void
}

/** A policy as the store holds it after the person confirmed it: the default with the keys given, and its own hash. */
export async function confirmedPolicy(keys: Partial<Policy> = {}): Promise<Policy> {
  const policy: Policy = { ...DEFAULT_POLICY, ...keys, confirmed_hash: null }
  return { ...policy, confirmed_hash: await policyHash(policy) }
}

const PRESENTATION = { isFullscreen: false, columns: 120 } as const

/** A `/rex` line as the person types it, or from the origin given. */
export function rex(args: string, origin: PromptOrigin = { kind: 'composer' }): CommandRunInput {
  return { command: 'rex', args, origin, presentation: PRESENTATION }
}

function agentIdOf(e: AgentSpawnInput, count: number): string {
  if (e.description.startsWith('rex builder ')) {
    return `b${String(count)}`
  }
  return `a${String(count)}`
}

/** Registers the master's world beneath the plugin, then the shared world; call it before any other hook of the test that answers the same events. */
export function masterWorld(on: On, options: MasterOptions = {}): MasterWorld {
  const registered: string[] = []
  const spawned: SeenSpawn[] = []
  const sent: SessionSendInput[] = []
  const prompts: PromptSubmitInput[] = []
  const written: { path: string; text: string }[] = []
  const agents: AgentInfo[] = []
  const tree: Record<string, string> = { ...(options.files ?? {}) }
  let spawns = 0

  on('fs.read', { path: /\/(skills\/orchestration-master|\.rex\/plans)\// }, (_$, e) => {
    if (e.path.endsWith('/skills/orchestration-master/builder.md')) {
      return { value: BUILDER_TEXT }
    }
    const text = tree[e.path]
    return text === undefined ? { deny: `ENOENT: no file at ${e.path}` } : { value: text }
  })
  on('fs.read', { path: /^\/home\/person\/repo\/(?!\.rex\/config\.json)/ }, (_$, e) => {
    const text = tree[e.path]
    return text === undefined ? { deny: `ENOENT: no file at ${e.path}` } : { value: text }
  })
  on('fs.exists', { path: /\.rex\/plans\// }, (_$, e) => ({ value: tree[e.path] !== undefined }))
  on('fs.write', (_$, e) => {
    written.push({ path: e.path, text: e.text })
    tree[e.path] = e.text
    return { value: undefined }
  })
  on('fs.stat', (_$, e) => ({ value: { kind: 'dir', size: 0, mtimeMs: 0, isLink: false, realPath: e.path } }))
  mock.env(on, options.env ?? { CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH: '4' })
  let isStoreBroken = false
  on('store.get', { key: /(?:)/ }, (_$, e, next) => (isStoreBroken ? { deny: 'the store could not be read' } : next(e)))
  mock.store(on, options.store ?? {})
  on('settings.read', () => ({ value: { permissions: { allow: [...(options.allow ?? [])] } } }))
  on('session.usage', () =>
    options.usageRefusal === undefined ? { value: { startedAt: 0, context: { window: 200_000 }, rateLimits: [...(options.rateLimits ?? [])] } } : { deny: options.usageRefusal },
  )
  on('agent.register', (_$, e) => {
    registered.push(e.name)
    return { value: { agent: `super-rex:${e.name}` } }
  })
  on('agent.spawn', (_$, e) => {
    spawned.push(seenSpawn(e))
    const refusal = options.spawnRefusal?.(e) ?? null
    if (refusal !== null) {
      return { deny: refusal }
    }
    spawns += 1
    const agentId = agentIdOf(e, spawns)
    agents.push({ id: agentId, description: e.description, type: seenSpawn(e).type, status: 'running', spawnedBy: 'super-rex' })
    return { model: 'claude-opus-5-5', agentId }
  })
  on('agent.list', () => ({ value: [...agents] }))
  on('session.send', (_$, e) => {
    sent.push(e)
    return { isDelivered: true }
  })
  on('prompt.submit', (_$, e) => {
    prompts.push(e)
    return { text: e.text }
  })

  on('turn.step', async function* (_$, e) {
    const result: TurnStepResult = { turnId: e.turnId, index: e.index, answer: '', toolUses: [], stopReason: 'end_turn', usage: TURN_USAGE }
    return result
  })
  on('turn.complete', (_$, e) => ({ text: e.answer }))
  on('tool.call', { tool: 'Agent' }, (_$, e) => ({ result: `started ${String(e.description)}` }))

  // With no answers of its own, a test runs in a repo of mode origin, whose places a master's start reads.
  const shared = world(on, { config: CONFIG_ON, processAnswer: byRule(placesRules('origin')), ...options })
  const breakStore = (): void => {
    isStoreBroken = true
  }
  const woken = (): readonly string[] => prompts.filter(e => e.origin.kind === 'plugin' && e.origin.name === 'super-rex').map(e => e.text)
  return { ...shared, registered, spawned, sent, prompts, woken, written, agents, breakStore }
}

export const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
export const HEADLESS_START = { cwd: ROOT, surface: 'terminal', isInteractive: false } as const

/** What every model request of a test costs: one million input tokens of Sonnet, $2 by the shared prices table. */
export const TURN_USAGE: TurnUsage = { model: 'claude-sonnet-5-5', input_tokens: 1_000_000, output_tokens: 0, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 }

export const TREE = 't1'

/** The classification every rex_build of a test carries, which the tool requires; a test that needs another size or type gives it after. */
export const KIND = { type: 'fix', size: 'medium', langs: ['rust'] } as const

/** A book of this session with storage on and no tree, as a reload finds it. */
export function sessionBook(): RexBook {
  return { ...emptyBook(), session: SESSION, storage: 'on', versions: { mod: '0.8.0', binary: '0.8.0', claudeCode: '2.1.323' }, ingest: { ...newIngest('aaaaaaaaaaaaaaaa', '1'.repeat(64)), acked: 0 } }
}

/** A book after rex_tree_open opened tree t1 under the policy given, its master the main loop of this session, with storage on, as a reload finds it. */
export async function treeBook(policy: Policy): Promise<RexBook> {
  return treeIn(sessionBook(), { id: TREE, charter: 'Ship the parser.\n\n## Constraints\n\n- Keep infra/ as it is.\n', charterHash: 'sha256:c', policyHash: await policyHash(policy), state: 'running', capNano: Math.round(policy.tree_usd * 1e9) }, START_MS)
}

/** The job the main loop of this session builds, opened from the person's own job-start prompt. */
export const MAIN_JOB = '0gaq820009'

/** A book of this session whose main loop builds job MAIN_JOB, with no tree. */
export function mainJobBook(): RexBook {
  const input = {
    builder: 'main',
    tree: null,
    origin: 'composer',
    request: 'Fix the lexer',
    repo: ROOT,
    base: 'main',
    type: 'fix',
    size: 'small',
    consequence: null,
    langs: ['rust'],
    prefs: { duration: null, budget: null, autonomy: 'high' },
    limit: { nano: 10_000_000_000, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  return openJobIn(sessionBook(), input, MAIN_JOB, START_MS).book
}

export const MASTER_SKILL = 'super-rex:orchestration-master'
export const TREE_OPEN = 'mcp__super-rex__rex_tree_open'

/** The person's prompt of the master skill as the engine raises it: `args` is the goal, `@<file>` or `resume`. */
export function masterPrompt(args: string, origin: PromptOrigin = { kind: 'composer' }, name: string = MASTER_SKILL): PromptSubmitInput {
  return { text: `/${name} ${args}`.trimEnd(), command: { name, args }, origin, wait: false }
}

/** What the session does after the person's prompt of the master skill: the prompt enters, then the main loop calls rex_tree_open. */
export async function openedTree($: Engine, args: string, origin: PromptOrigin = { kind: 'composer' }): Promise<ToolCallResult> {
  await $.prompt.submit(masterPrompt(args, origin))
  return $.tool.call({ tool: TREE_OPEN })
}

/** One answer of a process call, picked by the first rule whose words start the call's argv; `rex ingest` always gets its last sequence. */
export type Rule = { readonly starts: readonly string[]; readonly answer: ProcessAnswer }

export function byRule(rules: readonly Rule[], seen: ProcessCall[] = []): (call: ProcessCall) => ProcessAnswer {
  let last = 0
  return call => {
    if (call.argv[0] === 'rex' && call.argv[1] === 'ingest') {
      const sent: unknown = JSON.parse(call.stdin ?? '{}')
      const events: unknown = typeof sent === 'object' && sent !== null ? Reflect.get(sent, 'events') : undefined
      const seqs = Array.isArray(events) ? events.map((event: unknown) => (typeof event === 'object' && event !== null ? Number(Reflect.get(event, 'seq')) : 0)) : []
      last = Math.max(last, ...seqs)
      return { exitCode: 0, stdout: `last_sequence=${String(last)}\n`, stderr: '' }
    }
    seen.push(call)
    const rule = rules.find(each => each.starts.every((word, index) => call.argv[index] === word))
    return rule?.answer ?? { reject: `no rule for ${call.argv.join(' ')}` }
  }
}

/**
 * Settles the clock until `isDone` holds, or until `tries` settles ran. The
 * engine answers each call of a hook between two settles, so on a busy machine
 * one settle can return before the work that a timer started has ended.
 */
export async function settleUntil(clock: MockClock, isDone: () => boolean, tries = 1_000): Promise<void> {
  for (let tried = 0; tried < tries && !isDone(); tried += 1) {
    await clock.settle()
  }
}

/** Settles several times, for a test that checks that nothing more happened. */
export function settleLong(clock: MockClock): Promise<void> {
  return settleUntil(clock, () => false, 10)
}

export function ran(stdout = ''): ProcessAnswer {
  return { exitCode: 0, stdout, stderr: '' }
}

export function lineOf(result: unknown, key: string): string | undefined {
  const text = typeof result === 'object' && result !== null ? Reflect.get(result, 'result') : undefined
  return typeof text === 'string' ? text.split('\n').find(line => line.startsWith(`${key}=`))?.slice(key.length + 1) : undefined
}

export function resultText(result: unknown): string {
  const text = typeof result === 'object' && result !== null ? Reflect.get(result, 'result') : undefined
  return typeof text === 'string' ? text : ''
}

export function omsAnswer(agent: string): ProcessAnswer {
  return ran(`${JSON.stringify({ agent, demand: agent, rules: [], conflict: [], inputs: '-', version: 2 })}\n`)
}

/** The full hash `git rev-parse origin/main` answers in a test: the commit each tree job's worktree starts from. */
export const START_COMMIT = '5a1f0c9e2b7d4a6f8c3e1b0d9a7f5c3e1b2d4f6a'

/** The git calls that read the places of the session root: a repo with a commit, `main` checked out and, in mode `origin`, held on origin. */
export function placesRules(remote: 'origin' | 'none'): readonly Rule[] {
  return [
    { starts: ['git', '-C', ROOT, 'rev-parse', '--verify', '--quiet', 'HEAD'], answer: ran('c0\n') },
    { starts: ['git', '-C', ROOT, 'symbolic-ref', '--quiet', '--short', 'HEAD'], answer: ran('main\n') },
    { starts: ['git', '-C', ROOT, 'remote'], answer: ran(remote === 'origin' ? 'origin\n' : '') },
    { starts: ['git', '-C', ROOT, 'symbolic-ref', '--short', 'refs/remotes/origin/HEAD'], answer: ran('origin/main\n') },
    { starts: ['git', '-C', ROOT, 'for-each-ref'], answer: ran('refs/remotes/origin/HEAD\nrefs/remotes/origin/main\n') },
    { starts: ['git', '-C', ROOT, 'status', '--porcelain'], answer: ran() },
  ]
}

/** The git calls of a session root with no commit, whose first commit answers `commit`; they go before `buildRules`. */
export function firstCommitRules(commit: ProcessAnswer = ran('[master (root-commit) c0] chore: first commit\n')): readonly Rule[] {
  return [
    { starts: ['git', '-C', ROOT, 'rev-parse', '--verify', '--quiet', 'HEAD'], answer: { exitCode: 1, stdout: '', stderr: '' } },
    { starts: ['git', '-C', ROOT, 'write-tree'], answer: ran('4b825dc642cb6eb9a060e54bf8d69288fbee4904\n') },
    { starts: ['git', '-C', ROOT, 'add', '--all'], answer: ran() },
    { starts: ['git', '-C', ROOT, 'read-tree'], answer: ran() },
    { starts: ['git', '-C', ROOT, 'commit', '--allow-empty', '--message', 'chore: first commit, the base of rex jobs'], answer: commit },
  ]
}

/** The process calls of one rex_build in the session root, the grid agent `rex oms` gives named; mode `origin` unless `remote` says `none`. */
export function buildRules(agent: string, remote: 'origin' | 'none' = 'origin'): readonly Rule[] {
  return [
    { starts: ['rex', 'oms'], answer: omsAnswer(agent) },
    ...placesRules(remote),
    { starts: ['git', '-C', ROOT, 'fetch'], answer: ran() },
    { starts: ['git', '-C', ROOT, 'rev-parse', 'origin/main'], answer: ran(`${START_COMMIT}\n`) },
    { starts: ['git', '-C', ROOT, 'rev-parse', 'main'], answer: ran(`${START_COMMIT}\n`) },
    { starts: ['rex', 'worktree', 'add'], answer: ran() },
    { starts: ['rex', 'worktree', 'remove'], answer: ran() },
  ]
}

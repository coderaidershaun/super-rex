// A session for the mod's tests: answers every call the mod makes on the engine from memory, and records what the mod wrote, showed and ran.
import { mock } from 'claude-code/testing'
import type { MockClock } from 'claude-code/testing'
import type { AgentInfo, AgentSpawnInput, ModelCompleteInput, ModelCompleteResult, On, ProcessRunResult, StateRead } from 'claude-code'

import type { RexBook } from '../../types'
import { BOOK_SHAPE } from './api'

export const SESSION = '00000000-0000-4000-8000-000000000001'
export const ROOT = '/home/person/repo'
/** 2026-11-13T20:00:00.000Z, the time of the shared fixtures. */
export const START_MS = 1_794_600_000_000
export const CONFIG_ON = '{ "version": 1, "storage": "on", "depth": 4, "jobs_from_sdk": true, "min_claude_code": "2.1.323" }'
export const CONFIG_OFF = '{ "version": 1, "storage": "off", "depth": 4, "jobs_from_sdk": true, "min_claude_code": "2.1.323" }'
/** A repo after `rex init --auto-merge`. */
export const CONFIG_AUTO_MERGE = '{ "version": 1, "storage": "on", "auto_merge": true, "depth": 4, "jobs_from_sdk": true, "min_claude_code": "2.1.323" }'
export const PLUGIN_VERSION = '0.8.0'

export type ProcessCall = {
  readonly argv: readonly string[]
  readonly stdin: string | undefined
  readonly cwd: string | undefined
  readonly env: Readonly<Record<string, string>> | undefined
  readonly timeoutMs: number | undefined
}
export type ProcessAnswer = { readonly exitCode: number; readonly stdout: string; readonly stderr: string } | { readonly reject: string }
export type ModelAnswer = ModelCompleteResult | { readonly reject: string }

export type WorldOptions = {
  /** The text of `.rex/config.json`; null when the file is missing. */
  readonly config?: string | null
  /** A book that the session's state holds before the mod first reads it, as after a reload. */
  readonly seed?: RexBook
  readonly processAnswer?: (call: ProcessCall) => ProcessAnswer
  readonly modelAnswer?: (input: ModelCompleteInput) => ModelAnswer
  /** What `rex --version` prints; null makes the call fail. It is kept apart from the other calls. */
  readonly binaryVersion?: string | null
  /** What `$.agent.list` answers at each call; when not given, the agent of each open run of the seed, as a reload finds them still running. */
  readonly listAgents?: () => readonly AgentInfo[]
}

export type World = {
  readonly clock: MockClock
  readonly statuses: string[]
  /** The status line on screen now: the last one set, undefined once it is cleared. */
  statusLine: () => string | undefined
  readonly calls: ProcessCall[]
  readonly versionCalls: ProcessCall[]
  readonly completions: ModelCompleteInput[]
  readonly opened: string[]
  readonly commands: string[]
  readonly tools: string[]
  /** Each write of a live snapshot below `.rex/live/`, in order. */
  readonly liveWrites: { readonly path: string; readonly text: string }[]
  /** Makes the next write of a live snapshot fail, as a folder that cannot be written would. */
  failNextLiveWrite: () => void
  /** Makes the mod's next read of its book fail once, as a broken read would. */
  failNextBookRead: () => void
  /** The book as the mod last wrote it. */
  book: () => RexBook
  setProcessAnswer: (answer: (call: ProcessCall) => ProcessAnswer) => void
}

export function ingestAnswer(lastSequence: number): ProcessAnswer {
  return { exitCode: 0, stdout: `last_sequence=${String(lastSequence)}\n`, stderr: '' }
}

export function world(on: On, options: WorldOptions = {}): World {
  const statuses: string[] = []
  let statusLine: string | undefined
  const calls: ProcessCall[] = []
  const versionCalls: ProcessCall[] = []
  const completions: ModelCompleteInput[] = []
  const opened: string[] = []
  const commands: string[] = []
  const tools: string[] = []
  const liveWrites: { readonly path: string; readonly text: string }[] = []
  let isLiveWriteBroken = false
  const books: RexBook[] = []
  let isSeedPending = options.seed !== undefined
  let isReadBroken = false
  let processAnswer = options.processAnswer ?? ((call: ProcessCall): ProcessAnswer => ({ reject: `no answer for ${call.argv.join(' ')}` }))
  const config = options.config === undefined ? CONFIG_OFF : options.config

  on('state.get', { plugin: 'super-rex', key: 'book' }, (_$, e, next) => {
    if (isReadBroken) {
      isReadBroken = false
      return { deny: 'the book could not be read' }
    }
    if (isSeedPending && options.seed !== undefined) {
      isSeedPending = false
      const seeded: StateRead = { value: { shape: BOOK_SHAPE, value: options.seed }, version: 0 }
      return { value: seeded }
    }
    return next(e)
  })
  on('state.set', { plugin: 'super-rex', key: 'book' }, (_$, e, next) => {
    books.push(e.value.value)
    // SMELL: the compiler gives up on the depth of the book's type where it checks this call; the call itself is the plain pass-through every hook makes.
    // @ts-expect-error TS2589: type instantiation of the book's state event is too deep for the compiler
    return next(e)
  })
  const clock = mock.clock(on, { now: START_MS })
  on('session.id', () => ({ value: SESSION }))
  on('session.root', () => ({ value: ROOT }))
  on('session.version', () => ({ value: { version: '2.1.323', base: '2.1.323' } }))
  on('fs.exists', (_$, e) => ({ value: e.path === `${ROOT}/.rex/config.json` ? config !== null : e.path.endsWith('/.claude-plugin/plugin.json') }))
  on('fs.read', (_$, e) => {
    if (e.path === `${ROOT}/.rex/config.json` && config !== null) {
      return { value: config }
    }
    if (e.path.endsWith('/.claude-plugin/plugin.json')) {
      return { value: JSON.stringify({ name: 'super-rex', version: PLUGIN_VERSION }) }
    }
    return { deny: `no file at ${e.path}` }
  })
  on('fs.stat', { path: /^\/home\/person\/repo$/ }, (_$, e) => ({ value: { kind: 'dir', size: 0, mtimeMs: 0, isLink: false, realPath: e.path } }))
  on('fs.write', { path: /\/\.rex\/live\// }, (_$, e) => {
    if (isLiveWriteBroken) {
      isLiveWriteBroken = false
      return { deny: `EACCES: permission denied, open ${e.path}` }
    }
    liveWrites.push({ path: e.path, text: e.text })
    return { value: undefined }
  })
  on('ui.status', (_$, e) => {
    statusLine = e.text
    if (e.text !== undefined) {
      statuses.push(e.text)
    }
    return { value: undefined }
  })
  on('ui.open', (_$, e) => {
    opened.push(e.id)
    return { value: { isPlaced: true } }
  })
  on('command.register', (_$, e) => {
    commands.push(e.name)
    return { value: { command: e.name } }
  })
  on('tool.register', (_$, e) => {
    tools.push(e.name)
    return { value: { tool: `mcp__super-rex__${e.name}` } }
  })
  on('process.run', (_$, e) => {
    const call: ProcessCall = { argv: e.argv, stdin: e.init?.stdin, cwd: e.init?.cwd, env: e.init?.env, timeoutMs: e.init?.timeoutMs }
    const isVersion = e.argv.length === 2 && e.argv[0] === 'rex' && e.argv[1] === '--version'
    if (isVersion) {
      versionCalls.push(call)
    } else {
      calls.push(call)
    }
    const version = options.binaryVersion === undefined ? 'rex 0.8.0\n' : options.binaryVersion
    const answer: ProcessAnswer = isVersion
      ? version === null
        ? { exitCode: 127, stdout: '', stderr: 'rex: command not found\n' }
        : { exitCode: 0, stdout: version, stderr: '' }
      : processAnswer(call)
    if ('reject' in answer) {
      return { deny: answer.reject }
    }
    const result: ProcessRunResult = { ...answer, isStdoutTruncated: false, isStderrTruncated: false }
    return { value: result }
  })
  on('model.complete', (_$, e) => {
    completions.push(e)
    const answer = options.modelAnswer?.(e) ?? { reject: 'no model in this test' }
    return 'reject' in answer ? { deny: answer.reject } : { value: answer }
  })
  on('session.start', (_$, e) => ({ cwd: e.cwd }))
  const seededAgents: readonly AgentInfo[] = (options.seed?.runs ?? []).filter(run => run.state === 'open').map(run => ({ id: run.agentId, description: run.agentId, type: run.agentType, status: 'running' }))
  on('agent.list', () => ({ value: [...(options.listAgents?.() ?? seededAgents)] }))

  return {
    clock,
    statuses,
    statusLine: () => statusLine,
    calls,
    versionCalls,
    completions,
    opened,
    commands,
    tools,
    liveWrites,
    failNextLiveWrite: () => {
      isLiveWriteBroken = true
    },
    failNextBookRead: () => {
      isReadBroken = true
    },
    book: () => {
      const last = books[books.length - 1]
      if (last === undefined) {
        throw new Error('the mod has written no book yet')
      }
      return last
    },
    setProcessAnswer: answer => {
      processAnswer = answer
    },
  }
}

/** A spawn as the engine raises it for the Agent tool, the fields a test does not name filled in. */
export function spawnInput(fields: Partial<AgentSpawnInput>): AgentSpawnInput {
  return {
    tool_use_id: `toolu_${fields.description ?? 'agent'}`,
    prompt: 'Read the file.',
    description: 'agent',
    subagentType: 'general-purpose',
    provider: { plugin: 'engine', tier: 'core' },
    parentModel: 'claude-opus-5-5',
    background: false,
    fork: false,
    ...fields,
  }
}

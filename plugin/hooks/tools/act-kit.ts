// Test support for the act modules: exact answers to each process call in order, and a file tree under the skill, plan and worktree folders.
import type { On } from 'claude-code'

import type { RexBook, RexJob } from '../../types'
import { emptyBook, newIngest } from '../book/events'
import { openJobIn } from '../book/jobs'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { ROOT, SESSION, START_MS } from '../book/kit'

export const HOME = '/home/person'

export type Scripted = { readonly argv: readonly string[]; readonly answer: ProcessAnswer }

/**
 * Answers each call from the list in order and refuses any other; `rex ingest`
 * gets `last_sequence` of the highest sequence it was sent, so a test with
 * storage on needs no line for it.
 */
export function scripted(list: readonly Scripted[], seen: ProcessCall[] = []): (call: ProcessCall) => ProcessAnswer {
  const left = [...list]
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
    const next = left.shift()
    if (next === undefined) {
      return { reject: `no answer left for ${call.argv.join(' ')}` }
    }
    if (JSON.stringify(next.argv) !== JSON.stringify(call.argv)) {
      return { reject: `expected ${next.argv.join(' ')}, got ${call.argv.join(' ')}` }
    }
    return next.answer
  }
}

export function ok(stdout = ''): ProcessAnswer {
  return { exitCode: 0, stdout, stderr: '' }
}

export function exit(exitCode: number, stderr: string): ProcessAnswer {
  return { exitCode, stdout: '', stderr }
}

/** The git call that names the base of a job of the session root. */
export const BASE_CALL: Scripted = { argv: ['git', '-C', ROOT, 'symbolic-ref', '--short', 'refs/remotes/origin/HEAD'], answer: ok('origin/main\n') }

function base64Of(text: string): string {
  const bytes = new TextEncoder().encode(text)
  let binary = ''
  for (const byte of bytes) {
    binary += String.fromCharCode(byte)
  }
  return btoa(binary)
}

const FOLDERS = /\/(skills|library|plans|worktrees)\//

/** Where a path lands when each symbolic link of `links` (link path to target) on its way is followed. */
function landing(path: string, links: { readonly [link: string]: string }): string {
  let landed = path
  for (let hop = 0; hop < 8; hop += 1) {
    const link = Object.keys(links).find(each => landed === each || landed.startsWith(`${each}/`))
    if (link === undefined) {
      return landed
    }
    landed = `${links[link] ?? link}${landed.slice(link.length)}`
  }
  return landed
}

/**
 * Answers the file calls the act modules make, from a map of absolute path to
 * text; a path below the plugin is named from its `skills/` or `library/`
 * folder on, as `plugin:skills/<name>/<file>`. `links` maps a symbolic link's
 * absolute path to its target, which reads and `realPath` follow. Register it
 * before `world`.
 */
export function files(on: On, tree: { readonly [path: string]: string }, links: { readonly [link: string]: string } = {}): void {
  const lookup = (path: string): string | undefined => {
    const landed = landing(path, links)
    const own = tree[landed]
    if (own !== undefined || landed.startsWith(ROOT) || landed.startsWith(HOME)) {
      return own
    }
    const plugin = /\/((?:skills|library)\/.+)$/.exec(landed)?.[1]
    return plugin === undefined ? undefined : tree[`plugin:${plugin}`]
  }
  on('fs.exists', { path: FOLDERS }, (_$, e) => ({ value: lookup(e.path) !== undefined }))
  on('fs.read', { path: FOLDERS }, (_$, e) => {
    const text = lookup(e.path)
    if (text === undefined) {
      return { deny: `ENOENT: no file at ${e.path}` }
    }
    return { value: e.as === 'bytes' ? { base64: base64Of(text) } : text }
  })
  on('fs.stat', (_$, e) => ({ value: { kind: 'dir', size: 0, mtimeMs: 0, isLink: links[e.path] !== undefined, realPath: landing(e.path, links) } }))
  on('env.get', (_$, e) => ({ value: e.name === 'HOME' ? HOME : undefined }))
}

/** Stands for the engine beneath the plugin for a prompt: the prompt as it arrived. */
export function promptsPass(on: On): void {
  on('prompt.submit', (_$, e) => ({ text: e.text }))
}

export const JOB = '0gaq820001'

export type SeedOptions = {
  readonly storage?: 'on' | 'off'
  readonly builder?: string
  readonly prefs?: RexJob['prefs']
  readonly limitNano?: number
}

/** A book after a session start and the open of job 0gaq820001, the shared fixtures' job, by the main loop unless a builder is named. */
export function seededBook(options: SeedOptions = {}): RexBook {
  const storage = options.storage ?? 'off'
  const started: RexBook = { ...emptyBook(), session: SESSION, storage, versions: { mod: '0.8.0', binary: null, claudeCode: '2.1.323' }, ingest: storage === 'on' ? { ...newIngest('aaaaaaaaaaaaaaaa', '1'.repeat(64)), acked: 0 } : null }
  const input = {
    builder: options.builder ?? 'main',
    tree: null,
    origin: 'composer',
    request: 'Fix the parser',
    repo: ROOT,
    base: 'main',
    type: 'fix',
    size: 'small',
    consequence: 'low',
    langs: ['rust'],
    prefs: options.prefs ?? { duration: null, budget: null, autonomy: null },
    limit: { nano: options.limitNano ?? 10_000_000_000, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  return openJobIn(started, input, JOB, START_MS).book
}

/** The value of one `key=` line of a tool's text result. */
export function lineOf(result: unknown, key: string): string | undefined {
  const text = typeof result === 'object' && result !== null ? Reflect.get(result, 'result') : undefined
  return typeof text === 'string' ? text.split('\n').find(line => line.startsWith(`${key}=`))?.slice(key.length + 1) : undefined
}

export function resultText(result: unknown): string {
  const text = typeof result === 'object' && result !== null ? Reflect.get(result, 'result') : undefined
  return typeof text === 'string' ? text : ''
}

/** The parts of a shared call file a test answers from. */
export function callFile(value: unknown): { readonly argv: readonly string[]; readonly stdout: string } {
  const argv: unknown = typeof value === 'object' && value !== null ? Reflect.get(value, 'argv') : undefined
  const expected: unknown = typeof value === 'object' && value !== null ? Reflect.get(value, 'expect') : undefined
  const field = (key: string): unknown => (typeof expected === 'object' && expected !== null ? Reflect.get(expected, key) : undefined)
  const stdout = field('stdout')
  const json = field('stdout_json') ?? field('stdout_json_contains')
  return {
    argv: Array.isArray(argv) ? argv.map(String) : [],
    stdout: typeof stdout === 'string' ? stdout : `${JSON.stringify(json)}\n`,
  }
}

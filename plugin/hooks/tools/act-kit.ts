// Test support for the act modules: exact answers to each process call in order, the git calls of a job's open and clean-up, and a file tree under the skill, plan and worktree folders.
import type { FsEntry, On } from 'claude-code'

import type { RexBook, RexJob, RexPlan } from '../../types'
import { emptyBook, newIngest } from '../book/events'
import { openJobIn, planJobIn } from '../book/jobs'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { ROOT, SESSION, START_MS } from '../book/kit'

export const HOME = '/home/person'

export type Scripted = { readonly argv: readonly string[]; readonly answer: ProcessAnswer }

/** A word of a scripted argv that any word matches: the path of a job whose id the test cannot know ahead. */
export const ANY_WORD = '<any>'

function isSameArgv(script: readonly string[], argv: readonly string[]): boolean {
  return script.length === argv.length && script.every((word, index) => word === ANY_WORD || word === argv[index])
}

/**
 * Answers each call from the list in order and refuses any other; `rex ingest`
 * gets `last_sequence` of the highest sequence it was sent, so a test with
 * storage on needs no line for it. A word `ANY_WORD` of the list matches any word.
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
    if (!isSameArgv(next.argv, call.argv)) {
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

function gitCall(args: readonly string[], answer: ProcessAnswer = ok()): Scripted {
  return { argv: ['git', '-C', ROOT, ...args], answer }
}

/** The git calls that read the places of the session root in mode `origin`: `main` checked out, and `origin/HEAD` naming `origin/main`. */
export const ORIGIN_PLACES: readonly Scripted[] = [
  gitCall(['symbolic-ref', '--quiet', '--short', 'HEAD'], ok('main\n')),
  gitCall(['remote'], ok('origin\n')),
  gitCall(['symbolic-ref', '--short', 'refs/remotes/origin/HEAD'], ok('origin/main\n')),
  gitCall(['for-each-ref', '--format=%(refname)', 'refs/remotes/origin'], ok('refs/remotes/origin/HEAD\nrefs/remotes/origin/main\n')),
]

/** The git calls that read the places of the session root in mode `none`: `main` checked out, and no remote. */
export const LOCAL_PLACES: readonly Scripted[] = [gitCall(['symbolic-ref', '--quiet', '--short', 'HEAD'], ok('main\n')), gitCall(['remote'], ok(''))]

/** The check for a commit that a repo with one answers. */
export const HAS_COMMIT: Scripted = gitCall(['rev-parse', '--verify', '--quiet', 'HEAD'], ok('c0\n'))

/** The tree `git write-tree` answers for an empty index, as real git does: the index the person had staged, kept for a failed first commit. */
export const EMPTY_TREE = '4b825dc642cb6eb9a060e54bf8d69288fbee4904'

/** The calls of a repo with no commit: the check, then the first commit the mod makes. */
export const FIRST_COMMIT: readonly Scripted[] = [
  gitCall(['rev-parse', '--verify', '--quiet', 'HEAD'], { exitCode: 1, stdout: '', stderr: '' }),
  gitCall(['write-tree'], ok(`${EMPTY_TREE}\n`)),
  gitCall(['add', '--all'], ok()),
  gitCall(['commit', '--allow-empty', '--message', 'chore: first commit, the base of rex jobs'], ok('[master (root-commit) c0] chore: first commit\n')),
]

/** The count of uncommitted paths in the main checkout, `uncommitted` lines long. */
export function statusCall(uncommitted: readonly string[] = []): Scripted {
  return gitCall(['status', '--porcelain'], ok(uncommitted.map(line => `${line}\n`).join('')))
}

/** The full hash the base answers in a test: the commit each job worktree starts from. */
export const START_HASH = '5a1f0c9e2b7d4a6f8c3e1b0d9a7f5c3e1b2d4f6a'

/** The calls that read the commit a job starts from: a fetch of origin and `origin/<base>` in mode `origin`, the local base in mode `none`. */
export function startCalls(base: string, remote: 'origin' | 'none'): readonly Scripted[] {
  const read = gitCall(['rev-parse', remote === 'origin' ? `origin/${base}` : base], ok(`${START_HASH}\n`))
  return remote === 'origin' ? [gitCall(['fetch', 'origin', base]), read] : [read]
}

/** The rex worktree call that makes the worktree of a new job, whose id, and so its path and branch, the test does not know ahead. */
export const WORKTREE_ADD: Scripted = { argv: ['rex', 'worktree', 'add', ROOT, ANY_WORD, START_HASH, '--branch', ANY_WORD], answer: ok(`worktree=${ROOT}.worktrees/job\nsettings=added\n`) }

/** The calls of one rex_job_open in a repo with a commit and origin. */
export const OPEN_CALLS: readonly Scripted[] = [HAS_COMMIT, ...ORIGIN_PLACES, ...startCalls('main', 'origin'), WORKTREE_ADD, statusCall()]

/** `git worktree list --porcelain` of a repo whose main checkout holds `main`, and the linked worktrees `paths` on their branches. */
export function worktreeList(paths: readonly { readonly path: string; readonly branch: string }[] = []): ProcessAnswer {
  const blocks = [{ path: ROOT, branch: 'main' }, ...paths].map(each => `worktree ${each.path}\nHEAD ${START_HASH}\nbranch refs/heads/${each.branch}\n`)
  return ok(blocks.join('\n'))
}

/** The calls of the clean-up of job `job` that finds nothing to remove: no worktree and no branch of it, here or on origin. */
export function cleanCalls(remote: 'origin' | 'none', job: string = JOB): readonly Scripted[] {
  const branch = `rex/${job}`
  return [
    ...(remote === 'origin' ? ORIGIN_PLACES : LOCAL_PLACES),
    ...(remote === 'origin' ? [gitCall(['fetch', '--prune', 'origin'])] : []),
    gitCall(['worktree', 'list', '--porcelain'], worktreeList()),
    gitCall(['for-each-ref', '--format=%(refname:short)', `refs/heads/${branch}`, `refs/heads/${branch}--*`]),
    ...(remote === 'origin' ? [gitCall(['for-each-ref', '--format=%(refname:lstrip=3)', `refs/remotes/origin/${branch}`, `refs/remotes/origin/${branch}--*`])] : []),
  ]
}

/** The calls of a rex_job_open that answers the caller's job it already has. */
export const EXISTING_CALLS: readonly Scripted[] = [...ORIGIN_PLACES, statusCall()]

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
  on('fs.list', { path: FOLDERS }, (_$, e) => ({ value: listing(tree, e.path) }))
  on('env.get', (_$, e) => ({ value: e.name === 'HOME' ? HOME : undefined }))
}

/** The entries of one folder of `tree`: a name with more path after it is a folder, else a file. */
function listing(tree: { readonly [path: string]: string }, dir: string): FsEntry[] {
  const plugin = /\/((?:skills|library)\/.+)$/.exec(dir)?.[1]
  const prefix = dir.startsWith(ROOT) || dir.startsWith(HOME) || plugin === undefined ? `${dir}/` : `plugin:${plugin}/`
  const entries = new Map<string, FsEntry['kind']>()
  for (const path of Object.keys(tree).filter(each => each.startsWith(prefix))) {
    const [name = '', ...rest] = path.slice(prefix.length).split('/')
    entries.set(name, rest.length === 0 ? 'file' : 'dir')
  }
  return [...entries].map(([name, kind]) => ({ name, kind, size: 0, mtimeMs: 0, isLink: false }))
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

/** The plan a builder records before it merges: one task T1 of role engineer on model-high-effort-high. */
const ONE_TASK_PLAN: RexPlan = {
  orchestrator: 'single-owner-edit',
  breakdown: 'T1 engineer',
  tasks: [{ id: 'T1', role: 'engineer', skills: [], openWhen: [] }],
  roles: [{ role: 'engineer', ratings: { intelligence: 'high', reliability: 'high', consequence: 'high', verification: 'low' }, gridAgent: 'model-high-effort-high', conflict: null, omsVersion: 2 }],
  manifest: [],
}

/** The same book with each job that has no plan yet planned, as rex_merge needs. */
export function plannedAll(book: RexBook): RexBook {
  return book.jobs.filter(job => job.status === 'created').reduce((planned, job) => planJobIn(planned, job.id, ONE_TASK_PLAN, START_MS), book)
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

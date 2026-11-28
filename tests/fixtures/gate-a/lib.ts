// Shared parts of the gate A scripts: the fixed paths, flag reading, JSON reading with checks, and calls to the job's own rex binary.
// Every script imports it, so a path or a parse rule changes in one place.
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { basename, join, resolve } from 'node:path'

export const JOB_DIR = resolve(import.meta.dirname, '../../..')
export const HERE = import.meta.dirname
export const REX = join(JOB_DIR, 'target/debug/rex')
export const LIBRARY = join(JOB_DIR, 'plugin/library')
export const VOCABULARY = join(JOB_DIR, 'plugin/vocab/terms.json')
/** The commit that the measured binary and library were built from. */
export const BASE_COMMIT = '75566e35d9aa83c2ae166ae1906a83a3b33b8dae'
/** The filler pool lives outside the repo because it is outside text that nobody reviewed. */
export const DEFAULT_POOL = '/Users/shaun/Code/DEVELOPMENT/gate-a-pool/terminalskills/skills'
export const DEFAULT_RAW = '/Users/shaun/Code/DEVELOPMENT/gate-a-pool/raw'
export const TASKS_FILE = join(HERE, 'tasks.json')
export const HEADER_QUOTES_FILE = join(HERE, 'header-quotes.json')
/** Large enough for one search answer or the whole tree listing. */
const MAX_OUTPUT_BYTES = 256 * 1024 * 1024

export class ScriptError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'ScriptError'
  }
}

export type Json = null | boolean | number | string | readonly Json[] | { readonly [key: string]: Json }
export type JsonObject = { readonly [key: string]: Json }

export type Flags = { readonly values: ReadonlyMap<string, readonly string[]>; readonly positional: readonly string[] }

/** Reads `--name value` pairs (a flag may repeat) and the words that are not flags. */
export function readFlags(argv: readonly string[], known: readonly string[]): Flags {
  const values = new Map<string, string[]>()
  const positional: string[] = []
  for (let at = 0; at < argv.length; at += 1) {
    const word = argv[at] ?? ''
    if (!word.startsWith('--')) {
      positional.push(word)
      continue
    }
    const name = word.slice(2)
    if (!known.includes(name)) throw new ScriptError(`unknown flag --${name}; known flags: ${known.map((k) => `--${k}`).join(' ')}`)
    const value = argv[at + 1]
    if (value === undefined) throw new ScriptError(`flag --${name} needs a value`)
    values.set(name, [...(values.get(name) ?? []), value])
    at += 1
  }
  return { values, positional }
}

export function flag(flags: Flags, name: string, fallback: string): string {
  const given = flags.values.get(name)
  return given === undefined ? fallback : (given[given.length - 1] ?? fallback)
}

export function requiredFlag(flags: Flags, name: string): string {
  const given = flags.values.get(name)
  const value = given?.[given.length - 1]
  if (value === undefined) throw new ScriptError(`flag --${name} is required`)
  return value
}

export function sha256(text: string): string {
  return createHash('sha256').update(text).digest('hex')
}

export function isObject(value: Json | undefined): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function isArray(value: Json | undefined): value is readonly Json[] {
  return Array.isArray(value)
}

export function readJson(path: string): Json {
  if (!existsSync(path)) throw new ScriptError(`file ${path} does not exist`)
  try {
    return JSON.parse(readFileSync(path, 'utf8')) as Json
  } catch (cause: unknown) {
    throw new ScriptError(`file ${path} is not JSON: ${cause instanceof Error ? cause.message : 'unknown parse error'}`)
  }
}

export function writeJson(path: string, value: Json): void {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`)
}

export function ensureFolder(path: string): void {
  mkdirSync(path, { recursive: true })
}

/** The `.json` files of a folder, sorted by name, so every run reads them in the same order. A name that starts with `_` describes the run and is skipped. */
export function jsonFiles(folder: string): string[] {
  if (!existsSync(folder)) throw new ScriptError(`folder ${folder} does not exist`)
  if (folder.endsWith('.json')) return [folder]
  return readdirSync(folder)
    .filter((name) => name.endsWith('.json') && !name.startsWith('_'))
    .sort()
    .map((name) => join(folder, name))
}

export function text(value: Json | undefined): string | undefined {
  return typeof value === 'string' ? value : undefined
}

export function textList(value: Json | undefined): string[] {
  return isArray(value) ? value.filter((entry): entry is string => typeof entry === 'string') : []
}

export type Run = { readonly status: number; readonly stdout: string; readonly stderr: string }

export function run(command: string, args: readonly string[], input?: string): Run {
  const options = { encoding: 'utf8' as const, maxBuffer: MAX_OUTPUT_BYTES, cwd: JOB_DIR }
  const result = spawnSync(command, args, input === undefined ? options : { ...options, input })
  if (result.error !== undefined) throw new ScriptError(`failed to run ${command} ${args.join(' ')}: ${result.error.message}`)
  return { status: result.status ?? -1, stdout: result.stdout, stderr: result.stderr }
}

/** One call of `rex skills search` over the given folders, in priority order. Returns the answer line. */
export function search(libraries: readonly string[], request: Json): string {
  const args = ['skills', 'search', ...libraries.flatMap((folder) => ['--library', folder])]
  const result = run(REX, args, JSON.stringify(request))
  if (result.status !== 0) throw new ScriptError(`rex ${args.join(' ')} exited ${result.status}: ${result.stderr.trim()}`)
  return result.stdout.trim()
}

/** The name the binary gives a library: its folder's base name. */
export function libraryName(folder: string): string {
  return basename(folder)
}

export type Item = {
  readonly rank: number
  readonly skill: string
  readonly library: string
  readonly tier: string
  readonly slot: string
  readonly raw: JsonObject
}

export type Result = {
  readonly request: string
  readonly items: readonly Item[]
  readonly more: number
  readonly cursor: string | null
  readonly raw: JsonObject
}

/** Reads one answer line of `rex skills search` and checks the fields the scripts rely on. */
export function readAnswer(line: string): { readonly indexVersion: string; readonly results: readonly Result[] } {
  const answer = JSON.parse(line) as Json
  if (!isObject(answer) || !isArray(answer['results'])) throw new ScriptError('the answer has no `results` list')
  const results = answer['results'].map((entry, at): Result => {
    if (!isObject(entry) || !isArray(entry['items'])) throw new ScriptError(`result ${at + 1} has no \`items\` list`)
    const items = entry['items'].map((item, rank): Item => {
      if (!isObject(item)) throw new ScriptError(`item ${rank + 1} of result ${at + 1} is not an object`)
      return {
        rank: rank + 1,
        skill: text(item['skill']) ?? '',
        library: text(item['library']) ?? '',
        tier: text(item['tier']) ?? '',
        slot: text(item['slot']) ?? '',
        raw: item,
      }
    })
    const more = entry['more']
    const cursor = entry['cursor']
    return {
      request: text(entry['request']) ?? '',
      items,
      more: typeof more === 'number' ? more : 0,
      cursor: typeof cursor === 'string' ? cursor : null,
      raw: entry,
    }
  })
  return { indexVersion: text(answer['index_version']) ?? '', results }
}

/** Every page of one form's answer, by following its cursor at the given limit. */
export function allPages(libraries: readonly string[], form: Json, limit: number): Item[] {
  const items: Item[] = []
  let cursor: string | null = null
  for (;;) {
    const line = search(libraries, { requests: [form], limit, cursor })
    const result = readAnswer(line).results[0]
    if (result === undefined) throw new ScriptError('the answer holds no result')
    items.push(...result.items.map((item) => ({ ...item, rank: items.length + item.rank })))
    if (result.cursor === null) return items
    cursor = result.cursor
  }
}

/** Prints one numbered line of a script's output, so every result can be quoted by number. */
export function say(number: string, line: string): void {
  process.stdout.write(`[${number}] ${line}\n`)
}

/** Runs a script's main function and turns a `ScriptError` into a message and exit code 1. */
export function main(body: () => void): void {
  try {
    body()
  } catch (error: unknown) {
    if (!(error instanceof ScriptError)) throw error
    process.stderr.write(`error: ${error.message}\n`)
    process.exitCode = 1
  }
}

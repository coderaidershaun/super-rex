// Lists each tool call of a sub-agent transcript and flags each read outside its input folder, each skill call and each search, to check that a form writer stayed blind.
// Usage: node tests/fixtures/gate-a-2/transcript-check.ts --transcript <jsonl> --input <folder> [--allow <folder>]... [--expect <transcripts.txt> --section <name>]
import { existsSync, readFileSync } from 'node:fs'
import { basename } from 'node:path'

import { flag, type Json, type JsonObject, main, readFlags, requiredFlag, say, ScriptError, text } from './lib.ts'
import { type ToolCall, toolCalls } from './transcript-lib.ts'

/** The input fields a listing shows: the path or command of a call, never written content. */
const SHOWN_FIELDS = ['file_path', 'path', 'notebook_path', 'command', 'pattern', 'glob', 'skill', 'name', 'file', 'query', 'url']
/** The first measurement cut each listed call at this many characters. */
const CUT = 300
const READ_TOOLS = ['Read', 'Glob', 'Grep', 'LS', 'NotebookRead']
const WRITE_TOOLS = ['Write', 'Edit', 'MultiEdit', 'NotebookEdit']
/** Paths that every shell command may name, such as `/dev/null`. */
const ALWAYS_ALLOWED = ['/dev/']

/** One value as Python's `json.dumps` writes it, which is how the first measurement wrote its listing: `, ` and `: ` between parts, and every character outside printable ASCII escaped. */
function pythonJson(value: Json): string {
  if (typeof value === 'string') return JSON.stringify(value).replace(/[^\x20-\x7e]/g, (char) => `\\u${char.charCodeAt(0).toString(16).padStart(4, '0')}`)
  if (Array.isArray(value)) return `[${value.map(pythonJson).join(', ')}]`
  if (typeof value === 'object' && value !== null) return `{${Object.entries(value).map(([key, entry]) => `${pythonJson(key)}: ${pythonJson(entry)}`).join(', ')}}`
  return JSON.stringify(value)
}

/** `<tool> <shown input>`, cut at 300 characters with trailing space removed, as the committed listing of the first measurement holds it. */
export function listed(call: ToolCall): string {
  const shownInput: Record<string, Json> = {}
  for (const [key, value] of Object.entries(call.input)) if (SHOWN_FIELDS.includes(key)) shownInput[key] = value
  return `${call.name} ${pythonJson(shownInput)}`.slice(0, CUT).trimEnd()
}

function isUnder(path: string, folders: readonly string[]): boolean {
  return folders.some((folder) => path === folder.replace(/\/$/, '') || path.startsWith(folder.endsWith('/') ? folder : `${folder}/`))
}

/** The absolute paths that a shell command names. */
function commandPaths(command: string): string[] {
  return [...command.matchAll(/(?:^|[\s'"=(<>:])(\/[^\s'";|&<>()]+)/g)].map((match) => match[1] ?? '')
}

function flagsOf(call: ToolCall, input: string, allowed: readonly string[]): string[] {
  const found: string[] = []
  const pathOf = (record: JsonObject): string | undefined => text(record['file_path']) ?? text(record['path']) ?? text(record['notebook_path'])
  const path = pathOf(call.input)
  if (READ_TOOLS.includes(call.name)) {
    if (path === undefined) found.push('read with no path, so from the working folder')
    else if (!isUnder(path, [input])) found.push(`read outside the input folder: ${path}`)
  }
  if (WRITE_TOOLS.includes(call.name) && path !== undefined && !isUnder(path, [input, ...allowed])) found.push(`write outside the input and allowed folders: ${path}`)
  if (call.name === 'Skill' || call.name.includes('rex_skill')) found.push(`skill call: ${call.name} ${pythonJson(call.input)}`)
  const command = text(call.input['command'])
  if (/search/i.test(call.name)) found.push(`search tool: ${call.name}`)
  if (command !== undefined) {
    if (/skills\s+search|rex_search/.test(command)) found.push('shell call holds `skills search` (check by hand)')
    if (/(?:^|[\s;&|(`])(?:\S*\/)?(?:rex|claude)(?=\s|$)/.test(command)) found.push('shell call names rex or claude as a command word (check by hand)')
    const outside = commandPaths(command).filter((named) => !isUnder(named, [input, ...allowed, ...ALWAYS_ALLOWED]))
    if (outside.length > 0) found.push(`shell call names paths outside the input and allowed folders (check by hand): ${[...new Set(outside)].join(' ')}`)
  }
  return found
}

/** The lines of one section of a committed listing: after `== <section>`, up to the next blank line. */
function sectionLines(file: string, section: string): string[] {
  if (!existsSync(file)) throw new ScriptError(`file ${file} does not exist`)
  const lines = readFileSync(file, 'utf8').split('\n')
  const start = lines.indexOf(`== ${section}`)
  if (start < 0) throw new ScriptError(`${file} has no section == ${section}`)
  const rest = lines.slice(start + 1)
  const end = rest.findIndex((line) => line.trim() === '' || line.startsWith('== '))
  return (end < 0 ? rest : rest.slice(0, end)).map((line) => line.trimEnd())
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['transcript', 'input', 'allow', 'expect', 'section'])
  const transcript = requiredFlag(flags, 'transcript')
  const input = requiredFlag(flags, 'input')
  const allowed = flags.values.get('allow') ?? []
  const calls = toolCalls(transcript)
  say('t0', `${basename(transcript)}: input folder ${input}; allowed for writes and shell paths: ${allowed.join(', ') || 'none'}; each call cut at ${CUT} characters`)
  calls.forEach((call) => say('t1', listed(call)))
  const distinct = new Map<string, ToolCall>()
  for (const call of calls) if (!distinct.has(call.id)) distinct.set(call.id, call)
  say('t2', `${calls.length} tool call blocks listed, ${distinct.size} distinct calls by id; the transcript repeats a call in the record of a later request`)
  let flagged = 0
  for (const call of distinct.values()) {
    const found = flagsOf(call, input, allowed)
    if (found.length > 0) flagged += 1
    found.forEach((entry) => say('t3', `${call.name} (transcript line ${call.line}): ${entry}`))
  }
  const names = [...distinct.values()].map((call) => call.name)
  const reads = [...distinct.values()].filter((call) => READ_TOOLS.includes(call.name)).length
  const skills = names.filter((name) => name === 'Skill' || name.includes('rex_skill')).length
  say('t4', `${distinct.size} distinct calls, ${flagged} flagged; ${reads} read calls; ${skills} skill calls; tools used: ${[...new Set(names)].sort().join(', ')}`)
  const expect = flag(flags, 'expect', '')
  if (expect === '') return
  const section = requiredFlag(flags, 'section')
  const expected = sectionLines(expect, section)
  const actual = calls.map(listed)
  const same = expected.length === actual.length && expected.every((line, at) => line === actual[at])
  expected.forEach((line, at) => {
    if (line !== actual[at]) say('t5', `line ${at + 1} differs: committed ${JSON.stringify(line)}, listed ${JSON.stringify(actual[at] ?? '')}`)
  })
  say('t5', `section == ${section} of ${basename(expect)}: ${expected.length} lines; this listing: ${actual.length} lines; ${same ? 'every line is the same' : 'the listings DIFFER'} (trailing space removed on both sides)`)
  if (!same) process.exitCode = 1
})

// Runs the WP0 proof that a mod reaches its own plugin's MCP server with `$.mcp.call`: one nested headless Claude Code session per case, each in a new temp folder.
// It prints, for each case, the marker and nonce the session returned and the stub's log line for that nonce. Usage: node tests/fixtures/wp0-mcp/run.ts a b c
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

type CaseName = 'a' | 'b' | 'c'

type Session = { readonly dir: string; readonly nonce: string; readonly log: string; readonly builderDir: string }

type Evidence = {
  readonly exit: string
  readonly stubStatus: string
  readonly stubToolsOffered: readonly string[]
  readonly denials: readonly string[]
  readonly toolUses: readonly string[]
  readonly markers: readonly string[]
  readonly started: string | null
  readonly logLines: readonly string[]
  readonly finalText: string
  readonly cost: string
}

const PLUGIN_DIR = resolve(import.meta.dirname, 'probe-plugin')
/** The repo's own plugin, loaded only for case c, so its grid agent types exist; its mod stays inert in a folder with no `.rex/config.json`. */
const SUPER_REX_DIR = resolve(import.meta.dirname, '../../../plugin')
const PLUGIN = 'wp0-mcp-probe'
const PROBE = `mcp__${PLUGIN}__mcp_probe`
const SPAWN = `mcp__${PLUGIN}__spawn_probe`
const STUB_SERVER = `plugin:${PLUGIN}:stub`
const STUB_TOOL_PREFIX = `mcp__plugin_${PLUGIN}_stub`
const MARKER = 'WP0-MCP-MARKER'
const CLAUDE = process.env['CLAUDE_BIN'] ?? 'claude'
const RUN_TIMEOUT_MS = 600_000
const MAX_OUTPUT_BYTES = 64 * 1024 * 1024

/** Every stub tool is off for the model, so the nonce can reach the stub only through the mod. */
const DISALLOWED = [STUB_TOOL_PREFIX, `${STUB_TOOL_PREFIX}__*`]

const ALLOWED: Readonly<Record<CaseName, readonly string[]>> = { a: [PROBE], b: [PROBE, 'Agent'], c: [PROBE, SPAWN] }

const PLUGIN_DIRS: Readonly<Record<CaseName, readonly string[]>> = { a: [PLUGIN_DIR], b: [PLUGIN_DIR], c: [PLUGIN_DIR, SUPER_REX_DIR] }

function prompt(name: CaseName, session: Session): string {
  const call = `Call the tool ${PROBE} once with {"nonce": "${session.nonce}"}, then reply with its output exactly as it came back, and nothing else.`
  switch (name) {
    case 'a':
      return `${call} Call no other tool.`
    case 'b':
      return `Use the Agent tool once, with subagent_type "general-purpose" and model "haiku", and give the sub-agent this task: '${call}' Do not call ${PROBE} yourself. When the sub-agent answers, reply with its answer exactly, and nothing else.`
    case 'c':
      return `Call the tool ${SPAWN} once with {"nonce": "${session.nonce}", "cwd": "${session.builderDir}"}. It starts a background agent. Wait for that agent to finish, then reply with the agent's final answer exactly, and nothing else. Call no other tool.`
    default:
      return assertNever(name)
  }
}

function assertNever(value: never): never {
  throw new Error(`unhandled case: ${JSON.stringify(value)}`)
}

function newSession(name: CaseName): Session {
  const dir = mkdtempSync(join(tmpdir(), `wp0-mcp-${name}-`))
  const builderDir = mkdtempSync(join(tmpdir(), 'wp0-mcp-builder-'))
  return { dir, nonce: randomUUID(), log: join(dir, 'stub-calls.log'), builderDir }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function parsedLines(stdout: string): readonly Record<string, unknown>[] {
  const records: Record<string, unknown>[] = []
  for (const line of stdout.split('\n')) {
    if (!line.startsWith('{')) {
      continue
    }
    const value: unknown = JSON.parse(line)
    if (isRecord(value)) {
      records.push(value)
    }
  }
  return records
}

function stubStatus(init: Record<string, unknown> | undefined): string {
  const servers = init?.['mcp_servers']
  if (!Array.isArray(servers)) {
    return 'no init message'
  }
  const stub = servers.find((server: unknown) => isRecord(server) && server['name'] === STUB_SERVER)
  return isRecord(stub) ? `${STUB_SERVER} ${String(stub['status'])}` : `${STUB_SERVER} not listed`
}

function offeredStubTools(init: Record<string, unknown> | undefined): readonly string[] {
  const tools = init?.['tools']
  return Array.isArray(tools) ? tools.filter((tool: unknown): tool is string => typeof tool === 'string' && tool.startsWith(STUB_TOOL_PREFIX)) : []
}

function denialsOf(result: Record<string, unknown> | undefined): readonly string[] {
  const denials = result?.['permission_denials']
  return Array.isArray(denials) ? denials.map((denial: unknown) => (isRecord(denial) ? String(denial['tool_name']) : String(denial))) : []
}

/** Each marker span that names this run's nonce, wherever it shows in the transcript. */
function markersIn(stdout: string, nonce: string): readonly string[] {
  const unescaped = stdout.replaceAll('\\"', '"')
  const spans = unescaped.match(new RegExp(`${MARKER} [^\\n]*?${MARKER}-END`, 'g')) ?? []
  return [...new Set(spans.filter(span => span.includes(nonce)))]
}

function logLinesFor(log: string, nonce: string): readonly string[] {
  if (!existsSync(log)) {
    return [`no log at ${log}`]
  }
  return readFileSync(log, 'utf8').split('\n').filter(line => line.includes(nonce) || line.includes('"event":"start"') || line.includes('"event":"initialize"'))
}

/** Every tool call in the transcript, sub-agents' included, with the agent loop it came from. */
function toolUsesIn(records: readonly Record<string, unknown>[]): readonly string[] {
  const uses: string[] = []
  for (const record of records) {
    const message = record['message']
    const content = isRecord(message) ? message['content'] : undefined
    if (record['type'] !== 'assistant' || !Array.isArray(content)) {
      continue
    }
    const parent = typeof record['parent_tool_use_id'] === 'string' ? `in ${record['parent_tool_use_id']}` : 'in main'
    for (const block of content) {
      if (isRecord(block) && block['type'] === 'tool_use') {
        uses.push(`${String(block['name'])} ${parent}`)
      }
    }
  }
  return uses
}

function evidence(stdout: string, exit: string, session: Session): Evidence {
  const records = parsedLines(stdout)
  const init = records.find(record => record['type'] === 'system' && record['subtype'] === 'init')
  const result = records.findLast(record => record['type'] === 'result')
  const started = /started agent=(\S+)/.exec(stdout)?.[1] ?? null
  return {
    exit,
    stubStatus: stubStatus(init),
    stubToolsOffered: offeredStubTools(init),
    denials: denialsOf(result),
    toolUses: toolUsesIn(records),
    markers: markersIn(stdout, session.nonce),
    started,
    logLines: logLinesFor(session.log, session.nonce),
    finalText: typeof result?.['result'] === 'string' ? result['result'] : 'none',
    cost: String(result?.['total_cost_usd'] ?? 'unknown'),
  }
}

function callerOf(logLines: readonly string[], nonce: string): string | null {
  const call = logLines.find(line => line.includes('"event":"tools/call"') && line.includes(nonce))
  return call === undefined ? null : (/"caller":"([^"]*)"/.exec(call)?.[1] ?? null)
}

/** PASSED needs the marker with the nonce from the session and a stub log line for the nonce from the right caller. */
function verdict(name: CaseName, found: Evidence, nonce: string): string {
  const caller = callerOf(found.logLines, nonce)
  if (found.markers.length === 0 || caller === null) {
    return `FAILED: marker with nonce ${found.markers.length > 0 ? 'seen' : 'missing'}, stub log line ${caller === null ? 'missing' : 'seen'}`
  }
  switch (name) {
    case 'a':
      return caller === 'main' ? 'PASSED' : `FAILED: caller ${caller}, expected main`
    case 'b':
      return caller !== 'main' ? `PASSED (sub-agent ${caller})` : 'FAILED: caller main, expected a sub-agent'
    case 'c':
      return caller === found.started ? `PASSED (spawned agent ${caller})` : `FAILED: caller ${caller}, spawned agent ${found.started ?? 'none'}`
    default:
      return assertNever(name)
  }
}

function runCase(name: CaseName): void {
  const session = newSession(name)
  const args = ['-p', prompt(name, session), '--model', 'haiku', ...PLUGIN_DIRS[name].flatMap(dir => ['--plugin-dir', dir]), '--output-format', 'stream-json', '--verbose', '--allowedTools', ...ALLOWED[name], '--disallowedTools', ...DISALLOWED]
  const run = spawnSync(CLAUDE, args, { cwd: session.dir, env: { ...process.env, WP0_MCP_LOG: session.log }, encoding: 'utf8', timeout: RUN_TIMEOUT_MS, maxBuffer: MAX_OUTPUT_BYTES })
  writeFileSync(join(session.dir, 'transcript.jsonl'), run.stdout)
  writeFileSync(join(session.dir, 'stderr.txt'), run.stderr)
  const exit = run.error === undefined ? `status ${String(run.status)} signal ${String(run.signal)}` : `spawn error ${run.error.message}`
  const found = evidence(run.stdout, exit, session)
  const lines = [
    `=== case ${name} ===`,
    `command: (cd ${session.dir} && WP0_MCP_LOG=${session.log} ${CLAUDE} ${args.map(arg => JSON.stringify(arg)).join(' ')})`,
    `nonce: ${session.nonce}`,
    `exit: ${found.exit}; cost usd ${found.cost}`,
    `stub server at init: ${found.stubStatus}`,
    `stub tools offered to the model: ${found.stubToolsOffered.length === 0 ? 'none' : found.stubToolsOffered.join(', ')}`,
    `permission denials: ${found.denials.length === 0 ? 'none' : found.denials.join(', ')}`,
    `tool calls in the transcript: ${found.toolUses.length === 0 ? 'none' : found.toolUses.join(', ')}`,
    ...(name === 'c' ? [`builder folder: ${session.builderDir}`, `spawned agent: ${found.started ?? 'none'}`] : []),
    ...found.markers.map(marker => `marker: ${marker}`),
    ...found.logLines.map(line => `log: ${line}`),
    `final answer: ${found.finalText}`,
    `transcript: ${join(session.dir, 'transcript.jsonl')}`,
    `verdict: ${verdict(name, found, session.nonce)}`,
  ]
  process.stdout.write(`${lines.join('\n')}\n\n`)
}

function isCaseName(value: string): value is CaseName {
  return value === 'a' || value === 'b' || value === 'c'
}

const asked = process.argv.slice(2)
const unknown = asked.filter(arg => !isCaseName(arg))
if (asked.length === 0 || unknown.length > 0) {
  process.stdout.write(`usage: node run.ts a|b|c ..., got ${asked.length === 0 ? 'no case' : unknown.join(' ')}\n`)
  process.exitCode = 2
} else {
  for (const name of asked.filter(isCaseName)) {
    runCase(name)
  }
}

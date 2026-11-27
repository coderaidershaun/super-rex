// Runs the live proof that Claude Code starts the library server that this repo's plugin declares, and that a mod's `$.mcp.call` of `head` gets its answer.
// It starts one nested headless session with this repo's plugin and a probe plugin, then prints the evidence and one verdict for each line of RESULTS.md. Usage: node tests/fixtures/wp5-live/run.ts [control]
import { spawn, spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { existsSync, mkdtempSync, readFileSync, realpathSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const WORKTREE = resolve(import.meta.dirname, '../../..')
const SUPER_REX_DIR = join(WORKTREE, 'plugin')
const LIBRARY_DIR = join(SUPER_REX_DIR, 'library')
const REX_DIR = join(WORKTREE, 'target', 'debug')
const REX_BIN = join(REX_DIR, 'rex')
const PROBE_DIR = resolve(import.meta.dirname, 'probe-plugin')
const PROBE_TOOL = 'mcp__wp5-live-probe__head_probe'
const LIBRARY_SERVER = 'plugin:super-rex:library'
const LIBRARY_TOOL_PREFIX = 'mcp__plugin_super-rex_library'
const MARKER = 'WP5-LIVE-MARKER'
/** The runner puts the marker in the session's environment too, so the server process that the session starts carries it. */
const MARKER_ENV = 'WP5_LIVE_MARKER'
const CLAUDE = process.env['CLAUDE_BIN'] ?? 'claude'
const MAX_TURNS = '3'
const RUN_TIME_LIMIT_MS = 300_000
const POLL_MS = 150
const SERVER_COMMAND = /^(?:\S*\/)?rex skills serve /
const HEAD_KEYS = ['library', 'protocol', 'skills', 'version', 'vocabulary']
const EXPECTED_SKILLS = 25
const EXPECTED_VOCABULARY = '1'

/** The model may not call the raw library tools, so a marked answer can only have come through the mod. */
const DISALLOWED = [LIBRARY_TOOL_PREFIX, `${LIBRARY_TOOL_PREFIX}__*`]

type Head = { readonly protocol: string; readonly library: string; readonly version: string; readonly skills: number; readonly vocabulary: string | null }

type HeadParse = { readonly ok: true; readonly head: Head; readonly keys: readonly string[] } | { readonly ok: false; readonly why: string }

type ProcessRow = { readonly pid: number; readonly ppid: number; readonly command: string }

type ServerProcess = { readonly pid: number; readonly ppid: number; readonly command: string; readonly binary: string; readonly hasMarker: boolean; readonly isOfSession: boolean }

type Run = { readonly stdout: string; readonly stderr: string; readonly exit: string; readonly claudePid: number | null; readonly servers: readonly ServerProcess[] }

type Session = { readonly dir: string; readonly marker: string; readonly debugLog: string; readonly env: Readonly<Record<string, string | undefined>> }

type ToolUse = { readonly id: string; readonly name: string; readonly parent: string; readonly input: unknown }

type Line = { readonly verdict: string; readonly evidence: readonly string[] }

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function errorText(error: unknown): string {
  return error instanceof Error ? `${error.name}: ${error.message}` : String(error)
}

function parseHead(text: string): HeadParse {
  let value: unknown
  try {
    value = JSON.parse(text)
  } catch (error) {
    return { ok: false, why: `the answer is not JSON (${errorText(error)}): ${text}` }
  }
  if (!isRecord(value)) {
    return { ok: false, why: `the answer is not a JSON object: ${text}` }
  }
  const protocol = value['protocol']
  const library = value['library']
  const version = value['version']
  const skills = value['skills']
  const vocabulary = value['vocabulary']
  if (typeof protocol !== 'string' || typeof library !== 'string' || typeof version !== 'string' || typeof skills !== 'number' || !(typeof vocabulary === 'string' || vocabulary === null)) {
    return { ok: false, why: `a field of the answer has the wrong type: ${text}` }
  }
  return { ok: true, head: { protocol, library, version, skills, vocabulary }, keys: Object.keys(value).sort() }
}

/** The text blocks of a JSON-RPC answer to `tools/call`, joined. */
function toolCallText(message: Record<string, unknown>): string | null {
  const result = message['result']
  const content = isRecord(result) ? result['content'] : undefined
  if (!Array.isArray(content)) {
    return null
  }
  return content.map((block: unknown) => (isRecord(block) && typeof block['text'] === 'string' ? block['text'] : '')).join('')
}

/** The `head` answer of this build for this checkout's library, asked directly, with no Claude Code in between. */
function expectedHead(): HeadParse {
  const messages = [
    { jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'wp5-live-runner', version: '0' } } },
    { jsonrpc: '2.0', method: 'notifications/initialized' },
    { jsonrpc: '2.0', id: 2, method: 'tools/call', params: { name: 'head', arguments: {} } },
  ]
  const input = `${messages.map(message => JSON.stringify(message)).join('\n')}\n`
  const run = spawnSync(REX_BIN, ['skills', 'serve', '--name', 'super-rex', '--library', LIBRARY_DIR], { encoding: 'utf8', timeout: 30_000, input })
  if (run.error !== undefined) {
    return { ok: false, why: `cannot start ${REX_BIN}: ${run.error.message}` }
  }
  for (const line of run.stdout.split('\n')) {
    const message: unknown = line.startsWith('{') ? JSON.parse(line) : null
    if (isRecord(message) && message['id'] === 2) {
      const text = toolCallText(message)
      return text === null ? { ok: false, why: `the direct head answer has no content: ${line}` } : parseHead(text)
    }
  }
  return { ok: false, why: `${REX_BIN} gave no answer with id 2; stdout ${run.stdout}; stderr ${run.stderr}` }
}

function processTable(): readonly ProcessRow[] {
  const run = spawnSync('ps', ['-axww', '-o', 'pid=,ppid=,command='], { encoding: 'utf8', timeout: 5_000 })
  const rows: ProcessRow[] = []
  for (const line of run.stdout.split('\n')) {
    const match = /^\s*(\d+)\s+(\d+)\s+(.*)$/.exec(line)
    if (match?.[1] !== undefined && match[2] !== undefined && match[3] !== undefined) {
      rows.push({ pid: Number(match[1]), ppid: Number(match[2]), command: match[3] })
    }
  }
  return rows
}

/** The file the process runs, as the kernel has it open, so a PATH lookup cannot hide which rex this is. */
function binaryOf(pid: number): string {
  const run = spawnSync('lsof', ['-a', '-p', String(pid), '-d', 'txt', '-Fn'], { encoding: 'utf8', timeout: 5_000 })
  const name = run.stdout.split('\n').find(line => line.startsWith('n'))
  return name === undefined ? `unknown (lsof exit ${String(run.status)}: ${run.stderr.trim()})` : name.slice(1)
}

function environmentHasMarker(pid: number, marker: string): boolean {
  const run = spawnSync('ps', ['-wwE', '-o', 'command=', '-p', String(pid)], { encoding: 'utf8', timeout: 5_000 })
  return run.stdout.includes(`${MARKER_ENV}=${marker}`)
}

function descendsFrom(pid: number, ancestor: number | undefined, rows: readonly ProcessRow[]): boolean {
  let current: number | undefined = pid
  for (let step = 0; step < 16 && current !== undefined && current > 1; step += 1) {
    if (current === ancestor) {
      return true
    }
    const at: number = current
    current = rows.find(row => row.pid === at)?.ppid
  }
  return false
}

/** Runs the nested session and watches the process table meanwhile, because the server lives only as long as the session. */
function runSession(args: readonly string[], session: Session): Promise<Run> {
  return new Promise(settle => {
    const child = spawn(CLAUDE, args, { cwd: session.dir, env: session.env, stdio: ['ignore', 'pipe', 'pipe'] })
    const servers = new Map<number, ServerProcess>()
    let stdout = ''
    let stderr = ''
    let isTimedOut = false
    let isDone = false
    child.stdout.setEncoding('utf8')
    child.stderr.setEncoding('utf8')
    child.stdout.on('data', chunk => {
      stdout += chunk
    })
    child.stderr.on('data', chunk => {
      stderr += chunk
    })
    const poll = (): void => {
      const rows = processTable()
      for (const row of rows) {
        if (SERVER_COMMAND.test(row.command) && !servers.has(row.pid)) {
          servers.set(row.pid, { ...row, binary: binaryOf(row.pid), hasMarker: environmentHasMarker(row.pid, session.marker), isOfSession: descendsFrom(row.pid, child.pid, rows) })
        }
      }
    }
    const timer = setInterval(poll, POLL_MS)
    const limit = setTimeout(() => {
      isTimedOut = true
      child.kill('SIGTERM')
    }, RUN_TIME_LIMIT_MS)
    const finish = (exit: string): void => {
      if (isDone) {
        return
      }
      isDone = true
      clearInterval(timer)
      clearTimeout(limit)
      settle({ stdout, stderr, exit, claudePid: child.pid ?? null, servers: [...servers.values()] })
    }
    child.on('error', error => {
      finish(`spawn error ${error.message}`)
    })
    child.on('close', (code, signal) => {
      finish(`status ${String(code)} signal ${String(signal)}${isTimedOut ? ' after the time limit' : ''}`)
    })
  })
}

function parsedLines(stdout: string): { readonly records: readonly Record<string, unknown>[]; readonly unparsed: number } {
  const records: Record<string, unknown>[] = []
  let unparsed = 0
  for (const line of stdout.split('\n')) {
    if (!line.startsWith('{')) {
      continue
    }
    try {
      const value: unknown = JSON.parse(line)
      if (isRecord(value)) {
        records.push(value)
      }
    } catch {
      unparsed += 1
    }
  }
  return { records, unparsed }
}

function contentOf(record: Record<string, unknown>): readonly unknown[] {
  const message = record['message']
  const content = isRecord(message) ? message['content'] : undefined
  return Array.isArray(content) ? content : []
}

function toolUsesIn(records: readonly Record<string, unknown>[]): readonly ToolUse[] {
  const uses: ToolUse[] = []
  for (const record of records.filter(each => each['type'] === 'assistant')) {
    const parent = typeof record['parent_tool_use_id'] === 'string' ? `in ${record['parent_tool_use_id']}` : 'in main'
    for (const block of contentOf(record)) {
      if (isRecord(block) && block['type'] === 'tool_use') {
        uses.push({ id: String(block['id']), name: String(block['name']), parent, input: block['input'] })
      }
    }
  }
  return uses
}

function resultTextFor(records: readonly Record<string, unknown>[], toolUseId: string): string | null {
  for (const record of records.filter(each => each['type'] === 'user')) {
    for (const block of contentOf(record)) {
      if (!isRecord(block) || block['type'] !== 'tool_result' || block['tool_use_id'] !== toolUseId) {
        continue
      }
      const content = block['content']
      if (typeof content === 'string') {
        return content
      }
      if (Array.isArray(content)) {
        return content.map((part: unknown) => (isRecord(part) && typeof part['text'] === 'string' ? part['text'] : '')).join('')
      }
    }
  }
  return null
}

function listOf(init: Record<string, unknown> | undefined, key: string): readonly Record<string, unknown>[] {
  const value = init?.[key]
  return Array.isArray(value) ? value.filter(isRecord) : []
}

function samePath(left: string, right: string): boolean {
  return existsSync(left) && existsSync(right) && realpathSync(left) === realpathSync(right)
}

function libraryServerLine(init: Record<string, unknown> | undefined): Line {
  if (init === undefined) {
    return { verdict: 'FAILED: the session wrote no init message', evidence: [] }
  }
  const servers = listOf(init, 'mcp_servers')
  const entry = servers.find(server => server['name'] === LIBRARY_SERVER)
  const evidence = [`init mcp_servers entry: ${entry === undefined ? 'none' : JSON.stringify(entry)}`, `init mcp_servers names: ${servers.map(server => String(server['name'])).join(', ')}`]
  if (entry === undefined) {
    return { verdict: `FAILED: ${LIBRARY_SERVER} is not in the init message`, evidence }
  }
  return entry['status'] === 'connected' ? { verdict: 'PASSED', evidence } : { verdict: `FAILED: ${LIBRARY_SERVER} has status ${String(entry['status'])}`, evidence }
}

type HeadCall = { readonly line: Line; readonly live: HeadParse | null }

function headCallLine(records: readonly Record<string, unknown>[], marker: string): HeadCall {
  const uses = toolUsesIn(records)
  const probes = uses.filter(use => use.name === PROBE_TOOL)
  const others = uses.filter(use => use.name !== PROBE_TOOL)
  const evidence = [`tool calls in the transcript: ${uses.length === 0 ? 'none' : uses.map(use => `${use.name} ${use.parent}`).join(', ')}`]
  const span = new RegExp(`${MARKER} marker=(\\S+) caller=(\\S+) server=(\\S+) route=(.*?) isError=(\\S+) answer=\\[(.*)\\] ${MARKER}-END`, 's')
  for (const probe of probes) {
    evidence.push(`probe tool input: ${JSON.stringify(probe.input)}`)
    const text = resultTextFor(records, probe.id)
    evidence.push(`probe tool result: ${text ?? 'none'}`)
    const match = text === null ? null : span.exec(text)
    if (match === null) {
      continue
    }
    const [, seenMarker, , , , isError, answer] = match
    const live = parseHead(answer ?? '')
    const problems: string[] = []
    if (seenMarker !== marker) {
      problems.push(`marker ${String(seenMarker)}, expected ${marker}`)
    }
    if (isError !== 'false') {
      problems.push(`isError ${String(isError)}`)
    }
    if (others.length > 0) {
      problems.push(`other tool calls: ${others.map(use => use.name).join(', ')}`)
    }
    if (!live.ok) {
      problems.push(live.why)
    } else {
      if (live.keys.join(',') !== HEAD_KEYS.join(',')) {
        problems.push(`fields ${live.keys.join(',')}, expected ${HEAD_KEYS.join(',')}`)
      }
      if (live.head.library !== 'super-rex') {
        problems.push(`library ${live.head.library}, expected super-rex`)
      }
      if (live.head.skills !== EXPECTED_SKILLS) {
        problems.push(`skills ${String(live.head.skills)}, expected ${String(EXPECTED_SKILLS)}`)
      }
      if (live.head.vocabulary !== EXPECTED_VOCABULARY) {
        problems.push(`vocabulary ${String(live.head.vocabulary)}, expected ${EXPECTED_VOCABULARY}`)
      }
    }
    return { line: { verdict: problems.length === 0 ? 'PASSED' : `FAILED: ${problems.join('; ')}`, evidence }, live }
  }
  return { line: { verdict: `FAILED: no result of ${PROBE_TOOL} holds a marker span`, evidence }, live: null }
}

function thisRepoLine(init: Record<string, unknown> | undefined, live: HeadParse | null, expected: HeadParse, run: Run): Line {
  const superRex = listOf(init, 'plugins').filter(plugin => plugin['name'] === 'super-rex')
  const ofSession = run.servers.filter(server => server.isOfSession)
  const elsewhere = run.servers.filter(server => !server.isOfSession)
  const libraryArg = (command: string): string => command.split(' --library ')[1]?.split(' ')[0] ?? 'none'
  const isVersionSame = live?.ok === true && expected.ok && live.head.version === expected.head.version
  const isPluginOurs = superRex.length === 1 && samePath(String(superRex[0]?.['path']), SUPER_REX_DIR)
  const isProcessOurs = ofSession.length > 0 && ofSession.every(server => samePath(server.binary, REX_BIN) && samePath(libraryArg(server.command), LIBRARY_DIR) && server.command.includes('--name super-rex') && server.hasMarker)
  const evidence = [
    `version, live: ${live?.ok === true ? live.head.version : 'none'}`,
    `version, direct from ${REX_BIN} for ${LIBRARY_DIR}: ${expected.ok ? expected.head.version : expected.why}`,
    `super-rex plugins at init: ${JSON.stringify(superRex)}`,
    `claude pid: ${String(run.claudePid)}`,
    ...ofSession.map(server => `server of the session: pid ${String(server.pid)} ppid ${String(server.ppid)} binary ${server.binary} marker in its environment ${String(server.hasMarker)} command ${server.command}`),
    `rex skills serve processes seen that the session did not start: ${String(elsewhere.length)}`,
  ]
  const missing = [isVersionSame ? null : 'version differs', isPluginOurs ? null : 'super-rex plugin is not this checkout or not alone', isProcessOurs ? null : 'no server process of the session runs this build on this library with the marker'].filter(problem => problem !== null)
  return { verdict: missing.length === 0 ? 'PASSED' : `FAILED: ${missing.join('; ')}`, evidence }
}

const IN_JOB_LINE: Line = {
  verdict: 'NOT EXERCISED: the nested session has no job, so the guard on the raw tools has nothing to refuse; the in-job case is proven by plugin/hooks/library-guard.test.ts',
  evidence: [],
}

/** The control run leaves this build off PATH, so lines 1 to 3 must fail; it shows that the checks can fail. */
type Mode = 'proof' | 'control'

function newSession(mode: Mode): Session {
  const dir = mkdtempSync(join(tmpdir(), `wp5-live-${mode}-`))
  const marker = randomUUID()
  const path = mode === 'proof' ? `${REX_DIR}:${process.env['PATH'] ?? ''}` : process.env['PATH']
  const env = { ...process.env, PATH: path, [MARKER_ENV]: marker }
  return { dir, marker, debugLog: join(dir, 'debug.log'), env }
}

function debugLinesOf(path: string): readonly string[] {
  if (!existsSync(path)) {
    return [`no debug log at ${path}`]
  }
  return readFileSync(path, 'utf8')
    .split('\n')
    .filter(line => line.includes(LIBRARY_SERVER))
    .slice(0, 12)
    .map(line => (line.length > 400 ? `${line.slice(0, 400)}…` : line))
}

async function main(mode: Mode): Promise<void> {
  const session = newSession(mode)
  const expected = expectedHead()
  const resolved = spawnSync('/bin/sh', ['-c', 'command -v rex'], { encoding: 'utf8', timeout: 5_000, env: session.env }).stdout.trim()
  const prompt = `Call the tool ${PROBE_TOOL} once with {"marker": "${session.marker}"}, then reply with its output exactly as it came back, and nothing else. Call no other tool.`
  const args = ['-p', prompt, '--model', 'haiku', '--max-turns', MAX_TURNS, '--plugin-dir', PROBE_DIR, '--plugin-dir', SUPER_REX_DIR, '--output-format', 'stream-json', '--verbose', '--debug-file', session.debugLog, '--allowedTools', PROBE_TOOL, '--disallowedTools', ...DISALLOWED]
  const run = await runSession(args, session)
  writeFileSync(join(session.dir, 'transcript.jsonl'), run.stdout)
  writeFileSync(join(session.dir, 'stderr.txt'), run.stderr)
  const { records, unparsed } = parsedLines(run.stdout)
  const init = records.find(record => record['type'] === 'system' && record['subtype'] === 'init')
  const result = records.findLast(record => record['type'] === 'result')
  const tools = init?.['tools']
  const offered = Array.isArray(tools) ? tools.filter((tool: unknown): tool is string => typeof tool === 'string' && tool.startsWith(LIBRARY_TOOL_PREFIX)) : []
  const denials = result?.['permission_denials']
  const denied = Array.isArray(denials) ? denials.map((denial: unknown) => (isRecord(denial) ? String(denial['tool_name']) : String(denial))) : []
  const connected = libraryServerLine(init)
  const headCall = headCallLine(records, session.marker)
  const ours = thisRepoLine(init, headCall.live, expected, run)
  const report = [
    `mode: ${mode === 'proof' ? 'proof, this build is first on PATH' : 'control, this build is not on PATH, so lines 1 to 3 must fail'}`,
    `command: (cd ${session.dir} && ${mode === 'proof' ? `PATH=${REX_DIR}:$PATH ` : ''}${MARKER_ENV}=${session.marker} ${CLAUDE} ${args.map(arg => JSON.stringify(arg)).join(' ')})`,
    `marker: ${session.marker}`,
    `rex on the session PATH: ${resolved}`,
    `head, direct: ${expected.ok ? JSON.stringify(expected.head) : expected.why}`,
    `exit: ${run.exit}; cost usd ${String(result?.['total_cost_usd'] ?? 'unknown')}; unparsed transcript lines ${String(unparsed)}`,
    `raw library tools offered to the model: ${offered.length === 0 ? 'none' : offered.join(', ')}`,
    `permission denials: ${denied.length === 0 ? 'none' : denied.join(', ')}`,
    `final answer: ${typeof result?.['result'] === 'string' ? result['result'] : 'none'}`,
    `transcript: ${join(session.dir, 'transcript.jsonl')}`,
    ...debugLinesOf(session.debugLog).map(line => `debug: ${line}`),
    '',
    `(1) library server connected at init: ${connected.verdict}`,
    ...connected.evidence.map(line => `    ${line}`),
    `(2) the mod's $.mcp.call of head: ${headCall.line.verdict}`,
    ...headCall.line.evidence.map(line => `    ${line}`),
    `(3) the connected server is this checkout's: ${ours.verdict}`,
    ...ours.evidence.map(line => `    ${line}`),
    `(4) the mod's own call is not refused inside a job: ${IN_JOB_LINE.verdict}`,
  ]
  process.stdout.write(`${report.join('\n')}\n`)
  const verdicts = [connected.verdict, headCall.line.verdict, ours.verdict]
  process.exitCode = verdicts.some(verdict => verdict.startsWith('FAILED')) ? 1 : 0
}

const asked = process.argv.slice(2)
if (asked.length === 0 || (asked.length === 1 && asked[0] === 'control')) {
  await main(asked.length === 0 ? 'proof' : 'control')
} else {
  process.stdout.write(`usage: node run.ts [control], got ${asked.join(' ')}\n`)
  process.exitCode = 2
}

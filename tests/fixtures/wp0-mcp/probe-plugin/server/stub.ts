// A hand-written MCP server over stdio for the WP0 proof: its one tool echoes the nonce it is given, and every message it gets is appended to a log so a run can show which calls reached it.
// It runs under node with type stripping and uses built-in modules only.
import { appendFileSync } from 'node:fs'
import { join } from 'node:path'
import { createInterface } from 'node:readline'

type RequestId = string | number

type Message = { readonly id?: RequestId; readonly method: string; readonly params: Record<string, unknown> }

const SERVER_INFO = { name: 'wp0-mcp-stub', version: '0.0.1' }
const DEFAULT_PROTOCOL = '2025-06-18'
const METHOD_NOT_FOUND = -32601
const PARSE_ERROR = -32700

const ECHO_TOOL = {
  name: 'echo',
  description: 'Echoes the nonce and caller it is given, and logs the call.',
  inputSchema: { type: 'object', properties: { nonce: { type: 'string' }, caller: { type: 'string' } }, required: ['nonce'] },
}

/** The runner names the log; without it the log goes to the session's project folder. */
function logPath(): string {
  return process.env['WP0_MCP_LOG'] ?? join(process.env['CLAUDE_PROJECT_DIR'] ?? process.cwd(), 'wp0-mcp-stub.log')
}

function log(entry: Record<string, unknown>): void {
  appendFileSync(logPath(), `${JSON.stringify({ at: new Date().toISOString(), pid: process.pid, ...entry })}\n`)
}

function send(message: Record<string, unknown>): void {
  process.stdout.write(`${JSON.stringify({ jsonrpc: '2.0', ...message })}\n`)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function parse(line: string): Message | null {
  const value: unknown = JSON.parse(line)
  if (!isRecord(value) || typeof value['method'] !== 'string') {
    return null
  }
  const id = value['id']
  const params = isRecord(value['params']) ? value['params'] : {}
  return typeof id === 'string' || typeof id === 'number' ? { id, method: value['method'], params } : { method: value['method'], params }
}

function textOf(value: unknown): string {
  return typeof value === 'string' ? value : 'none'
}

function callTool(params: Record<string, unknown>): Record<string, unknown> {
  const name = textOf(params['name'])
  const args = isRecord(params['arguments']) ? params['arguments'] : {}
  if (name !== ECHO_TOOL.name) {
    return { content: [{ type: 'text', text: `no tool ${name}; the only tool is ${ECHO_TOOL.name}` }], isError: true }
  }
  const nonce = textOf(args['nonce'])
  const caller = textOf(args['caller'])
  log({ event: 'tools/call', tool: name, nonce, caller })
  return { content: [{ type: 'text', text: `stub-echo nonce=${nonce} caller=${caller} pid=${String(process.pid)}` }], isError: false }
}

function answer(message: Message): Record<string, unknown> | null {
  switch (message.method) {
    case 'initialize':
      log({ event: 'initialize', client: message.params['clientInfo'] ?? null })
      return { protocolVersion: typeof message.params['protocolVersion'] === 'string' ? message.params['protocolVersion'] : DEFAULT_PROTOCOL, capabilities: { tools: {} }, serverInfo: SERVER_INFO }
    case 'tools/list':
      return { tools: [ECHO_TOOL] }
    case 'tools/call':
      return callTool(message.params)
    case 'ping':
      return {}
    default:
      return null
  }
}

function handle(line: string): void {
  if (line.trim() === '') {
    return
  }
  let message: Message | null
  try {
    message = parse(line)
  } catch (error) {
    send({ id: null, error: { code: PARSE_ERROR, message: `not JSON: ${error instanceof Error ? error.message : String(error)}` } })
    return
  }
  if (message === null || message.id === undefined) {
    return
  }
  const result = answer(message)
  if (result === null) {
    send({ id: message.id, error: { code: METHOD_NOT_FOUND, message: `no method ${message.method}` } })
    return
  }
  send({ id: message.id, result })
}

log({ event: 'start', cwd: process.cwd() })
createInterface({ input: process.stdin }).on('line', handle)

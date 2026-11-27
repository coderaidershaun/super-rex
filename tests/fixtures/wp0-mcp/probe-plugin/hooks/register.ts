// The mod of a throwaway plugin that proves a mod can reach an MCP server its own manifest lists, through `$.mcp.call` alone.
// One tool calls the stub server and wraps its answer in a marker; the other spawns an agent the way a master spawns a builder, so that agent can call the first.
import type { EngineInterface, HookFailure, Register, ToolCallResult } from 'claude-code'

const PLUGIN = 'wp0-mcp-probe'
const TOOL_PREFIX = `mcp__${PLUGIN}__`
const PROBE_TOOL = 'mcp_probe'
const SPAWN_TOOL = 'spawn_probe'
const STUB_KEY = 'stub'
const STUB_NAME = `plugin:${PLUGIN}:${STUB_KEY}`
const STUB_TOOL = 'echo'
const MARKER = 'WP0-MCP-MARKER'

/** The agent type a master gives a builder; any `super-rex:model-*` grid agent stands in for it. */
const BUILDER_TYPE = 'super-rex:model-low-effort-low'

type ToolEvent = { readonly agentId?: string; readonly [argument: string]: unknown }

type StubReply = { readonly server: string; readonly route: string; readonly text: string; readonly isError: boolean }

function errorText(error: unknown): string {
  return error instanceof Error ? `${error.name}: ${error.message}` : String(error)
}

function textArg(event: ToolEvent, key: string): string | null {
  const value = event[key]
  return typeof value === 'string' && value.trim() !== '' ? value : null
}

function seen(value: unknown): string {
  return value === undefined ? 'none' : (JSON.stringify(value) ?? String(value))
}

async function callOnce($: EngineInterface, server: string, nonce: string, caller: string): Promise<{ readonly text: string; readonly isError: boolean }> {
  const result = await $.mcp.call(server, STUB_TOOL, { nonce, caller })
  const text = result.content.map(block => block.text ?? '').join('')
  return { text, isError: result.isError }
}

/**
 * Calls the stub by the name the engine gives a plugin's server. When that
 * name is wrong, it asks `$.mcp.connect` for the real one and says so in
 * `route`, because later work needs to know which of the two is required.
 */
async function callStub($: EngineInterface, nonce: string, caller: string): Promise<StubReply> {
  try {
    const reply = await callOnce($, STUB_NAME, nonce, caller)
    return { server: STUB_NAME, route: 'call', ...reply }
  } catch (error) {
    const direct = errorText(error)
    const connected = await $.mcp.connect(STUB_KEY)
    if (!connected.isConnected) {
      throw new Error(`$.mcp.call(${STUB_NAME}) failed with ${direct}, and $.mcp.connect(${STUB_KEY}) refused with ${connected.reason}: ${connected.message}`)
    }
    const reply = await callOnce($, connected.server, nonce, caller)
    return { server: connected.server, route: `connect-then-call, the direct call failed with ${direct}`, ...reply }
  }
}

async function serveProbe($: EngineInterface, event: ToolEvent): Promise<ToolCallResult> {
  const nonce = textArg(event, 'nonce')
  if (nonce === null) {
    return { deny: `${PROBE_TOOL}: nonce must be text, got ${seen(event['nonce'])}; pass the nonce you were given` }
  }
  const caller = event.agentId ?? 'main'
  const reply = await callStub($, nonce, caller)
  return { result: `${MARKER} caller=${caller} server=${reply.server} route=${reply.route} isError=${String(reply.isError)} answer=[${reply.text}] ${MARKER}-END` }
}

function builderPrompt(nonce: string): string {
  return `Call the tool ${TOOL_PREFIX}${PROBE_TOOL} once with {"nonce": "${nonce}"}. Then reply with its output exactly as it came back, and nothing else.`
}

/**
 * Starts an agent from inside a tool hook, as the master's build tool starts
 * a builder: a grid agent type and a folder other than the session's. It
 * returns once the agent has started, because a hook may not wait on it.
 */
async function serveSpawn($: EngineInterface, event: ToolEvent): Promise<ToolCallResult> {
  const nonce = textArg(event, 'nonce')
  const cwd = textArg(event, 'cwd')
  if (nonce === null || cwd === null) {
    return { deny: `${SPAWN_TOOL}: nonce and cwd must be text, got nonce ${seen(event['nonce'])} and cwd ${seen(event['cwd'])}; pass both as you were given them` }
  }
  const spawned = await $.agent.spawn({ subagentType: BUILDER_TYPE, description: `wp0 builder ${nonce.slice(0, 8)}`, prompt: builderPrompt(nonce), cwd })
  if (spawned.deny !== undefined || spawned.agentId === undefined) {
    return { deny: `${SPAWN_TOOL}: the agent did not start, got ${spawned.deny ?? 'no agent id'}` }
  }
  return { result: `${SPAWN_TOOL}: started agent=${spawned.agentId} type=${BUILDER_TYPE} model=${spawned.model} cwd=${cwd}; its answer arrives when it finishes` }
}

function refusal(tool: string, error: HookFailure): ToolCallResult {
  return { deny: `${tool} failed (${error.kind}): ${error.message ?? 'no message'}` }
}

export const register: Register = on => {
  on('session.start', { cwd: /(?:)/ }, async ($, e, next) => {
    await $.tool.register({
      name: PROBE_TOOL,
      description: 'Calls the stub MCP server of this plugin through the mod and returns its answer with a marker. Pass the nonce you were given.',
      inputSchema: { type: 'object', properties: { nonce: { type: 'string' } }, required: ['nonce'] },
      isDeferred: false,
    })
    await $.tool.register({
      name: SPAWN_TOOL,
      description: `Starts a ${BUILDER_TYPE} agent in the folder cwd that calls ${PROBE_TOOL} with the nonce. Pass the nonce and cwd you were given.`,
      inputSchema: { type: 'object', properties: { nonce: { type: 'string' }, cwd: { type: 'string' } }, required: ['nonce', 'cwd'] },
      isDeferred: false,
    })
    return next(e)
  })

  on('tool.call', { tool: `${TOOL_PREFIX}${PROBE_TOOL}` }, ($, e) => serveProbe($, e)).catch((_$, _e, next) => refusal(PROBE_TOOL, next.error))
  on('tool.call', { tool: `${TOOL_PREFIX}${SPAWN_TOOL}` }, ($, e) => serveSpawn($, e)).catch((_$, _e, next) => refusal(SPAWN_TOOL, next.error))
}

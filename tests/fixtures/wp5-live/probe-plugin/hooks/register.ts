// The mod of a throwaway plugin that proves, in a live session, that a mod gets the answer of `head` from the super-rex plugin's library server through `$.mcp.call`.
// Its one tool wraps the raw answer in a marker that holds the run's own nonce, so that an older or cached answer cannot pass.
import type { EngineInterface, HookFailure, Register, ToolCallResult } from 'claude-code'

const PROBE_TOOL = 'head_probe'
const PROBE_TOOL_NAME = `mcp__wp5-live-probe__${PROBE_TOOL}`
const LIBRARY_SERVER = 'plugin:super-rex:library'
const HEAD_TOOL = 'head'
const MARKER = 'WP5-LIVE-MARKER'

type ToolEvent = { readonly agentId?: string; readonly [argument: string]: unknown }

type HeadReply = { readonly server: string; readonly route: string; readonly text: string; readonly isError: boolean }

function errorText(error: unknown): string {
  return error instanceof Error ? `${error.name}: ${error.message}` : String(error)
}

function seen(value: unknown): string {
  return value === undefined ? 'none' : (JSON.stringify(value) ?? String(value))
}

async function callHead($: EngineInterface, server: string): Promise<{ readonly text: string; readonly isError: boolean }> {
  const result = await $.mcp.call(server, HEAD_TOOL, {})
  const text = result.content.map(block => block.text ?? '').join('')
  return { text, isError: result.isError }
}

/**
 * Calls `head` by the name the engine gives the other plugin's server. When
 * that call throws, it asks `$.mcp.connect` for the name and says so in
 * `route`, so the run shows which of the two ways the engine took.
 */
async function headOfLibrary($: EngineInterface): Promise<HeadReply> {
  try {
    const reply = await callHead($, LIBRARY_SERVER)
    return { server: LIBRARY_SERVER, route: 'call', ...reply }
  } catch (error) {
    const direct = errorText(error)
    const connected = await $.mcp.connect(LIBRARY_SERVER)
    if (!connected.isConnected) {
      throw new Error(`$.mcp.call(${LIBRARY_SERVER}, ${HEAD_TOOL}) failed with ${direct}, and $.mcp.connect(${LIBRARY_SERVER}) refused with ${connected.reason}: ${connected.message}`)
    }
    const reply = await callHead($, connected.server)
    return { server: connected.server, route: `connect-then-call, the direct call failed with ${direct}`, ...reply }
  }
}

async function serveProbe($: EngineInterface, event: ToolEvent): Promise<ToolCallResult> {
  const marker = event['marker']
  if (typeof marker !== 'string' || marker.trim() === '') {
    return { deny: `${PROBE_TOOL}: marker must be text, got ${seen(marker)}; pass the marker you were given` }
  }
  const caller = event.agentId ?? 'main'
  const reply = await headOfLibrary($)
  return { result: `${MARKER} marker=${marker} caller=${caller} server=${reply.server} route=${reply.route} isError=${String(reply.isError)} answer=[${reply.text}] ${MARKER}-END` }
}

function refusal(error: HookFailure): ToolCallResult {
  return { deny: `${PROBE_TOOL} failed (${error.kind}): ${error.message ?? 'no message'}` }
}

export const register: Register = on => {
  on('session.start', { cwd: /(?:)/ }, async ($, e, next) => {
    await $.tool.register({
      name: PROBE_TOOL,
      description: `Calls ${HEAD_TOOL} on the super-rex library server through the mod and returns its answer with a marker. Pass the marker you were given.`,
      inputSchema: { type: 'object', properties: { marker: { type: 'string' } }, required: ['marker'] },
      isDeferred: false,
    })
    return next(e)
  })

  on('tool.call', { tool: PROBE_TOOL_NAME }, ($, e) => serveProbe($, e)).catch((_$, _e, next) => refusal(next.error))
}

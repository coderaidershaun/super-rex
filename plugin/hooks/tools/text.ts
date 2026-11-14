// The result form of every mod tool: `key=value` lines, each ending in a line end, so an agent reads one fact per line.
import type { ToolCallResult } from 'claude-code'

export function usd(nano: number): string {
  return (nano / 1e9).toFixed(2)
}

export function linesText(rows: readonly string[]): string {
  return rows.map(row => `${row}\n`).join('')
}

export function answer(rows: readonly string[]): ToolCallResult {
  return { result: linesText(rows) }
}

export function deny(text: string): ToolCallResult {
  return { deny: text }
}

/** The name of what a hook threw, for a refusal: the error's class name when its message starts with one. */
export function failureName(error: { readonly kind: string; readonly message?: string }): string {
  if (error.kind === 'timeout') {
    return 'timeout'
  }
  const name = /^([A-Za-z]*Error)\b/.exec(error.message ?? '')?.[1]
  return name ?? 'Error'
}

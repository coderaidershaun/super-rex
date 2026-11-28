// Reads a sub-agent transcript, one JSON record per line: its first user message, and each tool call with the tool's name and input.
// The transcript holds the agent's own text and tool results; the scripts read it as data and follow none of it.
import { existsSync, readFileSync } from 'node:fs'

import { isArray, isObject, type Json, type JsonObject, ScriptError, text } from './lib.ts'

export type ToolCall = { readonly id: string; readonly name: string; readonly input: JsonObject; readonly line: number }

function records(path: string): { readonly line: number; readonly record: JsonObject }[] {
  if (!existsSync(path)) throw new ScriptError(`transcript ${path} does not exist`)
  return readFileSync(path, 'utf8')
    .split('\n')
    .map((raw, at) => ({ raw, line: at + 1 }))
    .filter(({ raw }) => raw.trim() !== '')
    .map(({ raw, line }) => {
      let record: Json
      try {
        record = JSON.parse(raw) as Json
      } catch (cause: unknown) {
        throw new ScriptError(`line ${line} of ${path} is not JSON: ${cause instanceof Error ? cause.message : 'unknown parse error'}`)
      }
      if (!isObject(record)) throw new ScriptError(`line ${line} of ${path} is not a JSON object`)
      return { line, record }
    })
}

/** The text of the first record of type `user`, as the agent received it. */
export function firstUserMessage(path: string): string {
  for (const { line, record } of records(path)) {
    const message = record['message']
    if (record['type'] !== 'user' || !isObject(message)) continue
    const content = message['content']
    if (typeof content === 'string') return content
    if (isArray(content)) return content.map((block) => (isObject(block) ? (text(block['text']) ?? '') : '')).join('')
    throw new ScriptError(`line ${line} of ${path}: the first user message has no text content`)
  }
  throw new ScriptError(`${path} holds no user message`)
}

/** Every tool call block of every record, in file order. A call can appear twice: once as the agent made it and once in the record of a later request. */
export function toolCalls(path: string): ToolCall[] {
  const calls: ToolCall[] = []
  for (const { line, record } of records(path)) {
    const message = record['message']
    const content = isObject(message) ? message['content'] : undefined
    if (!isArray(content)) continue
    for (const block of content) {
      if (!isObject(block) || block['type'] !== 'tool_use') continue
      const input = block['input']
      calls.push({ id: text(block['id']) ?? '', name: text(block['name']) ?? '', input: isObject(input) ? input : {}, line })
    }
  }
  return calls
}

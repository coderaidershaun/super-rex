// Calls the rex binary for what only the store knows; a refusal of the binary reaches the caller as its own first stderr line.
import type { EngineInterface } from 'claude-code'

import { fail } from '../book/index'

export type BinaryRead = { readonly ok: true; readonly stdout: string } | { readonly ok: false; readonly line: string }

export async function readBinary($: EngineInterface, argv: readonly string[]): Promise<BinaryRead> {
  const ran = await $.process.run(argv, { cwd: await $.session.root() })
  if (ran.exitCode === 0) {
    return { ok: true, stdout: ran.stdout }
  }
  await fail($, { kind: 'binary-read-failed', argv, exitCode: ran.exitCode, stderr: ran.stderr })
  const line = ran.stderr.split('\n')[0]?.trim() ?? ''
  return { ok: false, line: line === '' ? `${argv.join(' ')} exited ${String(ran.exitCode)}` : line }
}

/** One field of a parsed JSON value from the binary, or undefined. */
export function jsonField(value: unknown, key: string): unknown {
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? Reflect.get(value, key) : undefined
}

/** The binary's JSON answer, or undefined when it printed something else. */
export function parsedJson(text: string): unknown {
  try {
    const parsed: unknown = JSON.parse(text)
    return parsed
  } catch (error) {
    if (error instanceof SyntaxError) {
      return undefined
    }
    throw error
  }
}

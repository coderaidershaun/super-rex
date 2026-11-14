// Reads the arguments of the master's tools, which come from the model as unchecked JSON, and words each refusal as every mod tool does: what is wrong, the value seen, what to do.

export type Refused = { readonly refused: string }

export function isRefused(value: unknown): value is Refused {
  return typeof value === 'object' && value !== null && typeof Reflect.get(value, 'refused') === 'string'
}

function seenText(seen: unknown): string {
  if (seen === undefined) {
    return 'none'
  }
  return typeof seen === 'string' ? seen : (JSON.stringify(seen) ?? String(seen))
}

export function refused(tool: string, wrong: string, seen: unknown, todo: string): Refused {
  return { refused: `${tool}: ${wrong}, got ${seenText(seen)}; ${todo}` }
}

export function field(event: object, key: string): unknown {
  return Object.hasOwn(event, key) ? Reflect.get(event, key) : undefined
}

export function text(tool: string, event: object, key: string, todo: string): string | Refused {
  const value = field(event, key)
  return typeof value === 'string' && value.trim() !== '' ? value : refused(tool, `${key} must be text`, value, todo)
}

export function optionalText(tool: string, event: object, key: string): string | null | Refused {
  const value = field(event, key)
  if (value === undefined || value === null) {
    return null
  }
  return typeof value === 'string' && value.trim() !== '' ? value : refused(tool, `${key} must be text`, value, `call it again with ${key} as text, or without it`)
}

export function optionalWord<T extends string>(tool: string, event: object, key: string, words: readonly T[]): T | null | Refused {
  const value = field(event, key)
  if (value === undefined || value === null) {
    return null
  }
  return words.find(each => each === value) ?? refused(tool, `${key} must be one of ${words.join(', ')}`, value, 'call it again with one of them')
}

export function word<T extends string>(tool: string, event: object, key: string, words: readonly T[]): T | Refused {
  const value = optionalWord(tool, event, key, words)
  return value ?? refused(tool, `${key} must be one of ${words.join(', ')}`, undefined, 'call it again with one of them')
}

export function textList(tool: string, event: object, key: string): readonly string[] | Refused {
  const value = field(event, key)
  if (value === undefined || value === null) {
    return []
  }
  const isTextList = Array.isArray(value) && value.every((item: unknown) => typeof item === 'string')
  return isTextList ? value.map(String) : refused(tool, `${key} must be a list of job ids`, value, `call it again with a list such as ["0gaq820001"]`)
}

export function optionalNumber(tool: string, event: object, key: string, most: number): number | null | Refused {
  const value = field(event, key)
  if (value === undefined || value === null) {
    return null
  }
  const isAmount = typeof value === 'number' && Number.isFinite(value) && value >= 0 && value <= most
  return isAmount ? value : refused(tool, `${key} must be an amount from 0 to ${String(most)}`, value, 'call it again with a number of dollars')
}

export function optionalObject(tool: string, event: object, key: string): object | null | Refused {
  const value = field(event, key)
  if (value === undefined || value === null) {
    return null
  }
  return typeof value === 'object' && !Array.isArray(value) ? value : refused(tool, `${key} must be an object`, value, 'call it again with an object')
}

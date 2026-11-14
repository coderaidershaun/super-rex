// Reads a mod tool's arguments, which come from the model as unchecked JSON, and words each refusal the same way: what is wrong, the value seen, what to do.

/** A tool call refused before anything changed; `text` is what the model reads. */
export type Refused = { readonly refused: string }

export function isRefused(value: unknown): value is Refused {
  return typeof value === 'object' && value !== null && typeof Reflect.get(value, 'refused') === 'string'
}

export function refused(tool: string, wrong: string, seen: unknown, todo: string): Refused {
  return { refused: `${tool}: ${wrong}, got ${seenText(seen)}; ${todo}` }
}

/** How a value seen reads in a refusal: text as it is, anything else as JSON, nothing as `none`. */
export function seenText(seen: unknown): string {
  if (seen === undefined) {
    return 'none'
  }
  return typeof seen === 'string' ? seen : (JSON.stringify(seen) ?? String(seen))
}

/** The tool's own arguments: every field of the call but the ones the engine sets. */
export type Args = { readonly [key: string]: unknown }

const ENGINE_KEYS = new Set(['tool', 'tool_use_id', 'agentId', 'requestMeta', 'consent'])

export function argsOf(event: object): Args {
  return Object.fromEntries(Object.entries(event).filter(([key]) => !ENGINE_KEYS.has(key)))
}

export function field(args: Args, key: string): unknown {
  return Object.hasOwn(args, key) ? args[key] : undefined
}

export function textArg(tool: string, args: Args, key: string, todo: string): string | Refused {
  const value = field(args, key)
  return typeof value === 'string' && value.trim() !== '' ? value : refused(tool, `${key} must be text`, value, todo)
}

export function optionalText(tool: string, args: Args, key: string, todo: string): string | null | Refused {
  const value = field(args, key)
  if (value === undefined || value === null) {
    return null
  }
  return typeof value === 'string' ? value : refused(tool, `${key} must be text`, value, todo)
}

/** One of a fixed list of words, or null when the argument is left out. */
export function optionalWord<T extends string>(tool: string, args: Args, key: string, words: readonly T[]): T | null | Refused {
  const value = field(args, key)
  if (value === undefined || value === null) {
    return null
  }
  const word = words.find(each => each === value)
  return word ?? refused(tool, `${key} must be one of ${words.join(', ')}`, value, `call it again with one of them`)
}

export function word<T extends string>(tool: string, args: Args, key: string, words: readonly T[]): T | Refused {
  const value = optionalWord(tool, args, key, words)
  return value === null ? refused(tool, `${key} must be one of ${words.join(', ')}`, undefined, 'call it again with one of them') : value
}

export function optionalTextList(tool: string, args: Args, key: string): readonly string[] | Refused {
  const value = field(args, key)
  if (value === undefined || value === null) {
    return []
  }
  const isTextList = Array.isArray(value) && value.every((item: unknown) => typeof item === 'string')
  return isTextList ? value.map(String) : refused(tool, `${key} must be a list of text`, value, 'call it again with a list such as ["rust"]')
}

export function objectList(tool: string, args: Args, key: string, todo: string): readonly Args[] | Refused {
  const value = field(args, key)
  if (!Array.isArray(value)) {
    return refused(tool, `${key} must be a list`, value, todo)
  }
  const objects: Args[] = []
  for (const item of value) {
    if (typeof item !== 'object' || item === null || Array.isArray(item)) {
      return refused(tool, `each item of ${key} must be an object`, item, todo)
    }
    objects.push(argsOf(item))
  }
  return objects
}

export function optionalObject(tool: string, args: Args, key: string): Args | null | Refused {
  const value = field(args, key)
  if (value === undefined || value === null) {
    return null
  }
  return typeof value === 'object' && !Array.isArray(value) ? argsOf(value) : refused(tool, `${key} must be an object`, value, 'call it again with an object')
}

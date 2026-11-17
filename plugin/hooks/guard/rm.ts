// The guard's pure decision: which folders a recursive rm in a shell command would delete, and whether one of them lies outside the places agents may clear.
// It reads the command as text and follows no symbolic link, so it is a tripwire for a wrong path, not a sandbox.

/** One piece of a shell word: text as the program gets it, or something only the shell knows. */
type Part =
  | { readonly kind: 'text'; readonly text: string }
  | { readonly kind: 'var'; readonly name: string }
  | { readonly kind: 'home' }
  | { readonly kind: 'glob' }
  | { readonly kind: 'unknown' }

type Word = readonly Part[]

/** The folders the guard decides by, each an absolute path with no `/` at its end. */
export type RmPlaces = {
  /** Where the command runs; null when the mod cannot tell. */
  readonly cwd: string | null
  readonly home: string | null
  /** The main checkout of the session's repo. */
  readonly repo: string
  /** The folder that holds the main checkout, or null when that folder holds the home folder or is the root. */
  readonly parent: string | null
  /** The temp folders: `/tmp`, the system's own, and `TMPDIR`. */
  readonly temp: readonly string[]
}

/** One recursive rm the guard stops: the folder as the guard resolved it, and why it is not one to delete. */
export type RmAlert = { readonly target: string; readonly reason: string }

const NAME = /^[A-Za-z_][A-Za-z0-9_]*$/
const NAME_START = /[A-Za-z_]/
const NAME_CHAR = /[A-Za-z0-9_]/

/** The index of the bracket that closes the one before `from`, or the end of the text when none does. */
function closing(text: string, from: number, open: string, close: string): number {
  let depth = 1
  for (let index = from; index < text.length; index += 1) {
    const char = text[index]
    if (char === '\\') {
      index += 1
    } else if (char === open) {
      depth += 1
    } else if (char === close) {
      depth -= 1
      if (depth === 0) {
        return index
      }
    }
  }
  return text.length
}

/**
 * The simple commands of a shell line, each as its words. A command
 * substitution is read as commands of its own; a redirect and its target are
 * left out, and so is the body of a here-document, which is data.
 */
export function commandsOf(line: string): readonly (readonly Word[])[] {
  const commands: Word[][] = []
  let words: Word[] = []
  let parts: Part[] = []
  let isWordOpen = false
  let isRedirectTarget = false
  const heredocs: string[] = []

  const addText = (text: string): void => {
    const last = parts[parts.length - 1]
    if (last?.kind === 'text') {
      parts[parts.length - 1] = { kind: 'text', text: last.text + text }
    } else {
      parts.push({ kind: 'text', text })
    }
    isWordOpen = true
  }
  const add = (part: Part): void => {
    parts.push(part)
    isWordOpen = true
  }
  const endWord = (): void => {
    if (isWordOpen && !isRedirectTarget) {
      words.push(parts)
    }
    if (isWordOpen) {
      isRedirectTarget = false
    }
    parts = []
    isWordOpen = false
  }
  const endCommand = (): void => {
    endWord()
    if (words.length > 0) {
      commands.push(words)
    }
    words = []
  }
  /** Reads what follows a `$` at `at`; gives the index of its last character. */
  const dollar = (at: number): number => {
    const next = line[at + 1] ?? ''
    if (next === '(') {
      const end = closing(line, at + 2, '(', ')')
      commands.push(...commandsOf(line.slice(at + 2, end)).map(command => [...command]))
      add({ kind: 'unknown' })
      return end
    }
    if (next === '{') {
      const end = closing(line, at + 2, '{', '}')
      const inner = line.slice(at + 2, end)
      add(NAME.test(inner) ? { kind: 'var', name: inner } : { kind: 'unknown' })
      return end
    }
    if (NAME_START.test(next)) {
      let end = at + 1
      while (end + 1 < line.length && NAME_CHAR.test(line[end + 1] ?? '')) {
        end += 1
      }
      add({ kind: 'var', name: line.slice(at + 1, end + 1) })
      return end
    }
    if (next === '' || /\s/.test(next) || next === '"') {
      addText('$')
      return at
    }
    add({ kind: 'unknown' })
    return at + 1
  }
  /** Reads a command in backticks that opens at `at`; gives the index of the closing one. */
  const backtick = (at: number): number => {
    let end = at + 1
    while (end < line.length && line[end] !== '`') {
      end += line[end] === '\\' ? 2 : 1
    }
    commands.push(...commandsOf(line.slice(at + 1, end)).map(command => [...command]))
    add({ kind: 'unknown' })
    return end
  }

  let index = 0
  while (index < line.length) {
    const char = line[index] ?? ''
    if (char === "'") {
      const end = line.indexOf("'", index + 1)
      addText(line.slice(index + 1, end === -1 ? line.length : end))
      index = end === -1 ? line.length : end + 1
      continue
    }
    if (char === '"') {
      isWordOpen = true
      index += 1
      while (index < line.length && line[index] !== '"') {
        const inner = line[index] ?? ''
        if (inner === '\\' && /[$`"\\\n]/.test(line[index + 1] ?? '')) {
          if (line[index + 1] !== '\n') {
            addText(line[index + 1] ?? '')
          }
          index += 2
        } else if (inner === '$') {
          index = dollar(index) + 1
        } else if (inner === '`') {
          index = backtick(index) + 1
        } else {
          addText(inner)
          index += 1
        }
      }
      index += 1
      continue
    }
    if (char === '\\') {
      if (line[index + 1] !== '\n') {
        addText(line[index + 1] ?? '')
      }
      index += 2
      continue
    }
    if (char === '\n') {
      endCommand()
      index += 1
      while (heredocs.length > 0) {
        const delimiter = heredocs.shift() ?? ''
        while (index < line.length) {
          const end = line.indexOf('\n', index)
          const row = line.slice(index, end === -1 ? line.length : end)
          index = end === -1 ? line.length : end + 1
          if (row.trim() === delimiter) {
            break
          }
        }
      }
      continue
    }
    if (char === ' ' || char === '\t') {
      endWord()
      index += 1
      continue
    }
    if (char === '#' && !isWordOpen) {
      const end = line.indexOf('\n', index)
      index = end === -1 ? line.length : end
      continue
    }
    if (char === '<' && line[index + 1] === '<' && line[index + 2] !== '<') {
      endWord()
      const named = /^<<-?\s*(?:'([^']*)'|"([^"]*)"|\\?([^\s;|&<>()]+))/.exec(line.slice(index))
      if (named !== null) {
        heredocs.push(named[1] ?? named[2] ?? named[3] ?? '')
        index += named[0].length
        continue
      }
    }
    if (char === '>' || char === '<') {
      // A file descriptor before the redirect, as in `2>`, is no word of the command.
      const isDescriptor = parts.length === 1 && parts[0]?.kind === 'text' && /^\d+$/.test(parts[0].text)
      if (isDescriptor) {
        parts = []
        isWordOpen = false
      }
      endWord()
      index += 1
      while (line[index] === '>' || line[index] === '<' || line[index] === '|') {
        index += 1
      }
      if (line[index] === '&') {
        index += 1
        while (/[\d-]/.test(line[index] ?? '')) {
          index += 1
        }
      } else if (line[index] !== '(') {
        isRedirectTarget = true
      }
      continue
    }
    if (char === ';' || char === '|' || char === '&' || char === '(' || char === ')') {
      endCommand()
      index += 1
      continue
    }
    if (char === '$') {
      index = dollar(index) + 1
      continue
    }
    if (char === '`') {
      index = backtick(index) + 1
      continue
    }
    if (char === '~' && !isWordOpen) {
      const next = line[index + 1] ?? ''
      add(next === '' || next === '/' || /[\s;|&)]/.test(next) ? { kind: 'home' } : { kind: 'unknown' })
      index += 1
      continue
    }
    if (char === '*' || char === '?' || char === '[') {
      add({ kind: 'glob' })
      index += 1
      continue
    }
    addText(char)
    index += 1
  }
  endCommand()
  return commands
}

/** The word as plain text when the shell changes nothing in it, else null. */
function literal(word: Word | undefined): string | null {
  if (word === undefined) {
    return null
  }
  return word.every(part => part.kind === 'text') ? word.map(part => (part.kind === 'text' ? part.text : '')).join('') : null
}

/** The word as the line wrote it, near enough to run again as a command of its own. */
function written(word: Word): string {
  return word.map(part => (part.kind === 'text' ? part.text : part.kind === 'var' ? `$${part.name}` : part.kind === 'home' ? '~' : part.kind === 'glob' ? '*' : '$UNKNOWN')).join('')
}

function baseName(program: string): string {
  return program.slice(program.lastIndexOf('/') + 1)
}

/** Programs and shell words that run the command after them, with the options of each that take a value. */
const WRAPPERS: ReadonlyMap<string, ReadonlySet<string>> = new Map([
  ['env', new Set(['-u', '--unset', '-C', '--chdir', '-S', '--split-string'])],
  ['command', new Set<string>()],
  ['builtin', new Set<string>()],
  ['exec', new Set(['-a'])],
  ['nohup', new Set<string>()],
  ['time', new Set<string>()],
  ['nice', new Set(['-n'])],
  ['timeout', new Set(['-s', '--signal', '-k', '--kill-after'])],
  ['xargs', new Set(['-I', '-L', '-n', '-P', '-s', '-d', '-E', '-a'])],
  ['sudo', new Set(['-u', '-g', '-C', '-D', '-h', '-p', '-U'])],
  ['doas', new Set(['-u', '-C'])],
  ['stdbuf', new Set(['-i', '-o', '-e'])],
  ['if', new Set<string>()],
  ['then', new Set<string>()],
  ['else', new Set<string>()],
  ['elif', new Set<string>()],
  ['do', new Set<string>()],
  ['while', new Set<string>()],
  ['until', new Set<string>()],
  ['!', new Set<string>()],
  ['{', new Set<string>()],
])
const SHELLS = new Set(['sh', 'bash', 'zsh', 'dash', 'ksh', 'mksh', 'fish'])
const COUNT_OR_DURATION = /^\d+(\.\d+)?[smhd]?$/

/** What one simple command is to the guard, once its assignments and wrappers are passed. */
type Seen =
  | { readonly kind: 'rm'; readonly operands: readonly Word[] }
  | { readonly kind: 'cd'; readonly to: Word | null }
  | { readonly kind: 'script'; readonly text: string }
  | { readonly kind: 'set'; readonly values: readonly (readonly [string, Word | null])[] }
  | { readonly kind: 'other' }

/** Shell words that give variables a value for the commands after them. */
const DECLARES = new Set(['export', 'local', 'readonly', 'declare', 'typeset'])

/** The name and value of a word `NAME=value`, or null when the word is no assignment. */
function assigned(word: Word): readonly [string, Word] | null {
  const first = word[0]
  const match = first?.kind === 'text' ? /^([A-Za-z_][A-Za-z0-9_]*)=/.exec(first.text) : null
  if (first?.kind !== 'text' || match === null) {
    return null
  }
  const rest = first.text.slice(match[0].length)
  // The shell expands a `~` that opens the value, as it does one that opens a word.
  const isHome = rest === '~' || rest.startsWith('~/')
  const head: Word = rest === '' ? [] : isHome ? [{ kind: 'home' }, { kind: 'text', text: rest.slice(1) }] : [{ kind: 'text', text: rest }]
  return [match[1] ?? '', [...head, ...word.slice(1)]]
}

function seenIn(words: readonly Word[]): Seen {
  let rest = words
  for (;;) {
    const start = rest.findIndex(word => assigned(word) === null)
    if (start === -1) {
      // Only assignments: each holds for the commands after it.
      return { kind: 'set', values: rest.flatMap(word => [assigned(word)].filter(pair => pair !== null)) }
    }
    rest = rest.slice(start)
    const program = baseName(literal(rest[0]) ?? '')
    if (DECLARES.has(program)) {
      return { kind: 'set', values: rest.slice(1).flatMap(word => [assigned(word)].filter(pair => pair !== null)) }
    }
    if (program === 'for' || program === 'read') {
      // A loop or a read gives its variables values the guard cannot know.
      const names = rest.slice(1).map(word => literal(word) ?? '').filter(name => NAME.test(name) && name !== 'in')
      return { kind: 'set', values: (program === 'for' ? names.slice(0, 1) : names).map(name => [name, null]) }
    }
    const valued = WRAPPERS.get(program)
    if (valued !== undefined) {
      let index = 1
      while (index < rest.length) {
        const word = literal(rest[index]) ?? ''
        if (word === '--') {
          index += 1
          break
        }
        if (!word.startsWith('-') && !COUNT_OR_DURATION.test(word)) {
          break
        }
        index += valued.has(word) ? 2 : 1
      }
      rest = rest.slice(index)
      continue
    }
    if (program === 'eval') {
      return { kind: 'script', text: rest.slice(1).map(written).join(' ') }
    }
    if (SHELLS.has(program)) {
      const flag = rest.findIndex((word, index) => index > 0 && /^-[a-z]*c[a-z]*$/.test(literal(word) ?? ''))
      const script = flag === -1 ? undefined : rest[flag + 1]
      return script === undefined ? { kind: 'other' } : { kind: 'script', text: written(script) }
    }
    if (program === 'cd' || program === 'pushd') {
      return { kind: 'cd', to: rest[1] ?? null }
    }
    if (program === 'popd') {
      return { kind: 'cd', to: [{ kind: 'unknown' }] }
    }
    if (program !== 'rm') {
      return { kind: 'other' }
    }
    const operands: Word[] = []
    let isRecursive = false
    let isOptionsOver = false
    for (const word of rest.slice(1)) {
      const text = literal(word)
      if (!isOptionsOver && text === '--') {
        isOptionsOver = true
      } else if (!isOptionsOver && text !== null && text.startsWith('-') && text.length > 1) {
        isRecursive ||= text === '--recursive' || (!text.startsWith('--') && /[rR]/.test(text))
      } else {
        operands.push(word)
      }
    }
    return isRecursive ? { kind: 'rm', operands } : { kind: 'other' }
  }
}

/** A path with its `.` and `..` steps taken, by its text alone. */
function normal(path: string): string {
  const steps: string[] = []
  for (const step of path.split('/')) {
    if (step === '..') {
      steps.pop()
    } else if (step !== '' && step !== '.') {
      steps.push(step)
    }
  }
  return `/${steps.join('/')}`
}

/** True when `path` is `folder` or lies below it. */
function isAtOrBelow(path: string, folder: string): boolean {
  return path === folder || path.startsWith(folder === '/' ? '/' : `${folder}/`)
}

function isBelow(path: string, folder: string): boolean {
  return path !== folder && isAtOrBelow(path, folder)
}

/** The values the line itself gave its variables: a text, or null for a value only the shell knows. */
type Vars = ReadonlyMap<string, string | null>

/** A word as the shell expands it, as far as the text tells. */
type Expanded = {
  readonly text: string
  /** Where the first glob stands in `text`, or null when the word has none. */
  readonly glob: number | null
  /** A variable or a command that was never given a value in the line was taken as empty. */
  readonly hasUnknown: boolean
  /** A variable the line gave a value the guard cannot know: the word names nothing it can place. */
  readonly isOpaque: boolean
}

function expanded(word: Word, places: RmPlaces, vars: Vars): Expanded {
  let text = ''
  let glob: number | null = null
  let hasUnknown = false
  let isOpaque = false
  for (const part of word) {
    if (part.kind === 'text') {
      text += part.text
    } else if (part.kind === 'glob') {
      glob ??= text.length
      text += '*'
    } else if (part.kind === 'var' && vars.has(part.name)) {
      const value = vars.get(part.name) ?? null
      isOpaque ||= value === null
      text += value ?? ''
    } else if (part.kind === 'home' || (part.kind === 'var' && part.name === 'HOME')) {
      hasUnknown ||= places.home === null
      text += places.home ?? ''
    } else if (part.kind === 'var' && part.name === 'PWD') {
      isOpaque ||= places.cwd === null
      text += places.cwd ?? ''
    } else if (part.kind === 'var' && part.name === 'TMPDIR' && places.temp.length > 0) {
      text += places.temp[places.temp.length - 1] ?? ''
    } else {
      hasUnknown = true
    }
  }
  return { text, glob, hasUnknown, isOpaque }
}

/** What a word names once the shell has expanded it. */
type Named = {
  /** The absolute path, or null when the word names nothing the guard can place. */
  readonly path: string | null
  readonly hasUnknown: boolean
}

/**
 * The folder a word names. A variable the line never set counts as empty,
 * which is what the shell does with an unset one: `rm -rf "$DIR"/` then names
 * the root. A component that is only a glob names every entry of its folder,
 * so the folder itself is what is cleared; a glob inside a name stays an
 * entry of its folder.
 */
function namedBy(word: Word, places: RmPlaces, vars: Vars): Named {
  const { text, glob, hasUnknown, isOpaque } = expanded(word, places, vars)
  let path = text
  if (glob !== null) {
    const start = path.lastIndexOf('/', glob) + 1
    const end = path.indexOf('/', glob)
    const component = path.slice(start, end === -1 ? path.length : end)
    const folder = path.slice(0, start)
    // `*` and `.*` clear the folder; `build*` deletes entries of it.
    path = /^\.?\*+$/.test(component) ? (folder === '' ? '.' : folder) : `${folder}${component}`
  }
  if (isOpaque || path === '' || path === '-') {
    return { path: null, hasUnknown }
  }
  if (!path.startsWith('/')) {
    if (places.cwd === null) {
      return { path: null, hasUnknown }
    }
    path = `${places.cwd}/${path}`
  }
  return { path: normal(path), hasUnknown }
}

/** The folders in which a recursive rm may delete anything below them. */
export function clearable(places: RmPlaces): readonly string[] {
  return [places.parent ?? places.repo, `${places.repo}.worktrees`, ...places.temp]
}

/** Why a recursive rm of `path` is stopped, or null when it may run. */
function reasonFor(named: Named, places: RmPlaces): string | null {
  const path = named.path
  if (path === null) {
    return null
  }
  if (path === '/') {
    return 'the root folder'
  }
  if (places.home !== null && path === places.home) {
    return 'your home folder'
  }
  if (places.home !== null && isAtOrBelow(places.home, path)) {
    return 'a folder that holds your home folder'
  }
  if (named.hasUnknown) {
    return null
  }
  if (path === places.repo) {
    return 'the main checkout of this repo'
  }
  if (path === `${places.repo}/.git`) {
    return 'the Git folder of this repo'
  }
  if (isBelow(places.repo, path)) {
    return 'a folder that holds this repo'
  }
  if (clearable(places).some(folder => isBelow(path, folder))) {
    return null
  }
  return `outside ${places.parent ?? places.repo} and the temp folders`
}

/** How deep one command may run another before the guard stops reading: `sh -c "sh -c ..."` ends somewhere. */
const MAX_NESTING = 4

function alertsIn(line: string, start: RmPlaces, known: Vars, depth: number): readonly RmAlert[] {
  const alerts: RmAlert[] = []
  let places = start
  const vars = new Map(known)
  for (const words of commandsOf(line)) {
    const seen = seenIn(words)
    if (seen.kind === 'script' && depth < MAX_NESTING) {
      alerts.push(...alertsIn(seen.text, places, vars, depth + 1))
    }
    if (seen.kind === 'set') {
      for (const [name, value] of seen.values) {
        const given = value === null ? null : expanded(value, places, vars)
        vars.set(name, given === null || given.hasUnknown || given.isOpaque || given.glob !== null ? null : given.text)
      }
    }
    if (seen.kind === 'cd') {
      // `cd` with no folder goes home; a folder the guard cannot read leaves the place unknown.
      const to = seen.to === null ? { path: places.home, hasUnknown: false } : namedBy(seen.to, places, vars)
      places = { ...places, cwd: to.hasUnknown ? null : to.path }
    }
    if (seen.kind === 'rm') {
      for (const operand of seen.operands) {
        const named = namedBy(operand, places, vars)
        const reason = reasonFor(named, places)
        if (reason !== null && named.path !== null) {
          alerts.push({ target: named.path, reason })
        }
      }
    }
  }
  return alerts
}

/** A cheap first look, so a command with no rm in it costs the guard nothing more. */
export function mayRemove(command: string): boolean {
  return /\brm\b/.test(command)
}

/** The first recursive rm of the command that the guard stops, or null when each one may run. */
export function rmAlert(command: string, places: RmPlaces): RmAlert | null {
  return alertsIn(command, places, new Map(), 0)[0] ?? null
}

/** What the agent reads in place of the command's result. */
export function alertText(alert: RmAlert, places: RmPlaces): string {
  const allowed = clearable(places).join(', ')
  return `rex guard: ALERT: a recursive rm of ${alert.target} was stopped: it is ${alert.reason}. This does not look right. Check the path first. A recursive rm may delete only below ${allowed}. IF the person wants this delete, THEN tell them, and they run it in their own shell.`
}

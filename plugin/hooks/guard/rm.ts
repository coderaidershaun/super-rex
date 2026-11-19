// The guard's decision on deletes: a recursive rm or a find that deletes of the wrong folder, and a git clean of ignored files in the main checkout.
// It reads the command as text and follows no symbolic link, so it is a tripwire for a wrong path, not a sandbox.
import { alertOf } from './alert'
import type { Alert } from './alert'
import { baseName, clearable, hasShort, isAtOrBelow, isBelow, literal, machineReason, namedBy, runsOf, splitArgs } from './shell'
import type { Named, RmPlaces, Run, Word } from './shell'

/** One recursive rm the guard stops: the folder as the guard resolved it, and why it is not one to delete. */
export type RmAlert = { readonly target: string; readonly reason: string }

/** Why a recursive rm of `path` is stopped, or null when it may run. */
function reasonFor(named: Named, places: RmPlaces): string | null {
  const path = named.path
  if (path === null) {
    return null
  }
  const machine = machineReason(named, places)
  if (machine !== null) {
    return machine
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

function rmAlertsOf(run: Run): readonly RmAlert[] {
  if (run.program !== 'rm') {
    return []
  }
  const { options, plain } = splitArgs(run.args)
  const isRecursive = options.includes('--recursive') || options.some(option => !option.startsWith('--') && /[rR]/.test(option))
  if (!isRecursive) {
    return []
  }
  return plain.flatMap(operand => {
    const named = namedBy(operand, run.places, run.vars)
    const reason = reasonFor(named, run.places)
    return reason !== null && named.path !== null ? [{ target: named.path, reason }] : []
  })
}

/** The first recursive rm of the command that the guard stops, or null when each one may run. */
export function rmAlert(command: string, places: RmPlaces): RmAlert | null {
  return runsOf(command, places).flatMap(rmAlertsOf)[0] ?? null
}

/** What the agent reads in place of the command's result. */
export function alertText(alert: RmAlert, places: RmPlaces): string {
  const allowed = clearable(places).join(', ')
  return `rex guard: ALERT: a recursive rm of ${alert.target} was stopped: it is ${alert.reason}. This does not look right. Check the path first. A recursive rm may delete only below ${allowed}. IF the person wants this delete, THEN tell them, and they run it in their own shell.`
}

/** The tests of `find` that pick some files, so a delete with one of them in the repo is aimed, not a clear-out. */
const FIND_FILTER = /^-(i?name|i?path|i?regex|i?wholename|empty|newer\w*|[acm]time|[acm]min|size|user|perm|links|inum|samefile)$/
/** The options of `find` that come before its start folders, and those of them that take a value. */
const FIND_LEADING = /^-([HLPEXsx]+|O\d)$/

/** Why a find that deletes below `named` is stopped: it starts at the machine's own folders, at the repo with no test that picks files, or outside the folders agents may clear. */
function findReason(named: Named, places: RmPlaces, isFiltered: boolean): string | null {
  const path = named.path
  if (path === null) {
    return null
  }
  const machine = machineReason(named, places)
  if (machine !== null) {
    return machine
  }
  if (named.hasUnknown) {
    return null
  }
  const ofRepo = path === places.repo ? 'the main checkout of this repo' : path === `${places.repo}/.git` ? 'the Git folder of this repo' : isBelow(places.repo, path) ? 'a folder that holds this repo' : null
  if (ofRepo !== null) {
    return isFiltered ? null : ofRepo
  }
  if (clearable(places).some(folder => isAtOrBelow(path, folder))) {
    return null
  }
  return `a folder outside ${places.parent ?? places.repo} and the temp folders`
}

function findAlertsOf(run: Run): readonly Alert[] {
  if (run.program !== 'find') {
    return []
  }
  let index = 0
  while (index < run.args.length && FIND_LEADING.test(literal(run.args[index]) ?? '')) {
    index += 1
  }
  if (literal(run.args[index]) === '-D') {
    index += 2
  }
  const starts: Word[] = []
  while (index < run.args.length && !/^[-(!)]/.test(literal(run.args[index]) ?? 'x')) {
    starts.push(run.args[index] ?? [])
    index += 1
  }
  const expression = run.args.slice(index).map(word => literal(word) ?? '')
  const execs = expression.flatMap((word, at) => (/^-(exec|execdir|ok|okdir)$/.test(word) ? [baseName(expression[at + 1] ?? '')] : []))
  const how = expression.includes('-delete') ? 'find -delete' : execs.includes('rm') ? 'find -exec rm' : null
  if (how === null) {
    return []
  }
  const isFiltered = expression.some(word => FIND_FILTER.test(word))
  const folders: readonly Word[] = starts.length > 0 ? starts : [[{ kind: 'text', text: '.' }]]
  return folders.flatMap(start => {
    const named = namedBy(start, run.places, run.vars)
    const reason = findReason(named, run.places, isFiltered)
    return reason === null || named.path === null ? [] : [alertOf(`a ${how} in ${named.path}`, `deletes files below ${reason}`)]
  })
}

/** The options of git before its subcommand that take a value. */
const GIT_VALUED = new Set(['-C', '-c', '--git-dir', '--work-tree', '--namespace', '--exec-path', '--config-env'])

/** The subcommand of a git command, its words, and the folder it works in when the command names one with `-C` or `--work-tree`. */
export function gitCommand(run: Run): { readonly sub: string; readonly args: readonly Word[]; readonly dir: string | null } | null {
  if (run.program !== 'git') {
    return null
  }
  let dir = run.places.cwd
  let index = 0
  while (index < run.args.length) {
    const word = literal(run.args[index]) ?? ''
    if (!word.startsWith('-')) {
      break
    }
    const [name, value] = word.includes('=') ? [word.slice(0, word.indexOf('=')), word.slice(word.indexOf('=') + 1)] : [word, null]
    const given = value === null ? run.args[index + 1] : ([{ kind: 'text', text: value }] as Word)
    if (name === '-C' || name === '--work-tree') {
      const places = { ...run.places, cwd: dir }
      dir = given === undefined ? null : namedBy(given, places, run.vars).path
    }
    index += value === null && GIT_VALUED.has(name) ? 2 : 1
  }
  const sub = literal(run.args[index])
  return sub === null ? null : { sub, args: run.args.slice(index + 1), dir }
}

/** A git clean that deletes ignored files (`-x` or `-X`, with force) in the main checkout, where they are `.env` files and the store in `.rex/`. */
function cleanAlertsOf(run: Run): readonly Alert[] {
  const git = gitCommand(run)
  if (git?.sub !== 'clean') {
    return []
  }
  const { options } = splitArgs(git.args, new Set(['-e', '--exclude']))
  const isForced = options.includes('--force') || hasShort(options, 'f')
  const isIgnored = hasShort(options, 'x') || hasShort(options, 'X')
  const isDry = options.includes('--dry-run') || hasShort(options, 'n')
  if (!isForced || !isIgnored || isDry || git.dir === null || !isAtOrBelow(git.dir, run.places.repo)) {
    return []
  }
  return [alertOf(`a git clean ${options.join(' ')} in the main checkout ${run.places.repo}`, 'deletes every ignored file there, such as `.env` files and the store in `.rex/`, and no commit holds them')]
}

/** Every delete of the command the guard stops: a recursive rm, a find that deletes, a git clean of ignored files. */
export function deleteAlerts(command: string, places: RmPlaces): readonly Alert[] {
  return runsOf(command, places).flatMap(run => [
    ...rmAlertsOf(run).map(alert => ({ what: `a recursive rm of ${alert.target}`, reason: alert.reason, text: alertText(alert, run.places) })),
    ...findAlertsOf(run),
    ...cleanAlertsOf(run),
  ])
}

/** A cheap first look, so a command with no delete in it costs the guard nothing more. */
export function mayRemove(command: string): boolean {
  return /\b(rm|find|clean)\b/.test(command)
}

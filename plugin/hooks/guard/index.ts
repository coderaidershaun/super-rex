// Stops a shell command that could be a disaster that is hard to undo (a delete of the wrong folder, destructive SQL, a write over a disk, a cloud delete, a forced push to main) and tells the person.
// rex init turns no sandbox on, so this guard is what stands between an agent's wrong command and the person's files and data. An inert mod stops nothing.
import type { EngineInterface, HookFailure, On } from 'claude-code'

import { errorText, readBook } from '../book/index'
import type { GateCheck, Parts } from '../parts'
import { failureName, git } from '../tools/index'
import { statusOf } from './alert'
import type { Alert } from './alert'
import { disasterAlerts, mayBeDisaster } from './disaster'
import { deleteAlerts, mayRemove } from './rm'
import type { RmPlaces } from './shell'
import { mayDropData, sqlAlerts } from './sql'

export { alertOf, statusOf } from './alert'
export type { Alert } from './alert'
export { disasterAlerts, ROWS } from './disaster'
export { alertText, deleteAlerts, rmAlert } from './rm'
export type { RmAlert } from './rm'
export { clearable, commandsOf, simpleCommands } from './shell'
export type { RmPlaces } from './shell'
export { sqlAlerts } from './sql'

export const GUARD_CHECKS: readonly GateCheck[] = [
  { name: 'rm-guard', site: { kind: 'tool', tool: 'Bash' }, failure: 'closed' },
  { name: 'sql-guard', site: { kind: 'tool', tool: 'Bash' }, failure: 'closed' },
  { name: 'disaster-guard', site: { kind: 'tool', tool: 'Bash' }, failure: 'closed' },
]

/** Each check: its name, what it guards in the words of a failure, its cheap first look, and its alerts. */
const CHECKS: readonly { readonly name: string; readonly subject: string; readonly may: (command: string) => boolean; readonly alerts: (command: string, places: RmPlaces) => readonly Alert[] }[] = [
  { name: 'rm-guard', subject: 'a recursive rm', may: mayRemove, alerts: deleteAlerts },
  { name: 'sql-guard', subject: 'a database delete', may: mayDropData, alerts: sqlAlerts },
  { name: 'disaster-guard', subject: 'a disaster command', may: mayBeDisaster, alerts: disasterAlerts },
]

/** True when one of the checks may stop the command; else the guard costs it one test of each pattern and no book read. */
export function mayAlarm(command: string): boolean {
  return CHECKS.some(check => check.may(command))
}

/** The first alert of the first check that stops the command, or null when the command may run. */
export function guardAlert(command: string, places: RmPlaces): Alert | null {
  for (const check of CHECKS) {
    const alert = check.may(command) ? check.alerts(command, places)[0] : undefined
    if (alert !== undefined) {
      return alert
    }
  }
  return null
}

/** The temp folders of macOS and Linux, under each name the system gives them. */
const TEMP_FOLDERS: readonly string[] = ['/tmp', '/private/tmp', '/var/tmp', '/private/var/tmp', '/var/folders', '/private/var/folders']

function trimmed(path: string): string {
  return path.length > 1 ? path.replace(/\/+$/, '') : path
}

/** The main checkout of each session root this load of the mod has asked git about. */
const checkouts = new Map<string, string>()

/**
 * The main checkout of the session's repo: the folder that holds the Git
 * folder all its worktrees share, so a session that started in a job worktree
 * gets the same places as one in the main checkout. A folder git does not
 * know is its own main checkout.
 */
async function mainCheckout($: EngineInterface): Promise<string> {
  const root = await $.session.root()
  const known = checkouts.get(root)
  if (known !== undefined) {
    return known
  }
  const real = (await $.fs.stat(root, { resolve: true })).realPath ?? root
  const common = await git($, real, ['rev-parse', '--path-format=absolute', '--git-common-dir'])
  const repo = common.exitCode === 0 && common.stdout.endsWith('/.git') ? common.stdout.slice(0, -'/.git'.length) : real
  checkouts.set(root, repo)
  return repo
}

/** The places of one call. Read inside the hook, so `cwd` is the calling agent's own. */
export async function placesOf($: EngineInterface): Promise<RmPlaces> {
  const repo = trimmed(await mainCheckout($))
  const home = await $.env.get('HOME')
  const tmpdir = await $.env.get('TMPDIR')
  const above = repo.slice(0, repo.lastIndexOf('/'))
  // Agents may clear the folder above the repo only when it is a folder of projects: not the root, and not one that holds the home folder.
  const holdsHome = home === undefined || home === above || home.startsWith(`${above}/`)
  return {
    cwd: trimmed(await $.session.cwd()),
    home: home === undefined ? null : trimmed(home),
    repo,
    parent: above === '' || holdsHome ? null : above,
    temp: [...TEMP_FOLDERS, ...(tmpdir === undefined || tmpdir === '' ? [] : [trimmed(tmpdir)])],
  }
}

/**
 * The refusal of the first alert of the first check that stops the command,
 * with one status line for the person. A check's own error refuses the
 * command, so a broken guard never lets a disaster through unseen.
 */
async function guarded($: EngineInterface, command: string, agentId: string | undefined): Promise<string | null> {
  const checks = CHECKS.filter(check => check.may(command))
  if (checks.length === 0) {
    return null
  }
  let places: RmPlaces
  try {
    if ((await readBook($)).inert !== null) {
      return null
    }
    places = await placesOf($)
  } catch (error) {
    return `rex guard: the check of a command failed (${errorText(error)}); refused`
  }
  for (const check of checks) {
    let alert: Alert | undefined
    try {
      alert = check.alerts(command, places)[0]
    } catch (error) {
      return `rex guard: the check of ${check.subject} failed (${errorText(error)}); refused`
    }
    if (alert !== undefined) {
      $.ui.status(statusOf(alert, agentId ?? 'the main session'))
      return alert.text
    }
  }
  return null
}

function failed(error: HookFailure): { readonly deny: string } {
  return { deny: `rex guard: the guard failed (${failureName(error)}); refused` }
}

export function register(on: On, parts: Parts): void {
  for (const check of GUARD_CHECKS) {
    parts.gateChecks.add(check)
  }

  on('tool.call', { tool: 'Bash' }, async ($, e, next) => {
    const refusal = await guarded($, e.command, e.agentId)
    return refusal === null ? next(e) : { deny: refusal }
  }).catch((_$, e, next) => (next.called ? next(e) : failed(next.error)))

  // Monitor runs a shell command too.
  on('tool.call', { tool: 'Monitor' }, async ($, e, next) => {
    const refusal = e.command === undefined ? null : await guarded($, e.command, e.agentId)
    return refusal === null ? next(e) : { deny: refusal }
  }).catch((_$, e, next) => (next.called ? next(e) : failed(next.error)))
}

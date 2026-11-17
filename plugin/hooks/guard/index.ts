// Stops a recursive rm that would delete the root, the home folder, the repo itself, or anything outside the folder that holds the repo, and tells the person.
// rex init turns no sandbox on, so this guard is what stands between an agent's wrong path and the person's files. An inert mod stops nothing.
import type { EngineInterface, HookFailure, On } from 'claude-code'

import { errorText, readBook } from '../book/index'
import type { GateCheck, Parts } from '../parts'
import { failureName, git } from '../tools/index'
import { alertText, mayRemove, rmAlert } from './rm'
import type { RmPlaces } from './rm'

export { alertText, clearable, commandsOf, rmAlert } from './rm'
export type { RmAlert, RmPlaces } from './rm'

export const GUARD_CHECKS: readonly GateCheck[] = [{ name: 'rm-guard', site: { kind: 'tool', tool: 'Bash' }, failure: 'closed' }]

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

async function guardRefusal($: EngineInterface, command: string, agentId: string | undefined): Promise<string | null> {
  if (!mayRemove(command) || (await readBook($)).inert !== null) {
    return null
  }
  const places = await placesOf($)
  const alert = rmAlert(command, places)
  if (alert === null) {
    return null
  }
  $.ui.status(`rex: ALERT: stopped a recursive rm of ${alert.target} (${alert.reason}) by ${agentId ?? 'the main session'}; run it yourself if you mean it`)
  return alertText(alert, places)
}

/** The guard's own error refuses the command, so a broken guard never lets a delete through unseen. */
async function guarded($: EngineInterface, command: string, agentId: string | undefined): Promise<string | null> {
  try {
    return await guardRefusal($, command, agentId)
  } catch (error) {
    return `rex guard: the check of a recursive rm failed (${errorText(error)}); refused`
  }
}

function failed(error: HookFailure): { readonly deny: string } {
  return { deny: `rex guard: rm-guard failed (${failureName(error)}); refused` }
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

// /rex master: the person's own typing starts a master after every pre-flight check passes, with the charter stored as the person wrote it.
import type { CommandRunInput, CommandRunResult, EngineInterface } from 'claude-code'

import { errorText, fail, openTree, pluginVersion, readBook } from '../book/index'
import { originRefusal, rexLine } from '../commands/index'
import type { StatusSection, Table } from '../parts'
import { hashText } from '../skills/index'
import type { MasterTypeState } from './agent-type'
import { MASTER_SUBCOMMAND } from './entries'
import { treeIdOf } from './ids'
import { DEFAULT_POLICY, PolicyShapeError } from './policy'
import type { Policy } from './policy'
import { serveResume } from './resume'
import { spawnMaster } from './start'
import { startWatchdog } from './watch'
import { readPolicy, realRoot, writePolicy } from './store'
import type { StoredPolicy } from './store'

/** What the entry needs from the module: whether the type registered, and whether a person is at the terminal. */
export type EntryState = { readonly masterType: MasterTypeState; readonly isInteractive: boolean }

/** The policy as the entry reads it; an unreadable one is a refusal, never the default. */
type PolicyRead = { readonly kind: 'read'; readonly stored: StoredPolicy | null } | { readonly kind: 'refused'; readonly text: string }

async function policyRead($: EngineInterface): Promise<PolicyRead> {
  try {
    return { kind: 'read', stored: await readPolicy($) }
  } catch (error) {
    if (error instanceof PolicyShapeError) {
      return { kind: 'refused', text: `rex master: the stored policy of this repo is unreadable, got ${error.message}; run /rex policy set ${error.key} <value>` }
    }
    throw error
  }
}

const MIN_DEPTH = 4

/** Pre-flight 1 to 4: storage, binary version, depth, and the headless wait ceiling; null when each passes. */
async function preflightRefusal($: EngineInterface, state: EntryState): Promise<{ readonly step: number; readonly text: string } | null> {
  const book = await readBook($)
  if (book.storage !== 'on') {
    return { step: 1, text: 'rex master: needs storage; run rex init --with-storage' }
  }
  const ran = await $.process.run(['rex', '--version'], { cwd: await $.session.root() })
  const binary = ran.exitCode === 0 ? (/^rex (\S+)\n?$/.exec(ran.stdout)?.[1] ?? ran.stdout.trim()) : `unknown (exit ${String(ran.exitCode)})`
  const plugin = await pluginVersion($)
  if (binary !== plugin) {
    return { step: 2, text: `rex master: binary ${binary} does not match plugin ${plugin}; run rex doctor` }
  }
  const depthText = await $.env.get('CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH')
  const depth = depthText === undefined ? Number.NaN : Number(depthText)
  if (!Number.isInteger(depth) || depth < MIN_DEPTH) {
    return { step: 3, text: `rex master: depth is ${depthText ?? 'unset'}; run rex init (it writes 4)` }
  }
  if (!state.isInteractive && (await $.env.get('CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS')) !== '0') {
    return { step: 4, text: 'rex master: headless runs need CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0, or background agents stop after 10 minutes' }
  }
  return null
}

const MERGE_QUESTION = 'How may a master of this repo merge a job?'
const PARALLEL_QUESTION = 'How many builders may run at once?'

/** The person's first policy: two questions, then the policy stored and confirmed; null when a question was dismissed. */
async function askFirstPolicy($: EngineInterface): Promise<StoredPolicy | null> {
  try {
    const merge = await $.ui.ask(MERGE_QUESTION, ['never', 'ask', 'allowed_if_checks_pass'])
    const parallel = await $.ui.ask(PARALLEL_QUESTION, ['1', '2', '3', 'auto'])
    const policy: Policy = { ...DEFAULT_POLICY, merge: merge === 'never' || merge === 'allowed_if_checks_pass' ? merge : 'ask', max_parallel_builders: parallel === 'auto' ? 'auto' : Number(parallel) }
    return await writePolicy($, policy, 'confirm')
  } catch (error) {
    if (error instanceof Error) {
      return null
    }
    throw error
  }
}

/** Pre-flight 6: a confirmed policy, asked for on the first interactive run. */
async function confirmedPolicy($: EngineInterface, state: EntryState, stored: StoredPolicy | null): Promise<StoredPolicy | string> {
  if (stored === null) {
    if (!state.isInteractive) {
      return 'rex master: no confirmed policy for this repo; run /rex policy confirm in an interactive session'
    }
    return (await askFirstPolicy($)) ?? 'rex master: no confirmed policy for this repo; answer the two policy questions, or run /rex policy confirm'
  }
  return stored.isConfirmed ? stored : `rex master: the policy of this repo changed since you confirmed it, got hash ${stored.hash}; run /rex policy confirm`
}

type Charter = { readonly text: string } | { readonly refused: string }

async function charterOf($: EngineInterface, args: string): Promise<Charter> {
  if (!args.startsWith('@')) {
    return { text: args }
  }
  const named = args.slice(1).trim()
  const path = named.startsWith('/') ? named : `${await realRoot($)}/${named}`
  try {
    return { text: await $.fs.read(path) }
  } catch (error) {
    return { refused: `rex master: charter file ${path} could not be read, got ${errorText(error)}; name a file in the repo` }
  }
}

function lines(rows: readonly string[]): CommandRunResult {
  return { text: rows.map(row => `${row}\n`).join('') }
}

async function refuse($: EngineInterface, step: number, text: string): Promise<CommandRunResult> {
  await fail($, { kind: 'preflight-failed', step, text })
  return { text }
}

/** Every refusal before the tree opens leaves nothing started. */
export async function serveMaster($: EngineInterface, event: CommandRunInput, state: EntryState, sections: Table<StatusSection>): Promise<CommandRunResult> {
  const read = await policyRead($)
  if (read.kind === 'refused') {
    return refuse($, 6, read.text)
  }
  const policy = read.stored?.policy ?? DEFAULT_POLICY
  const originText = originRefusal(event, MASTER_SUBCOMMAND, event.origin.kind === 'sdk' && policy.allow_headless)
  if (originText !== null) {
    await fail($, { kind: 'origin-not-person', action: 'rex master', origin: event.origin.kind })
    return { text: originText }
  }
  const args = rexLine(event).args
  if (args === '') {
    return { text: 'rex master: name your goal, a charter file as @<path>, or resume, got nothing; run /rex master <goal>' }
  }
  const preflight = await preflightRefusal($, state)
  if (preflight !== null) {
    return refuse($, preflight.step, preflight.text)
  }
  if (state.masterType.kind === 'unregistered') {
    return refuse($, 0, `rex master: the master type is not registered, got ${state.masterType.reason}; run /rex doctor`)
  }
  const confirmed = await confirmedPolicy($, state, read.stored)
  if (typeof confirmed === 'string') {
    await fail($, { kind: 'policy-missing', repo: await $.session.root() })
    return { text: confirmed }
  }
  if (args === 'resume') {
    return serveResume($, confirmed, sections)
  }
  const charter = await charterOf($, args)
  if ('refused' in charter) {
    return { text: charter.refused }
  }
  const tree = treeIdOf(await $.clock.now())
  await openTree($, { id: tree, charter: charter.text, charterHash: await hashText(charter.text), policyHash: confirmed.hash, state: 'running', capNano: Math.round(confirmed.policy.tree_usd * 1e9) })
  const started = await spawnMaster($, { tree, charter: charter.text, digest: null })
  if ('refused' in started) {
    return { text: started.refused }
  }
  startWatchdog($, started.tree, confirmed.policy.watchdog_minutes, sections)
  return lines([`tree=${started.tree}`, `master=${started.master}`])
}

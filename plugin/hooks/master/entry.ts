// rex_tree_open: the main loop opens a master tree right after the person's own prompt of the master skill, once every pre-flight check passes, with the charter taken from that prompt as the person wrote it; storage is needed only to resume.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexTree, RexTreeStartPrompt } from '../../types'
import { dropTreeStart, errorText, fail, mainTree, openTree, pluginVersion, readBook } from '../book/index'
import type { StatusSection, Table } from '../parts'
import { hashText } from '../skills/index'
import { callerOf, deny, linesText } from '../tools/index'
import { TREE_OPEN_TOOL } from './entries'
import { treeIdOf } from './ids'
import { DEFAULT_POLICY, PolicyShapeError } from './policy'
import type { MergeRule, Policy } from './policy'
import { resumeTree } from './resume'
import { briefOf } from './start'
import { startWatchdog } from './watch'
import { readPolicy, realRoot, writePolicy } from './store'
import type { StoredPolicy } from './store'

const TOOL = TREE_OPEN_TOOL.name
const SKILL = '/super-rex:orchestration-master'

/** What the tool needs from the module: whether a person is at the terminal. */
export type EntryState = { readonly isInteractive: boolean }

/** The policy as the entry reads it; an unreadable one is a refusal, never the default. */
type PolicyRead = { readonly kind: 'read'; readonly stored: StoredPolicy | null } | { readonly kind: 'refused'; readonly text: string }

async function policyRead($: EngineInterface): Promise<PolicyRead> {
  try {
    return { kind: 'read', stored: await readPolicy($) }
  } catch (error) {
    if (error instanceof PolicyShapeError) {
      return { kind: 'refused', text: `${TOOL}: the stored policy of this repo is unreadable, got ${error.message}; the person runs /rex policy set ${error.key} <value>` }
    }
    throw error
  }
}

const MIN_DEPTH = 4

/** Pre-flight 2 to 4: binary version, depth, and the headless wait ceiling; null when each passes. */
async function preflightRefusal($: EngineInterface, state: EntryState): Promise<{ readonly step: number; readonly text: string } | null> {
  const ran = await $.process.run(['rex', '--version'], { cwd: await $.session.root() })
  const binary = ran.exitCode === 0 ? (/^rex (\S+)\n?$/.exec(ran.stdout)?.[1] ?? ran.stdout.trim()) : `unknown (exit ${String(ran.exitCode)})`
  const plugin = await pluginVersion($)
  if (binary !== plugin) {
    return { step: 2, text: `${TOOL}: binary ${binary} does not match plugin ${plugin}; the person runs rex doctor` }
  }
  const depthText = await $.env.get('CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH')
  const depth = depthText === undefined ? Number.NaN : Number(depthText)
  if (!Number.isInteger(depth) || depth < MIN_DEPTH) {
    return { step: 3, text: `${TOOL}: depth is ${depthText ?? 'unset'}; the person runs rex init (it writes 4), then starts a new session` }
  }
  if (!state.isInteractive && (await $.env.get('CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS')) !== '0') {
    return { step: 4, text: `${TOOL}: headless runs need CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0, or background agents stop after 10 minutes` }
  }
  return null
}

const MERGE_QUESTION = 'Merge each job of a master to its base branch when its checks pass?'
/** Each option of the first question, in the person's words, and the merge rule it stores. */
const MERGE_OPTIONS: readonly { readonly text: string; readonly rule: MergeRule }[] = [
  { text: 'Yes, merge', rule: 'allowed_if_checks_pass' },
  { text: 'No, keep the branches', rule: 'never' },
  { text: 'Ask me for each job', rule: 'ask' },
]

/** The person's first policy: one question, then the policy stored and confirmed; null when the question was dismissed. */
async function askFirstPolicy($: EngineInterface): Promise<StoredPolicy | null> {
  try {
    const chosen = await $.ui.ask(MERGE_QUESTION, MERGE_OPTIONS.map(option => option.text))
    const merge = MERGE_OPTIONS.find(option => option.text === chosen)?.rule ?? 'ask'
    const policy: Policy = { ...DEFAULT_POLICY, merge }
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
      return `${TOOL}: no confirmed policy for this repo; the person runs /rex policy confirm in an interactive session`
    }
    return (await askFirstPolicy($)) ?? `${TOOL}: no confirmed policy for this repo; the person answers the policy question, or runs /rex policy confirm, then ${SKILL} again`
  }
  return stored.isConfirmed ? stored : `${TOOL}: the policy of this repo changed since the person confirmed it, got hash ${stored.hash}; the person runs /rex policy confirm`
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
    return { refused: `${TOOL}: charter file ${path} could not be read, got ${errorText(error)}; the person names a file in the repo` }
  }
}

async function refuse($: EngineInterface, step: number, text: string): Promise<ToolCallResult> {
  await fail($, { kind: 'preflight-failed', step, text })
  return deny(text)
}

/** The answer of the tool: its `key=value` lines, then the brief's blocks. `person` says whether someone is at the terminal to answer a question. */
function opened(tree: RexTree, policy: Policy, state: EntryState, rows: readonly string[], brief: string): ToolCallResult {
  const lines = [`tree=${tree.id}`, `state=${tree.state}`, ...rows, `merge=${policy.merge}`, `person=${state.isInteractive ? 'yes' : 'no'}`]
  return { result: `${linesText(lines)}\n${brief}` }
}

/** The tree this session's main loop already runs: its brief again, so a master that lost its context reads its charter. */
async function existing($: EngineInterface, tree: RexTree, start: RexTreeStartPrompt | null, read: PolicyRead & { readonly kind: 'read' }, state: EntryState): Promise<ToolCallResult> {
  const isNewCharter = start !== null && start.text !== '' && start.text !== 'resume'
  if (isNewCharter) {
    await dropTreeStart($)
    return deny(`${TOOL}: master tree ${tree.id} of this session is still ${tree.state}, got a new charter; end it with rex_report first, then the person runs ${SKILL} <goal> again`)
  }
  const policy = read.stored?.policy ?? DEFAULT_POLICY
  return opened(tree, policy, state, ['existing=yes', 'resumed=no'], await briefOf($, { charter: tree.charter, digest: null, policy }))
}

/** Every refusal before the tree opens leaves nothing started. */
export async function serveTreeOpen($: EngineInterface, event: { readonly agentId?: string }, state: EntryState, sections: Table<StatusSection>): Promise<ToolCallResult> {
  const caller = await callerOf($, event.agentId)
  const book = await readBook($)
  if (caller.agentId !== null || caller.kind === 'agent') {
    await fail($, { kind: 'origin-not-person', action: TOOL, origin: caller.agentId ?? caller.kind })
    return deny(`${TOOL}: only the main loop opens a master tree, right after the person's own ${SKILL} prompt, got ${caller.kind}`)
  }
  if (caller.kind === 'builder') {
    return deny(`${TOOL}: the main loop builds job ${caller.job?.id ?? ''}, got an open job; close it with rex_job_close first, then the person runs ${SKILL} again`)
  }
  const read = await policyRead($)
  if (read.kind === 'refused') {
    return refuse($, 6, read.text)
  }
  const start = book.treeStartPrompt
  const live = mainTree(book)
  if (live !== undefined) {
    return existing($, live, start, read, state)
  }
  const policy = read.stored?.policy ?? DEFAULT_POLICY
  if (start === null || (start.origin === 'sdk' && !policy.allow_headless)) {
    await fail($, { kind: 'origin-not-person', action: TOOL, origin: start === null ? 'no prompt of the master skill' : start.origin })
    const why = start === null ? `got no ${SKILL} prompt of the person` : 'got a headless prompt while allow_headless of the policy is false'
    return deny(`${TOOL}: only the person's own ${SKILL} prompt opens a master tree, ${why}; the person runs ${SKILL} <goal>`)
  }
  if (start.text === '') {
    return deny(`${TOOL}: the prompt names no goal, got nothing after ${SKILL}; the person runs ${SKILL} <goal>, ${SKILL} @<charter file>, or ${SKILL} resume`)
  }
  const preflight = await preflightRefusal($, state)
  if (preflight !== null) {
    return refuse($, preflight.step, preflight.text)
  }
  const confirmed = await confirmedPolicy($, state, read.stored)
  if (typeof confirmed === 'string') {
    await fail($, { kind: 'policy-missing', repo: await $.session.root() })
    return deny(confirmed)
  }
  if (start.text === 'resume') {
    if (book.storage !== 'on') {
      return deny(`${TOOL}: resume needs storage, got storage off; the person runs rex init --with-storage, or starts a new tree with ${SKILL} <goal>`)
    }
    const resumed = await resumeTree($, confirmed, sections)
    if (typeof resumed === 'string') {
      return deny(resumed)
    }
    return opened(resumed.tree, confirmed.policy, state, ['existing=no', 'resumed=yes', ...resumed.lines], await briefOf($, { charter: resumed.tree.charter, digest: resumed.digest, policy: confirmed.policy }))
  }
  const charter = await charterOf($, start.text)
  if ('refused' in charter) {
    return deny(charter.refused)
  }
  const id = treeIdOf(await $.clock.now())
  await openTree($, { id, charter: charter.text, charterHash: await hashText(charter.text), policyHash: confirmed.hash, state: 'running', capNano: Math.round(confirmed.policy.tree_usd * 1e9) })
  startWatchdog($, id, confirmed.policy.watchdog_minutes, sections)
  const tree = (await readBook($)).trees.find(each => each.id === id)
  if (tree === undefined) {
    return deny(`${TOOL}: tree ${id} did not open; call ${TOOL} again`)
  }
  return opened(tree, confirmed.policy, state, ['existing=no', 'resumed=no'], await briefOf($, { charter: charter.text, digest: null, policy: confirmed.policy }))
}

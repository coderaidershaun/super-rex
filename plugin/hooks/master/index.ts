// The master: its agent type, its entry command, its policy, the builders it starts, its questions to the person, its wakes, its end, and its recovery.
import type { EngineInterface, On } from 'claude-code'

import { errorText, fail, readBook } from '../book/index'
import type { Parts } from '../parts'
import { toolFailed } from '../tools/index'
import { MASTER_TYPE, registerMasterType } from './agent-type'
import type { MasterTypeState } from './agent-type'
import { serveBuild } from './build'
import { CHARTER_PART } from './brief'
import { setAsking } from './ask'
import { serveAnswer, servePolicy } from './commands'
import { ASK_OWNER_TOOL, BUILD_TOOL_NAME, CONTINUE_TOOL, GATE_CHECKS, MERGE_TOOL_NAME, NOTE_TOOL, REPORT_TOOL, SUBCOMMANDS, TOOLS } from './entries'
import { serveMaster } from './entry'
import { serveMergePolicy } from './merge-policy'
import { notificationAnswer } from './notify'
import { liveTrees } from './pin'
import { DEFAULT_POLICY } from './policy'
import { masterStopped, serveReport } from './report'
import { MASTER_ONLY, sendRefusal } from './send'
import { masterSpawn, ownSpawn } from './spawn'
import { STATUS_SECTIONS, withPolicyLines } from './status'
import { readPolicy } from './store'
import { serveAskOwner, serveContinue, serveNote } from './tools'
import { isWakeOnTheWay } from './wake'
import { builderEnded, startWatchdog } from './watch'

const OWN_SPAWN = /^rex (master|builder) /

/** A policy that cannot be read here still leaves the tree a watchdog, on the default period; the line tells the person why. */
async function watchdogMinutes($: EngineInterface): Promise<number> {
  try {
    return ((await readPolicy($))?.policy ?? DEFAULT_POLICY).watchdog_minutes
  } catch (error) {
    $.ui.status(`rex: the watchdog runs every ${String(DEFAULT_POLICY.watchdog_minutes)} minutes, as the policy could not be read: ${errorText(error)}`)
    return DEFAULT_POLICY.watchdog_minutes
  }
}

/** Set again at each session start, which a reload also runs. */
const session: { masterType: MasterTypeState; isInteractive: boolean } = {
  masterType: { kind: 'unregistered', reason: 'the session has not started' },
  isInteractive: true,
}

export function register(on: On, parts: Parts): void {
  for (const entry of SUBCOMMANDS) {
    parts.subcommands.add(entry)
  }
  for (const entry of TOOLS) {
    parts.tools.add(entry)
  }
  for (const check of GATE_CHECKS) {
    parts.gateChecks.add(check)
  }
  for (const section of STATUS_SECTIONS) {
    parts.statusSections.add(section)
  }
  parts.briefParts.add(CHARTER_PART)

  on('session.start', { cwd: /(?:)/ }, async ($, e, next) => {
    session.isInteractive = e.isInteractive
    setAsking(e.isInteractive, parts.statusSections)
    const book = await readBook($)
    session.masterType = book.inert === null ? await registerMasterType($) : { kind: 'unregistered', reason: `the mod is off: ${book.inert}` }
    for (const tree of liveTrees(book)) {
      startWatchdog($, tree.id, await watchdogMinutes($), parts.statusSections)
    }
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'session.start', module: 'master', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  // The engine asks the offer also when it resumes a master, so a wake of the mod's own must be offered, or no master runs after its first turn.
  on('agent.offer', { agent: MASTER_TYPE }, async $ => ({ isOffered: liveTrees(await readBook($)).some(tree => tree.master !== null && isWakeOnTheWay(tree.master)) })).catch(() => ({
    isOffered: false,
  }))
  on('agent.spawn', { subagentType: MASTER_TYPE }, ($, e, next) => masterSpawn($, e, next.origin.plugin, next)).catch(async ($, e, next) => {
    await fail($, { kind: 'authority-check-failed', check: 'master-type-spawn', error: next.error.message ?? next.error.kind })
    return next.called ? next(e) : { deny: 'rex master: the master spawn check failed; refused' }
  })

  // Matched on the fixed descriptions of the mod's own spawns, which every spawn event carries.
  on('agent.spawn', { description: OWN_SPAWN }, ($, e, next) => ownSpawn($, e, next.origin.plugin, session.isInteractive, next)).catch(async ($, e, next) => {
    await fail($, { kind: 'authority-check-failed', check: 'master-spawn', error: next.error.message ?? next.error.kind })
    return next.called ? next(e) : { deny: 'rex master: the spawn check failed; refused' }
  })

  on('command.run', { command: 'rex', args: /^master(\s|$)/ }, ($, e) => serveMaster($, e, session, parts.statusSections)).catch((_$, _e, next) => ({
    text: `rex master: failed (${next.error.message ?? next.error.kind}); if a tree opened, run /rex status`,
  }))

  on('tool.call', { tool: BUILD_TOOL_NAME }, ($, e) => serveBuild($, e)).catch(($, _e, next) => toolFailed($, 'rex_build', next.error))
  on('tool.call', { tool: MERGE_TOOL_NAME }, ($, e, next) => serveMergePolicy($, e, next)).catch(async ($, e, next) => {
    await fail($, { kind: 'authority-check-failed', check: 'policy-merge', error: next.error.message ?? next.error.kind })
    return next.called ? next(e) : { deny: 'rex gate: policy-merge failed (timeout); refused' }
  })

  on('command.run', { command: 'rex', args: /^policy(\s|$)/ }, ($, e) => servePolicy($, e, parts.statusSections)).catch((_$, _e, next) => ({
    text: `rex policy: failed (${next.error.message ?? next.error.kind}); run /rex policy show to see what was stored`,
  }))
  on('command.run', { command: 'rex', args: /^answer(\s|$)/ }, ($, e) => serveAnswer($, e)).catch((_$, _e, next) => ({
    text: `rex answer: failed (${next.error.message ?? next.error.kind}); run /rex status to see whether it was recorded`,
  }))

  on('tool.call', { tool: 'mcp__super-rex__rex_continue' }, ($, e) => serveContinue($, e, CONTINUE_TOOL)).catch(($, _e, next) => toolFailed($, 'rex_continue', next.error))
  on('tool.call', { tool: 'mcp__super-rex__rex_note' }, ($, e) => serveNote($, e, NOTE_TOOL)).catch(($, _e, next) => toolFailed($, 'rex_note', next.error))
  on('tool.call', { tool: 'mcp__super-rex__rex_ask_owner' }, ($, e) => serveAskOwner($, e, ASK_OWNER_TOOL)).catch(($, _e, next) => toolFailed($, 'rex_ask_owner', next.error))
  on('tool.call', { tool: 'mcp__super-rex__rex_status' }, ($, e, next) => withPolicyLines($, e, next)).catch(($, _e, next) => toolFailed($, 'rex_status', next.error))
  on('tool.call', { tool: 'mcp__super-rex__rex_report' }, ($, e) => serveReport($, e, REPORT_TOOL)).catch(($, _e, next) => toolFailed($, 'rex_report', next.error))

  on('session.send', { to: /(?:)/ }, async ($, e, next) => (await sendRefusal($, e)) ?? next(e)).catch(() => ({ isDelivered: false, reason: MASTER_ONLY }))

  on('turn.complete', { reason: /(?:)/ }, async ($, e, next) => {
    const result = await next(e)
    if (e.agentId !== undefined) {
      await builderEnded($, { agentId: e.agentId, reason: e.reason, answer: e.answer }, parts.statusSections)
      await masterStopped($, e.agentId, e.answer)
    }
    return result
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'turn.complete', module: 'master', agentId: e.agentId ?? null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('prompt.submit', { origin: { kind: 'task-notification' } }, async ($, e, next) => {
    const answered = notificationAnswer(await readBook($), e)
    if (answered === null) {
      return next(e)
    }
    return 'drop' in answered ? answered : next({ ...e, text: answered.text })
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'prompt.submit', module: 'master', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })
}

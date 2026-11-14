// rex_job_plan, and the call to rex oms that gives each role its grid agent.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexDecision, RexJob, RexOpenWhen, RexRating, RexRatings, RexRole, RexSkillFile, RexTask } from '../../types'
import { amendPlan, fail, planJob, readBook, recordDecision } from '../book/index'
import type { ToolEntry } from '../parts'
import { hashText, resolveSkill, skillPlaces } from '../skills/index'
import { argsOf, field, isRefused, objectList, optionalText, optionalTextList, textArg, word } from './args'
import type { Args, Refused } from './args'
import { jsonField, readBinary } from './binary'
import { callerIn, callerRefusal, jobArgRefusal } from './caller'
import { answer, deny } from './text'

export type OmsInput = { readonly prefs: RexJob['prefs']; readonly ratings: RexRatings }
export type OmsAnswer = { readonly ok: true; readonly agent: string; readonly conflict: readonly string[]; readonly version: number } | { readonly ok: false; readonly line: string }

export function omsArgv(input: OmsInput): readonly string[] {
  const { prefs, ratings } = input
  return [
    'rex', 'oms', '--json',
    '--duration', prefs.duration ?? 'none',
    '--budget', prefs.budget ?? 'none',
    '--autonomy', prefs.autonomy ?? 'none',
    '--intelligence', ratings.intelligence,
    '--reliability', ratings.reliability,
    '--consequence', ratings.consequence,
    '--verification', ratings.verification,
  ]
}

/** Reads the one JSON line of `rex oms --json`; anything that is not its shape is a refusal with the text seen. */
export function omsAnswerOf(stdout: string): OmsAnswer {
  let parsed: unknown
  try {
    parsed = JSON.parse(stdout)
  } catch (error) {
    return { ok: false, line: `rex oms printed no JSON, got ${stdout.slice(0, 80)} (${error instanceof Error ? error.name : 'Error'})` }
  }
  const agent = jsonField(parsed, 'agent')
  const conflict = jsonField(parsed, 'conflict')
  const version = jsonField(parsed, 'version')
  const isConflictList = Array.isArray(conflict) && conflict.every((item: unknown) => typeof item === 'string')
  if (typeof agent !== 'string' || !isConflictList || typeof version !== 'number') {
    return { ok: false, line: `rex oms answered no agent, conflict and version, got ${stdout.slice(0, 80)}` }
  }
  return { ok: true, agent, conflict: conflict.map(String), version }
}

export async function runOms($: EngineInterface, input: OmsInput): Promise<OmsAnswer> {
  const read = await readBinary($, omsArgv(input))
  return read.ok ? omsAnswerOf(read.stdout) : { ok: false, line: read.line }
}

const TOOL = 'rex_job_plan'
const RATINGS: readonly RexRating[] = ['low', 'medium', 'high', 'none']
/** The sources of decisions that rex_assign records; a plan's own arguments are source `plan`. */
const ASSIGNED: readonly RexDecision['source'][] = ['rule', 'decision-agent', 'override']

type PlanArgs = {
  readonly orchestrator: string
  readonly breakdown: string
  readonly tasks: readonly RexTask[]
  readonly roles: readonly { readonly role: string; readonly ratings: RexRatings }[]
  readonly overrides: readonly { readonly subject: string; readonly item: string; readonly value: string; readonly reason: string }[]
  readonly isAmend: boolean
}

function taskOf(item: Args): RexTask | Refused {
  const todo = 'give each task id, role, skills and open_when'
  const id = textArg(TOOL, item, 'id', todo)
  const role = textArg(TOOL, item, 'role', todo)
  const skills = optionalTextList(TOOL, item, 'skills')
  const opens = field(item, 'open_when') === undefined ? [] : objectList(TOOL, item, 'open_when', 'give each open_when skill and when')
  if (isRefused(id)) return id
  if (isRefused(role)) return role
  if (isRefused(skills)) return skills
  if (isRefused(opens)) return opens
  const openWhen: RexOpenWhen[] = []
  for (const open of opens) {
    const skill = textArg(TOOL, open, 'skill', 'name the skill to open')
    const when = textArg(TOOL, open, 'when', 'say in which case the agent opens it')
    const file = optionalText(TOOL, open, 'file', 'name a .md file inside the skill')
    if (isRefused(skill)) return skill
    if (isRefused(when)) return when
    if (isRefused(file)) return file
    openWhen.push({ skill, file, when })
  }
  return { id, role, skills, openWhen }
}

function roleOf(item: Args): { readonly role: string; readonly ratings: RexRatings } | Refused {
  const role = textArg(TOOL, item, 'role', 'give each role its name and four ratings')
  const intelligence = word(TOOL, item, 'intelligence', RATINGS)
  const reliability = word(TOOL, item, 'reliability', RATINGS)
  const consequence = word(TOOL, item, 'consequence', RATINGS)
  const verification = word(TOOL, item, 'verification', RATINGS)
  if (isRefused(role)) return role
  if (isRefused(intelligence)) return intelligence
  if (isRefused(reliability)) return reliability
  if (isRefused(consequence)) return consequence
  if (isRefused(verification)) return verification
  return { role, ratings: { intelligence, reliability, consequence, verification } }
}

function planArgs(args: Args): PlanArgs | Refused {
  const orchestrator = textArg(TOOL, args, 'orchestrator', 'name the orchestrator and its version')
  const breakdown = textArg(TOOL, args, 'breakdown', 'give the work breakdown as text')
  const taskItems = objectList(TOOL, args, 'tasks', 'give a list of tasks')
  const roleItems = objectList(TOOL, args, 'roles', 'give a list of roles')
  const overrideItems = field(args, 'overrides') === undefined ? [] : objectList(TOOL, args, 'overrides', 'give each override subject, item, value and reason')
  if (isRefused(orchestrator)) return orchestrator
  if (isRefused(breakdown)) return breakdown
  if (isRefused(taskItems)) return taskItems
  if (isRefused(roleItems)) return roleItems
  if (isRefused(overrideItems)) return overrideItems
  const tasks: RexTask[] = []
  for (const item of taskItems) {
    const task = taskOf(item)
    if (isRefused(task)) return task
    tasks.push(task)
  }
  const roles: { readonly role: string; readonly ratings: RexRatings }[] = []
  for (const item of roleItems) {
    const role = roleOf(item)
    if (isRefused(role)) return role
    roles.push(role)
  }
  const overrides: PlanArgs['overrides'][number][] = []
  for (const item of overrideItems) {
    const todo = 'give each override subject, item, value and reason'
    const subject = textArg(TOOL, item, 'subject', todo)
    const name = textArg(TOOL, item, 'item', todo)
    const value = textArg(TOOL, item, 'value', todo)
    const reason = textArg(TOOL, item, 'reason', todo)
    if (isRefused(subject)) return subject
    if (isRefused(name)) return name
    if (isRefused(value)) return value
    if (isRefused(reason)) return reason
    overrides.push({ subject, item: name, value, reason })
  }
  return { orchestrator, breakdown, tasks, roles, overrides, isAmend: field(args, 'amend') === true }
}

/**
 * A task's skills from what rex_assign recorded for it: items `skill` (a
 * read-first skill), `open_when.<skill>` and `file.<skill>/<file>` (a file
 * inside a skill), each with its case; null when it recorded none.
 */
export function assignedSkills(decisions: readonly RexDecision[], task: RexTask): RexTask | null {
  const own = decisions.filter(decision => decision.subject === `task:${task.id}` && ASSIGNED.includes(decision.source))
  if (own.length === 0) {
    return null
  }
  const skills = own.filter(decision => decision.item === 'skill').map(decision => decision.value)
  const openWhen = own.flatMap((decision): RexOpenWhen[] => {
    if (decision.item.startsWith('open_when.')) {
      return [{ skill: decision.item.slice('open_when.'.length), file: null, when: decision.value }]
    }
    const path = decision.item.startsWith('file.') ? decision.item.slice('file.'.length) : ''
    const slash = path.indexOf('/')
    return slash > 0 ? [{ skill: path.slice(0, slash), file: path.slice(slash + 1), when: decision.value }] : []
  })
  return { ...task, skills, openWhen }
}

/** Every skill file a plan names: each read-first SKILL.md and each open-when file, once. */
export function planFiles(tasks: readonly RexTask[]): readonly { readonly skill: string; readonly file: string }[] {
  const named = tasks.flatMap(task => [
    ...task.skills.map(skill => ({ skill, file: 'SKILL.md' })),
    ...task.openWhen.map(open => ({ skill: open.skill, file: open.file ?? 'SKILL.md' })),
  ])
  return named.filter((one, index) => named.findIndex(other => other.skill === one.skill && other.file === one.file) === index)
}

/** Two read-first overrides of one task differ by their skill, so the value is part of the id of item `skill`. */
async function overrideId(job: string, override: PlanArgs['overrides'][number]): Promise<string> {
  return hashText(`${job}|${override.subject}|${override.item}|${override.item === 'skill' ? override.value : ''}|override`)
}

export async function serveJobPlan($: EngineInterface, event: { readonly agentId?: string }, entry: ToolEntry): Promise<ToolCallResult> {
  const book = await readBook($)
  const caller = callerIn(book, event.agentId)
  const callerText = callerRefusal(entry, caller)
  const job = caller.job
  if (callerText !== null || job === null) {
    await fail($, { kind: 'caller-not-allowed', tool: TOOL, caller: caller.kind })
    return deny(callerText ?? `${TOOL}: only a builder may call it, got ${caller.kind}`)
  }
  const args = argsOf(event)
  const jobText = jobArgRefusal(TOOL, field(args, 'job'), job)
  const parsed = planArgs(args)
  const refusal = jobText ?? (isRefused(parsed) ? parsed.refused : null)
  if (refusal !== null || isRefused(parsed)) {
    await fail($, { kind: 'bad-argument', tool: TOOL, argument: 'arguments', value: refusal ?? '' })
    return deny(refusal ?? '')
  }
  if (job.plan !== null && !parsed.isAmend) {
    return deny(`${TOOL}: job ${job.id} has a plan already, got a second plan; call it with amend: true to replace the rows it names`)
  }
  const roleNames = new Set([...parsed.roles.map(role => role.role), ...(job.plan?.roles.map(role => role.role) ?? [])])
  const orphan = parsed.tasks.find(task => !roleNames.has(task.role))
  if (orphan !== undefined) {
    return deny(`${TOOL}: task ${orphan.id} names role ${orphan.role}, got no such role in roles; add the role with its ratings`)
  }
  const roles: RexRole[] = []
  for (const role of parsed.roles) {
    const oms = await runOms($, { prefs: job.prefs, ratings: role.ratings })
    if (!oms.ok) {
      return deny(`${TOOL}: rex oms refused role ${role.role}, got ${oms.line}; call it again`)
    }
    roles.push({ role: role.role, ratings: role.ratings, gridAgent: oms.agent, conflict: oms.conflict.length === 0 ? null : oms.conflict.join(','), omsVersion: oms.version })
  }
  // The overrides of this call count for its own plan; they are recorded only after every check passed, so a refused plan changes nothing.
  const overrides: RexDecision[] = []
  for (const override of parsed.overrides) {
    overrides.push({ assignmentId: await overrideId(job.id, override), subject: override.subject, item: override.item, value: override.value, source: 'override', reason: override.reason, tableVersion: null, overlayVersion: null })
  }
  const newOverrides = overrides.filter(decision => !job.decisions.some(old => old.assignmentId === decision.assignmentId))
  const decisions = [...job.decisions, ...newOverrides]
  const tasks = parsed.tasks.map(task => assignedSkills(decisions, task) ?? task)
  const places = await skillPlaces($, job)
  const manifest: RexSkillFile[] = []
  for (const named of planFiles(tasks)) {
    const found = await resolveSkill($, places, named.skill, named.file)
    if (found === null) {
      return deny(`${TOOL}: no skill ${named.skill} holds ${named.file}, got ${named.skill}/${named.file}; name a skill of plugin/skills, plugin/library or .rex/skills`)
    }
    manifest.push({ skill: found.skill, file: found.file, hash: found.hash, source: found.source })
  }
  for (const decision of newOverrides) {
    await recordDecision($, job.id, decision)
  }
  if (job.plan === null) {
    await planJob($, job.id, { orchestrator: parsed.orchestrator, breakdown: parsed.breakdown, tasks, roles, manifest })
  } else {
    await amendPlan($, job.id, { tasks, roles })
  }
  const planned = (await readBook($)).jobs.find(each => each.id === job.id) ?? job
  const roleLines = (planned.plan?.roles ?? roles).flatMap(role => [
    `role.${role.role}=super-rex:${role.gridAgent}`,
    ...(role.conflict === null ? [] : [`conflict.${role.role}=${role.conflict}`]),
  ])
  return answer([`job=${job.id}`, `status=${planned.status}`, `orchestrator=${planned.plan?.orchestrator ?? parsed.orchestrator}`, ...roleLines, `manifest_files=${String(planned.plan?.manifest.length ?? manifest.length)}`])
}

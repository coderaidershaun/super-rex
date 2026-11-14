// rex_assign: reads the table in force for the caller's job, asks the decision agent what needs judgement, and records every answer and every skill it places with its source and reason.
// Every argument is checked before the book changes, and a question answered once keeps that answer: its assignment id gives the stored one back.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexDecision, RexJob, RexRating, RexRatings } from '../../types'
import { fail, readBook, recordDecision } from '../book/index'
import type { Caller, ToolEntry } from '../parts'
import { hashText } from '../skills/index'
import { answer, callerOf, deny, jobArgRefusal, runOms } from '../tools/index'
import { assignTasks } from './rules'
import type { Answer, AnswerOf, Placed, QuestionKey, TaskFacts } from './rules'
import { applyOverlay, EACH_STEP, parseTable } from './table'
import type { AssignTable, Read } from './table'

export const TOOL = 'rex_assign'

/** The role whose agent may answer the questions of its job, besides the builder. */
export const DECISION_ROLE = 'decision-maker'

/** The sources of the decisions rex_job_plan takes as a task's skills; a plan's own arguments are source `plan`. */
const ASSIGNED: readonly RexDecision['source'][] = ['rule', 'decision-agent', 'override']

const RATINGS: readonly RexRating[] = ['low', 'medium', 'high', 'none']
const TASK_ID = /^[A-Za-z0-9_-]+$/
const QUESTION = /^task:(\S+) row:(\S+) criterion:(\S+)/
const ENGINE_KEYS = new Set(['tool', 'tool_use_id', 'agentId', 'requestMeta', 'consent'])

type Fields = { readonly [key: string]: unknown }
type RoleArg = { readonly role: string; readonly ratings: RexRatings }
type AnswerArg = { readonly question: unknown; readonly answer: Answer }
type AssignArgs = { readonly tasks: readonly TaskFacts[]; readonly roles: readonly RoleArg[]; readonly answers: readonly AnswerArg[] }
type Refusal = { readonly refused: string }

function refusal(wrong: string, seen: unknown, todo: string): Refusal {
  return { refused: `${TOOL}: ${wrong}, got ${typeof seen === 'string' ? seen : (JSON.stringify(seen) ?? 'none')}; ${todo}` }
}

function isRefusal(value: unknown): value is Refusal {
  return typeof value === 'object' && value !== null && typeof Reflect.get(value, 'refused') === 'string'
}

function fieldsOf(value: unknown): Fields | null {
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? Object.fromEntries(Object.entries(value)) : null
}

function itemsOf(value: unknown, key: string, todo: string): readonly Fields[] | Refusal {
  if (!Array.isArray(value)) {
    return refusal(`${key} must be a list`, value, todo)
  }
  const items: Fields[] = []
  for (const item of value) {
    const fields = fieldsOf(item)
    if (fields === null) {
      return refusal(`each item of ${key} must be an object`, item, todo)
    }
    items.push(fields)
  }
  return items
}

/** One line of text: a reason with line ends would break the key=value lines it is shown in. */
function oneLine(text: string): string {
  return text.replace(/\s+/g, ' ').trim()
}

function taskOf(fields: Fields, job: RexJob): TaskFacts | Refusal {
  const todo = 'give each task id, role and writes_code, and langs when they differ from the job'
  const { id, role, writes_code: writesCode, langs } = fields
  if (typeof id !== 'string' || !TASK_ID.test(id)) return refusal('each task id must be letters, digits, _ or -', id, todo)
  if (typeof role !== 'string' || role.trim() === '') return refusal(`task ${id} role must be text`, role, todo)
  if (typeof writesCode !== 'boolean') return refusal(`task ${id} writes_code must be true or false`, writesCode, todo)
  if (langs !== undefined && !(Array.isArray(langs) && langs.every((lang: unknown) => typeof lang === 'string'))) return refusal(`task ${id} langs must be a list of text`, langs, todo)
  return { id, role, writesCode, langs: Array.isArray(langs) ? langs.map(String) : job.langs }
}

function roleOf(fields: Fields): RoleArg | Refusal {
  const todo = 'give each role its name and the four ratings low, medium, high or none'
  const { role } = fields
  if (typeof role !== 'string' || role.trim() === '') return refusal('each role must have a name', role, todo)
  const rating = (key: keyof RexRatings): RexRating | Refusal => RATINGS.find(each => each === fields[key]) ?? refusal(`role ${role} ${key} must be one of ${RATINGS.join(', ')}`, fields[key], todo)
  const intelligence = rating('intelligence')
  const reliability = rating('reliability')
  const consequence = rating('consequence')
  const verification = rating('verification')
  if (isRefusal(intelligence)) return intelligence
  if (isRefusal(reliability)) return reliability
  if (isRefusal(consequence)) return consequence
  if (isRefusal(verification)) return verification
  return { role, ratings: { intelligence, reliability, consequence, verification } }
}

function answerOf(fields: Fields): AnswerArg | Refusal {
  const todo = 'give each answer the question, yes or no, and a one-line reason'
  const { question, answer: given, reason } = fields
  if (typeof question !== 'string' && typeof question !== 'number') return refusal('each answer must name its question', question, todo)
  if (given !== 'yes' && given !== 'no') return refusal(`the answer to ${String(question)} must be yes or no`, given, todo)
  if (typeof reason !== 'string' || oneLine(reason) === '') return refusal(`the answer to ${String(question)} needs a reason`, reason, todo)
  return { question, answer: { answer: given, reason: oneLine(reason) } }
}

function parseArgs(args: Fields, job: RexJob): AssignArgs | Refusal {
  const taskItems = itemsOf(args['tasks'], 'tasks', 'give the tasks of the work breakdown')
  const roleItems = itemsOf(args['roles'], 'roles', 'give the roles of the role table with their ratings')
  const answerItems = args['answers'] === undefined ? [] : itemsOf(args['answers'], 'answers', 'give a list of answers')
  if (isRefusal(taskItems)) return taskItems
  if (isRefusal(roleItems)) return roleItems
  if (isRefusal(answerItems)) return answerItems
  const tasks: TaskFacts[] = []
  for (const item of taskItems) {
    const task = taskOf(item, job)
    if (isRefusal(task)) return task
    if (tasks.some(each => each.id === task.id)) return refusal('each task id must be named once', task.id, 'give each task one row')
    tasks.push(task)
  }
  const roles: RoleArg[] = []
  for (const item of roleItems) {
    const role = roleOf(item)
    if (isRefusal(role)) return role
    roles.push(role)
  }
  const answers: AnswerArg[] = []
  for (const item of answerItems) {
    const one = answerOf(item)
    if (isRefusal(one)) return one
    answers.push(one)
  }
  return { tasks, roles, answers }
}

/** Who may call it: a builder for its own job, or the decision agent of the job, by plan role or, before a plan, by its brief's Task line. */
function callerProblem(caller: Caller, run: { readonly role: string | null; readonly task: string | null } | undefined): string | null {
  const job = caller.job
  const isDecisionAgent = caller.kind === 'agent' && job !== null && (run?.role === DECISION_ROLE || (job.plan === null && run?.task === DECISION_ROLE))
  if ((caller.kind === 'builder' && job !== null) || isDecisionAgent) {
    return null
  }
  return `${TOOL}: only a builder or the ${DECISION_ROLE} agent of its job may call it, got ${caller.kind}`
}

async function readText($: EngineInterface, path: string): Promise<Read<string>> {
  try {
    return { ok: true, value: await $.fs.read(path) }
  } catch (error) {
    if (error instanceof Error) {
      return { ok: false, problem: `${path} could not be read: ${error.message}` }
    }
    throw error
  }
}

/** The plugin's table with the job repo's overlay, when the repo has one. */
export async function tableFor($: EngineInterface, job: RexJob): Promise<Read<AssignTable> & { readonly file: string }> {
  const tablePath = `${$.plugin.root}/assign/table.json`
  const text = await readText($, tablePath)
  if (!text.ok) return { ...text, file: 'assign/table.json of the plugin' }
  const table = parseTable(text.value, await hashText(text.value))
  if (!table.ok) return { ...table, file: 'assign/table.json of the plugin' }
  const overlayPath = `${job.repo}/.rex/assign.json`
  if (!(await $.fs.exists(overlayPath))) return { ...table, file: overlayPath }
  const overlay = await readText($, overlayPath)
  if (!overlay.ok) return { ...overlay, file: overlayPath }
  return { ...applyOverlay(table.value, overlay.value, await hashText(overlay.value)), file: overlayPath }
}

function keyText(key: QuestionKey): string {
  return `task:${key.task} row:${key.row} criterion:${key.criterion}`
}

function answerItem(key: QuestionKey): string {
  return `answer.${key.row}.${key.criterion}`
}

function isOfTable(decision: RexDecision, table: AssignTable): boolean {
  return decision.tableVersion === table.tableVersion && decision.overlayVersion === table.overlayVersion
}

/** The answers the book holds for this table, by question. */
function storedAnswers(job: RexJob, table: AssignTable): ReadonlyMap<string, Answer> {
  const stored = new Map<string, Answer>()
  for (const decision of job.decisions) {
    const [kind, row, criterion] = decision.item.split('.')
    const isAnswer = kind === 'answer' && decision.subject.startsWith('task:') && (decision.value === 'yes' || decision.value === 'no')
    if (isAnswer && isOfTable(decision, table) && row !== undefined && criterion !== undefined) {
      const key = { task: decision.subject.slice('task:'.length), row, criterion }
      stored.set(keyText(key), { answer: decision.value === 'yes' ? 'yes' : 'no', reason: decision.reason })
    }
  }
  return stored
}

/** A question the table can ask of one of the given tasks: a criterion with no rule, or the placement of a row that asks. */
function isAskable(key: QuestionKey, table: AssignTable, tasks: readonly TaskFacts[]): boolean {
  const row = table.rows.find(each => each.id === key.row)
  const isCriterion = row?.criteria.some(criterion => criterion.id === key.criterion && criterion.rule === null) === true
  const isPlacement = key.criterion === EACH_STEP && row?.placement === 'ask'
  return tasks.some(task => task.id === key.task) && (isCriterion || isPlacement)
}

/** The answers of this call by question: a question line's `task:… row:… criterion:…` text, or its number among the questions still open. */
function givenAnswers(answers: readonly AnswerArg[], table: AssignTable, tasks: readonly TaskFacts[], open: readonly QuestionKey[]): ReadonlyMap<string, Answer> | Refusal {
  const given = new Map<string, Answer>()
  for (const one of answers) {
    const number = typeof one.question === 'number' ? one.question : /^\d+$/.test(String(one.question)) ? Number(one.question) : null
    const named = number === null ? QUESTION.exec(String(one.question)) : null
    const key = number !== null ? open[number - 1] : named === null ? undefined : { task: named[1] ?? '', row: named[2] ?? '', criterion: named[3] ?? '' }
    if (key === undefined || !isAskable(key, table, tasks)) {
      return refusal('an answer names no question of these tasks', one.question, 'copy the task:<id> row:<row> criterion:<id> text of a question line')
    }
    const earlier = given.get(keyText(key))
    if (earlier !== undefined && earlier.answer !== one.answer.answer) {
      return refusal(`${keyText(key)} has two answers`, `${earlier.answer} and ${one.answer.answer}`, 'give each question one answer')
    }
    given.set(keyText(key), one.answer)
  }
  return given
}

/** The first table and overlay this job was assigned with; a later table would mix two tables in one plan. */
function versionProblem(job: RexJob, table: AssignTable): string | null {
  const earlier = job.decisions.find(decision => decision.source !== 'override' && decision.tableVersion !== null && !isOfTable(decision, table))
  if (earlier === undefined) {
    return null
  }
  const versions = (tableVersion: string | null, overlayVersion: string | null) => `table ${tableVersion ?? 'none'} and overlay ${overlayVersion ?? 'none'}`
  return `${TOOL}: job ${job.id} was assigned with ${versions(earlier.tableVersion, earlier.overlayVersion)}, got ${versions(table.tableVersion, table.overlayVersion)}; give each change as an override of rex_job_plan`
}

async function idOf(...parts: readonly string[]): Promise<string> {
  return hashText(parts.join('|'))
}

export type DecisionItem = { readonly item: string; readonly value: string; readonly placed: Placed }

/**
 * The record of one task's placements, in the shape rex_job_plan reads: a
 * read-first skill is item `skill`, an open-when skill `open_when.<skill>`,
 * and an open-when file inside a skill `file.<skill>/<file>`, each with its case.
 */
export function decisionItems(placements: readonly Placed[]): readonly DecisionItem[] {
  return placements.map(placed => {
    if (placed.file !== null) return { item: `file.${placed.skill}/${placed.file}`, value: placed.when ?? '', placed }
    return placed.place === 'read-first' ? { item: 'skill', value: placed.skill, placed } : { item: `open_when.${placed.skill}`, value: placed.when ?? '', placed }
  })
}

function ratingsText(ratings: RexRatings): string {
  return `rated intelligence ${ratings.intelligence}, reliability ${ratings.reliability}, consequence ${ratings.consequence}, verification ${ratings.verification}`
}

/** The lines of a finished assignment, from the book: what rex_job_plan will take for each task. */
function resultLines(job: RexJob, tasks: readonly TaskFacts[], roles: readonly { readonly role: string; readonly agent: string }[]): readonly string[] {
  const lines: string[] = ['questions=0']
  for (const task of tasks) {
    const own = job.decisions.filter(decision => decision.subject === `task:${task.id}` && ASSIGNED.includes(decision.source))
    const skills = own.filter(decision => decision.item === 'skill')
    const opens = own.flatMap(decision => {
      const name = decision.item.startsWith('open_when.') ? decision.item.slice('open_when.'.length) : decision.item.startsWith('file.') ? decision.item.slice('file.'.length) : null
      return name === null ? [] : [{ name, decision }]
    })
    lines.push(`task.${task.id}.skills=${skills.length === 0 ? 'none' : skills.map(decision => decision.value).join(',')}`)
    lines.push(...opens.map(({ name, decision }) => `task.${task.id}.open_when.${name}=${decision.value}`))
    lines.push(...skills.map(decision => `source.${task.id}.${decision.value}=${decision.source}`))
    lines.push(...opens.map(({ name, decision }) => `source.${task.id}.${name}=${decision.source}`))
  }
  lines.push(...roles.map(role => `role.${role.role}=super-rex:${role.agent}`))
  return lines
}

async function refuse($: EngineInterface, text: string, argument: string): Promise<ToolCallResult> {
  await fail($, { kind: 'bad-argument', tool: TOOL, argument, value: text })
  return deny(text)
}

export async function serveAssign($: EngineInterface, event: { readonly agentId?: string }): Promise<ToolCallResult> {
  const book = await readBook($)
  const caller = await callerOf($, event.agentId)
  const run = book.runs.find(each => each.agentId === event.agentId)
  const problem = callerProblem(caller, run)
  const job = caller.job
  if (problem !== null || job === null) {
    await fail($, { kind: 'caller-not-allowed', tool: TOOL, caller: caller.kind })
    return deny(problem ?? `${TOOL}: only a builder may call it, got ${caller.kind}`)
  }
  const args: Fields = Object.fromEntries(Object.entries(event).filter(([key]) => !ENGINE_KEYS.has(key)))
  const jobText = jobArgRefusal(TOOL, args['job'], job)
  if (jobText !== null) return refuse($, jobText, 'job')
  const parsed = parseArgs(args, job)
  if (isRefusal(parsed)) return refuse($, parsed.refused, 'arguments')
  const table = await tableFor($, job)
  if (!table.ok) return refuse($, `${TOOL}: the assignment table is broken, got ${table.problem}; fix ${table.file}`, 'table')
  const versionText = versionProblem(job, table.value)
  if (versionText !== null) return refuse($, versionText, 'table')

  const stored = storedAnswers(job, table.value)
  const before = assignTasks(table.value, parsed.tasks, key => stored.get(keyText(key)))
  const open = before.kind === 'questions' ? before.questions : []
  const given = givenAnswers(parsed.answers, table.value, parsed.tasks, open)
  if (isRefusal(given)) return refuse($, given.refused, 'answers')
  const lookup: AnswerOf = key => stored.get(keyText(key)) ?? given.get(keyText(key))
  const assigned = assignTasks(table.value, parsed.tasks, lookup)

  const roles: { readonly role: string; readonly agent: string; readonly ratings: RexRatings }[] = []
  if (assigned.kind === 'placed') {
    for (const role of parsed.roles) {
      const oms = await runOms($, { prefs: job.prefs, ratings: role.ratings })
      if (!oms.ok) {
        return deny(`${TOOL}: rex oms refused role ${role.role}, got ${oms.line}; call it again`)
      }
      roles.push({ role: role.role, agent: oms.agent, ratings: role.ratings })
    }
  }

  const versions = [table.value.tableVersion, table.value.overlayVersion ?? 'none'] as const
  const record = async (decision: Omit<RexDecision, 'assignmentId' | 'tableVersion' | 'overlayVersion'>, idParts: readonly string[], isOfTableRow: boolean): Promise<void> => {
    const tableVersion = isOfTableRow ? table.value.tableVersion : null
    const overlayVersion = isOfTableRow ? table.value.overlayVersion : null
    await recordDecision($, job.id, { ...decision, assignmentId: await idOf(job.id, ...idParts), tableVersion, overlayVersion })
  }
  for (const [text, one] of given) {
    if (stored.has(text)) continue
    const named = QUESTION.exec(text)
    const key = { task: named?.[1] ?? '', row: named?.[2] ?? '', criterion: named?.[3] ?? '' }
    await record({ subject: `task:${key.task}`, item: answerItem(key), value: one.answer, source: 'decision-agent', reason: one.reason }, [`task:${key.task}`, answerItem(key), ...versions], true)
  }
  if (assigned.kind === 'questions') {
    return answer([`questions=${String(assigned.questions.length)}`, ...assigned.questions.map((question, index) => `question.${String(index + 1)}=${keyText(question)} ${question.text}`)])
  }
  for (const task of assigned.tasks) {
    for (const { item, value, placed } of decisionItems(task.placements)) {
      await record({ subject: `task:${task.id}`, item, value, source: placed.source, reason: placed.reason }, [`task:${task.id}`, item, item === 'skill' ? value : '', ...versions], true)
    }
  }
  for (const role of roles) {
    await record({ subject: `role:${role.role}`, item: 'grid_agent', value: `super-rex:${role.agent}`, source: 'decision-agent', reason: ratingsText(role.ratings) }, [`role:${role.role}`, 'grid_agent', ratingsText(role.ratings)], false)
  }
  const recorded = (await readBook($)).jobs.find(each => each.id === job.id) ?? job
  return answer(resultLines(recorded, parsed.tasks, roles))
}

export const ASSIGN_TOOL: ToolEntry = {
  name: TOOL,
  description:
    "Assigns the skills of your job's tasks from the assignment table. The first call answers the questions that need judgement (questions=, question.<n>=). Call again with an answer to each: yes or no, with a one-line reason. Then it answers each task's read-first skills and open-when cases with their source, and the grid agent of each role. A question answered once keeps that answer.",
  inputSchema: {
    type: 'object',
    properties: {
      job: { type: 'string', description: 'Your own job id; left out, your own job.' },
      tasks: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            id: { type: 'string', description: 'The task id of the work breakdown; a lead or a role with no task gives its role name.' },
            role: { type: 'string' },
            writes_code: { type: 'boolean', description: 'True when the task writes or changes code.' },
            langs: { type: 'array', items: { type: 'string' }, description: 'The languages of its code, as rex codebase --only names them; left out, the languages of the job.' },
          },
          required: ['id', 'role', 'writes_code'],
        },
      },
      roles: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            role: { type: 'string' },
            intelligence: { type: 'string', enum: [...RATINGS] },
            reliability: { type: 'string', enum: [...RATINGS] },
            consequence: { type: 'string', enum: [...RATINGS] },
            verification: { type: 'string', enum: [...RATINGS] },
          },
          required: ['role', 'intelligence', 'reliability', 'consequence', 'verification'],
        },
      },
      answers: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            question: { type: 'string', description: 'The task:<id> row:<row> criterion:<id> text of a question line, or its number.' },
            answer: { type: 'string', enum: ['yes', 'no'] },
            reason: { type: 'string', description: 'One line: why.' },
          },
          required: ['question', 'answer', 'reason'],
        },
      },
    },
    required: ['tasks', 'roles'],
  },
  isDeferred: true,
  callers: ['builder', 'agent'],
}

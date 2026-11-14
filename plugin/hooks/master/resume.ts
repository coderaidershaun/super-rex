// /rex master resume: a new master for the newest tree the store holds open, its jobs and questions put back as they were, and each open job's dead builder started again or asked about by policy.
import type { CommandRunResult, EngineInterface } from 'claude-code'

import type { RexJob, RexJobStatus, RexLevel, RexMergeAnswer, RexOriginKind, RexQuestion, RexTree } from '../../types'
import { addQuestion, fail, isOpen, readBook, restoreTree } from '../book/index'
import type { Restored } from '../book/index'
import type { StatusSection, Table } from '../parts'
import { restartBuilder } from './build'
import { spawnMaster } from './start'
import type { StoredPolicy } from './store'
import { startWatchdog } from './watch'

const NAME = 'rex master resume'
const DONE_AGENT: readonly string[] = ['completed', 'failed', 'killed']
const START_AGAIN = 'Start a new builder'
const LEAVE_BLOCKED = 'Leave it blocked'

function field(value: unknown, key: string): unknown {
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? Reflect.get(value, key) : undefined
}

function textOf(value: unknown, key: string): string | null {
  const found = field(value, key)
  return typeof found === 'string' ? found : null
}

function numberOf(value: unknown, key: string): number {
  const found = field(value, key)
  return typeof found === 'number' && Number.isFinite(found) ? found : 0
}

function textsOf(value: unknown, key: string): readonly string[] {
  const found = field(value, key)
  return Array.isArray(found) ? found.filter((each: unknown): each is string => typeof each === 'string') : []
}

function listOf(value: unknown, key: string): readonly unknown[] {
  const found = field(value, key)
  const list: readonly unknown[] = Array.isArray(found) ? found : []
  return list
}

function oneOf<T extends string>(value: string | null, allowed: readonly T[]): T | null {
  return allowed.find(each => each === value) ?? null
}

function timeOf(value: unknown, key: string): number {
  const parsed = Date.parse(textOf(value, key) ?? '')
  return Number.isNaN(parsed) ? 0 : parsed
}

function nanoOf(usd: number): number {
  return Math.round(usd * 1e9)
}

const LEVELS: readonly RexLevel[] = ['low', 'medium', 'high']
const STATUSES: readonly RexJobStatus[] = ['created', 'planned', 'blocked', 'failed', 'delivered', 'cancelled', 'lost']
/** The store's statuses after delivery, which the book holds as delivered. */
const AFTER_DELIVERY: readonly string[] = ['accepted', 'rejected', 'analyzed']

function statusOf(job: unknown): RexJobStatus {
  const stored = textOf(job, 'stored_status') ?? textOf(job, 'status')
  return AFTER_DELIVERY.includes(stored ?? '') ? 'delivered' : (oneOf(stored, STATUSES) ?? 'lost')
}

function mergeOf(job: unknown): RexMergeAnswer | null {
  const merge = field(job, 'merge')
  const answer = oneOf(textOf(merge, 'answer'), ['yes', 'no'] as const)
  const source = oneOf(textOf(merge, 'source'), ['dialog', 'person-prompt', 'sdk-prompt', 'question', 'policy'] as const)
  if (answer === null || source === null) {
    return null
  }
  return { answer, source, question: textOf(merge, 'question'), policyHash: textOf(job, 'policy_hash'), at: timeOf(job, 'updated_at') }
}

/** One job as `rex job show --json` prints it, which `rex tree show --json` repeats for each job of the tree. */
function jobOf(job: unknown): RexJob | null {
  const id = textOf(job, 'id')
  const repo = textOf(job, 'repo')
  if (id === null || repo === null) {
    return null
  }
  const merge = mergeOf(job)
  const origin: RexOriginKind = oneOf(textOf(job, 'origin'), ['composer', 'bridge', 'sdk', 'master'] as const) ?? 'master'
  return {
    id,
    owner: { session: textOf(job, 'session_id') ?? '', builder: textOf(job, 'builder_agent_id') ?? 'pending' },
    tree: textOf(job, 'tree'),
    origin,
    request: textOf(job, 'request') ?? '',
    restatement: textOf(job, 'restatement'),
    repo,
    branch: textOf(job, 'branch') ?? `rex/${id}`,
    base: textOf(job, 'base') ?? 'main',
    worktree: textOf(job, 'worktree'),
    type: oneOf(textOf(job, 'work_type'), ['feature', 'fix', 'refactor', 'chore', 'research', 'other'] as const),
    size: oneOf(textOf(job, 'size'), ['small', 'medium', 'large'] as const),
    consequence: oneOf(textOf(job, 'consequence'), LEVELS),
    langs: textsOf(job, 'languages'),
    prefs: { duration: oneOf(textOf(job, 'pref_duration'), LEVELS), budget: oneOf(textOf(job, 'pref_budget'), LEVELS), autonomy: oneOf(textOf(job, 'pref_autonomy'), LEVELS) },
    status: statusOf(job),
    outcome: textOf(job, 'outcome'),
    merge,
    mergeQuestion: merge?.question ?? null,
    limit: { nano: nanoOf(numberOf(job, 'limit_usd')), source: oneOf(textOf(job, 'limit_source'), ['default', 'set', 'policy'] as const) ?? 'default' },
    spentNano: 0,
    tokens: { input: 0, output: 0, cacheRead: 0, cacheCreation: 0 },
    plan: null,
    decisions: [],
    deliveries: [],
    degraded: textsOf(job, 'degraded'),
    dependsOn: [],
    mergeAfter: [],
    outputFile: null,
    retries: 0,
    stuckWakes: 0,
    doneChecks: [],
    createdAt: timeOf(job, 'created_at'),
    updatedAt: timeOf(job, 'updated_at'),
  }
}

function questionOf(question: unknown): RexQuestion | null {
  const id = textOf(question, 'id')
  if (id === null) {
    return null
  }
  const option = textOf(question, 'answer')
  const source = oneOf(textOf(question, 'answer_source'), ['dialog', 'pane', 'command'] as const) ?? 'command'
  return {
    id,
    tree: textOf(question, 'tree'),
    job: textOf(question, 'job'),
    question: textOf(question, 'question') ?? '',
    options: textsOf(question, 'options'),
    recommendation: textOf(question, 'recommendation') ?? '',
    category: oneOf(textOf(question, 'category'), ['merge', 'scope', 'spend', 'other'] as const) ?? 'other',
    mergeFor: textOf(question, 'merge_for'),
    askedAt: timeOf(question, 'asked_at'),
    answer: option === null ? null : { option, source, at: timeOf(question, 'answered_at') },
  }
}

/** The tree comes back in this session, with no master yet, pinned to the policy that pre-flight just found confirmed. */
function treeOf(tree: unknown, session: string, policyHash: string): RexTree | null {
  const id = textOf(tree, 'id')
  const charter = textOf(tree, 'charter')
  if (id === null || charter === null) {
    return null
  }
  return {
    id,
    session,
    master: null,
    charter,
    charterHash: textOf(tree, 'charter_hash') ?? '',
    policyHash,
    state: 'running',
    ledgerNano: 0,
    capNano: nanoOf(numberOf(tree, 'cap_usd')),
    builders: [],
    ticks: [],
    notes: [],
    startedAt: timeOf(tree, 'started_at'),
    closedAt: null,
    reportFile: null,
  }
}

/** The answer of `rex tree show --json`, or the text that refuses the resume. */
async function storedTree($: EngineInterface, session: string, policyHash: string): Promise<Restored | string> {
  const ran = await $.process.run(['rex', 'tree', 'show', '--json'], { cwd: await $.session.root() })
  if (ran.exitCode !== 0) {
    const line = ran.stderr.split('\n')[0]?.trim() ?? ''
    await fail($, { kind: 'binary-read-failed', argv: ['rex', 'tree', 'show', '--json'], exitCode: ran.exitCode, stderr: ran.stderr })
    return `${NAME}: rex tree show failed, got exit ${String(ran.exitCode)}: ${line}; run rex doctor`
  }
  let shown: unknown
  try {
    shown = JSON.parse(ran.stdout)
  } catch (error) {
    if (error instanceof SyntaxError) {
      return `${NAME}: rex tree show printed no JSON, got ${ran.stdout.slice(0, 80)}; run rex doctor`
    }
    throw error
  }
  const tree = treeOf(field(shown, 'tree'), session, policyHash)
  if (tree === null) {
    return `${NAME}: no open tree in the store of this repo, got none; start one with /rex master <goal>`
  }
  const jobs = listOf(shown, 'jobs').map(jobOf).filter((job): job is RexJob => job !== null)
  const questions = listOf(shown, 'questions').map(questionOf).filter((question): question is RexQuestion => question !== null)
  return { tree, jobs, questions }
}

/** A tree of this session whose master still runs; resume would start a second master for it. */
async function liveRefusal($: EngineInterface): Promise<string | null> {
  const book = await readBook($)
  const agents = await $.agent.list()
  for (const tree of book.trees.filter(each => each.session === book.session && (each.state === 'running' || each.state === 'paused') && each.master !== null)) {
    const master = agents.find(agent => agent.id === tree.master)
    if (master !== undefined && !DONE_AGENT.includes(master.status)) {
      return `${NAME}: tree ${tree.id} is live in this session, got master ${master.id}; use /rex policy confirm`
    }
  }
  return null
}

function lines(rows: readonly string[]): CommandRunResult {
  return { text: rows.map(row => `${row}\n`).join('') }
}

function listed(ids: readonly string[]): string {
  return ids.length === 0 ? 'none' : ids.join(',')
}

export async function serveResume($: EngineInterface, policy: StoredPolicy, sections: Table<StatusSection>): Promise<CommandRunResult> {
  const live = await liveRefusal($)
  if (live !== null) {
    return { text: live }
  }
  const restored = await storedTree($, (await readBook($)).session, policy.hash)
  if (typeof restored === 'string') {
    return { text: restored }
  }
  await restoreTree($, restored)
  const { tree } = restored
  const open = restored.jobs.filter(isOpen)
  const isContinue = policy.policy.on_dead_builder === 'continue'
  const digest = restored.jobs.map(job => `job ${job.id} ${job.status}${isOpen(job) ? (isContinue ? ': its builder is gone, so a new builder continues it' : ': its builder is gone, and a question asks the person') : ''}`)
  const asked: string[] = []
  if (!isContinue) {
    for (const job of open) {
      const question = await addQuestion($, {
        tree: tree.id,
        job: job.id,
        question: `The builder of job ${job.id} ended with the last session. Start a new builder for it?`,
        options: [START_AGAIN, LEAVE_BLOCKED],
        recommendation: START_AGAIN,
        category: 'other',
        mergeFor: null,
      })
      asked.push(question.id)
    }
  }
  const questions = (await readBook($)).questions.filter(question => question.tree === tree.id && question.answer === null)
  digest.push(...questions.map(question => `question ${question.id} parked: ${question.question}`))
  const started = await spawnMaster($, { tree: tree.id, charter: tree.charter, digest: digest.join('\n') })
  if ('refused' in started) {
    return { text: started.refused }
  }
  const continued: string[] = []
  for (const job of isContinue ? open : []) {
    const restarted = await restartBuilder($, job, { ...tree, master: started.master }, policy.policy)
    if ('refused' in restarted) {
      $.ui.status(`rex: job ${job.id} of tree ${tree.id} got no new builder: ${restarted.refused}`)
    } else {
      continued.push(job.id)
    }
  }
  startWatchdog($, tree.id, policy.policy.watchdog_minutes, sections)
  return lines([`tree=${tree.id}`, `master=${started.master}`, `continued=${listed(continued)}`, `asked=${listed(asked)}`])
}

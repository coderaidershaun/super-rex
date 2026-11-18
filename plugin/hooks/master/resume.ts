// The resume of a master tree: the newest tree the store holds open comes back into this session with the main loop as its master, its jobs and questions put back as they were, and each open job's dead builder started again or asked about by policy.
import type { EngineInterface } from 'claude-code'

import type { RexJob, RexJobStatus, RexLevel, RexMergeAnswer, RexOriginKind, RexQuestion, RexTree } from '../../types'
import { fail, isOpen, MAIN_MASTER, readBook, restoreTree } from '../book/index'
import type { Restored } from '../book/index'
import { goneQuestionText, LEAVE_BLOCKED, START_AGAIN, startAgain } from './answers'
import { announceQuestion, parkQuestion } from './ask'
import type { StatusSection, Table } from '../parts'
import type { Policy } from './policy'
import type { StoredPolicy } from './store'
import { startWatchdog } from './watch'

const NAME = 'rex_tree_open resume'

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
  const source = oneOf(textOf(question, 'answer_source'), ['dialog', 'pane', 'command', 'mod'] as const) ?? 'command'
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

/** The tree comes back in this session, with its main loop as master, pinned to the policy that pre-flight just found confirmed: its hash, and its `tree_usd` as the cap. */
function treeOf(tree: unknown, session: string, policy: StoredPolicy): RexTree | null {
  const id = textOf(tree, 'id')
  const charter = textOf(tree, 'charter')
  if (id === null || charter === null) {
    return null
  }
  return {
    id,
    session,
    master: MAIN_MASTER,
    charter,
    charterHash: textOf(tree, 'charter_hash') ?? '',
    policyHash: policy.hash,
    state: 'running',
    ledgerNano: 0,
    capNano: nanoOf(policy.policy.tree_usd),
    builders: [],
    ticks: [],
    notes: [],
    startedAt: timeOf(tree, 'started_at'),
    closedAt: null,
    reportFile: null,
  }
}

/**
 * An open job comes back with what it has spent so far, as `rex limit` reads
 * it from the store, so its limit holds across a resume. A read that fails
 * leaves the spend at nothing, and the job still comes back.
 */
async function withStoredSpend($: EngineInterface, job: RexJob): Promise<RexJob> {
  const ran = await $.process.run(['rex', 'limit', job.id, '--json'], { cwd: await $.session.root() })
  if (ran.exitCode !== 0) {
    return job
  }
  try {
    const shown: unknown = JSON.parse(ran.stdout)
    return { ...job, spentNano: nanoOf(numberOf(shown, 'spent_usd')) }
  } catch (error) {
    if (error instanceof SyntaxError) {
      return job
    }
    throw error
  }
}

/** The answer of `rex tree show --json`, or the text that refuses the resume. */
async function storedTree($: EngineInterface, session: string, policy: StoredPolicy): Promise<Restored | string> {
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
  const tree = treeOf(field(shown, 'tree'), session, policy)
  if (tree === null) {
    return `${NAME}: no open tree in the store of this repo, got none; the person starts one with /super-rex:orchestration-master <goal>`
  }
  const stored = listOf(shown, 'jobs').map(jobOf).filter((job): job is RexJob => job !== null)
  const jobs = await Promise.all(stored.map(job => (isOpen(job) ? withStoredSpend($, job) : Promise.resolve(job))))
  const questions = listOf(shown, 'questions').map(questionOf).filter((question): question is RexQuestion => question !== null)
  return { tree, jobs, questions }
}

function listed(ids: readonly string[]): string {
  return ids.length === 0 ? 'none' : ids.join(',')
}

/** What became of the open jobs whose builders are gone: the jobs a new builder goes on with, and the questions parked for the others. */
export type Revived = { readonly continued: readonly string[]; readonly asked: readonly string[] }

/**
 * Each open job whose builder is gone gets a new builder that goes on from
 * the job's record (`on_dead_builder: continue`), or one question for the
 * person (`ask`); a job that already has such a question waiting gets no
 * second one. A restart that fails stops that job's restart only.
 */
export async function reviveBuilders($: EngineInterface, tree: RexTree, jobs: readonly RexJob[], policy: Policy): Promise<Revived> {
  const continued: string[] = []
  const asked: string[] = []
  for (const job of jobs) {
    if (policy.on_dead_builder === 'continue') {
      if (await startAgain($, tree, job, 'its builder ended with the last session, and on_dead_builder of the policy is continue')) {
        continued.push(job.id)
      }
      continue
    }
    const text = goneQuestionText(job.id)
    const waiting = (await readBook($)).questions.find(question => question.job === job.id && question.question === text && question.answer === null)
    const question = waiting ?? (await parkQuestion($, { tree: tree.id, job: job.id, question: text, options: [START_AGAIN, LEAVE_BLOCKED], recommendation: START_AGAIN, category: 'other', mergeFor: null }))
    asked.push(question.id)
  }
  return { continued, asked }
}

/** A resumed tree: the lines of the tool's answer, and the digest and charter its brief is made from. */
export type Resumed = { readonly tree: RexTree; readonly lines: readonly string[]; readonly digest: string }

/** Brings the newest open tree of the store back into this session; a text is the refusal. */
export async function resumeTree($: EngineInterface, policy: StoredPolicy, sections: Table<StatusSection>): Promise<Resumed | string> {
  const restored = await storedTree($, (await readBook($)).session, policy)
  if (typeof restored === 'string') {
    return restored
  }
  await restoreTree($, restored)
  const { tree } = restored
  const open = restored.jobs.filter(isOpen)
  const isContinue = policy.policy.on_dead_builder === 'continue'
  const digest = restored.jobs.map(job => `job ${job.id} ${job.status}${isOpen(job) ? (isContinue ? ': its builder is gone, so a new builder continues it' : ': its builder is gone, and a question asks the person') : ''}`)
  // A question that waited when the last session ended is shown again, so the person can answer it here.
  for (const question of restored.questions.filter(each => each.answer === null)) {
    announceQuestion($, question)
  }
  const revived = await reviveBuilders($, tree, open, policy.policy)
  const questions = (await readBook($)).questions.filter(question => question.tree === tree.id && question.answer === null)
  digest.push(...questions.map(question => `question ${question.id} parked: ${question.question}`))
  startWatchdog($, tree.id, policy.policy.watchdog_minutes, sections)
  return { tree, lines: [`continued=${listed(revived.continued)}`, `asked=${listed(revived.asked)}`], digest: digest.join('\n') }
}

/**
 * `resume` typed in the session that still holds the tree, as after the
 * person stopped a builder: the tree stays as it is, and each open job whose
 * builder the engine no longer lists gets the step of `on_dead_builder`. A
 * builder the engine still lists is left alone.
 */
export async function resumeLive($: EngineInterface, tree: RexTree, policy: StoredPolicy): Promise<Resumed> {
  const book = await readBook($)
  const alive = new Set((await $.agent.list()).map(agent => agent.id))
  const jobs = book.jobs.filter(job => job.tree === tree.id)
  const gone = jobs.filter(isOpen).filter(job => {
    const builder = tree.builders.find(each => each.job === job.id)?.agentId
    return builder === undefined || !alive.has(builder)
  })
  const revived = await reviveBuilders($, tree, gone, policy.policy)
  const isContinue = policy.policy.on_dead_builder === 'continue'
  const digest = jobs.map(job => {
    const state = !gone.some(each => each.id === job.id) ? '' : isContinue ? ': its builder is gone, so a new builder continues it' : ': its builder is gone, and a question asks the person'
    return `job ${job.id} ${job.status}${state}`
  })
  const questions = (await readBook($)).questions.filter(question => question.tree === tree.id && question.answer === null)
  digest.push(...questions.map(question => `question ${question.id} parked: ${question.question}`))
  return { tree, lines: [`continued=${listed(revived.continued)}`, `asked=${listed(revived.asked)}`], digest: digest.join('\n') }
}

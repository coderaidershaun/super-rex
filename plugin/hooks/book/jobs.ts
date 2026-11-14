// Pure changes of jobs, their plans, decisions, answers and questions; each one also queues its event for the binary.
import type {
  RexAnswer,
  RexBook,
  RexDecision,
  RexDelivery,
  RexDoneCheck,
  RexJob,
  RexJobStatus,
  RexLevel,
  RexLimit,
  RexMergeAnswer,
  RexOriginKind,
  RexOwner,
  RexPlan,
  RexQuestion,
  RexRole,
  RexTask,
} from '../../types'
import { assertNever } from './failure'
import { findJob, isOpen, NO_TOKENS, queued, replaceJob, withMark } from './events'
import { questionIdOf } from './ids'
import type { EventData } from './events'
import { readPersonWords } from './words'

export type OpenJobInput = {
  /** `main`, the agent id of a builder, or `pending` for a builder not started yet. */
  readonly builder: string
  readonly tree: string | null
  readonly origin: RexOriginKind
  readonly request: string
  readonly repo: string
  readonly base: string
  readonly type: RexJob['type']
  readonly size: RexJob['size']
  readonly consequence: RexLevel | null
  readonly langs: readonly string[]
  readonly prefs: RexJob['prefs']
  readonly limit: RexLimit
  readonly dependsOn: readonly string[]
  readonly mergeAfter: readonly string[]
}

/**
 * The job worktree of `id`: `<repo>.worktrees/<id>`, beside the repo, the only
 * folder `rex worktree add` makes job worktrees in.
 */
export function jobWorktree(repo: string, id: string): string {
  return `${repo.replace(/\/+$/, '')}.worktrees/${id}`
}

export type OpenedJob = { readonly book: RexBook; readonly job: RexJob; readonly isExisting: boolean }

export function openJobIn(book: RexBook, input: OpenJobInput, id: string, at: number): OpenedJob {
  const owner: RexOwner = { session: book.session, builder: input.builder }
  const existing = book.jobs.find(
    job => isOpen(job) && input.builder !== 'pending' && job.owner.session === owner.session && job.owner.builder === owner.builder,
  )
  if (existing !== undefined) {
    return { book, job: existing, isExisting: true }
  }
  const start = input.builder === 'main' ? book.jobStartPrompt : null
  const words = start === null ? null : readPersonWords(start.text)
  const limit: RexLimit = words?.limit.kind === 'amount' ? { nano: words.limit.nano, source: 'set' } : input.limit
  const job: RexJob = {
    id,
    owner,
    tree: input.tree,
    origin: input.origin,
    request: input.request,
    restatement: start?.restatement ?? null,
    repo: input.repo,
    branch: `rex/${id}`,
    base: input.base,
    worktree: jobWorktree(input.repo, id),
    type: input.type,
    size: input.size,
    consequence: input.consequence,
    langs: input.langs,
    prefs: input.prefs,
    status: 'created',
    outcome: null,
    merge: null,
    mergeQuestion: null,
    limit,
    spentNano: 0,
    tokens: NO_TOKENS,
    plan: null,
    decisions: [],
    deliveries: [],
    degraded: [],
    dependsOn: input.dependsOn,
    mergeAfter: input.mergeAfter,
    outputFile: null,
    retries: 0,
    stuckWakes: 0,
    doneChecks: [],
    createdAt: at,
    updatedAt: at,
  }
  let changed = queued({ ...book, jobs: [...book.jobs, job], jobStartPrompt: start === null ? book.jobStartPrompt : null }, at, 'job.open', owner.builder, id, jobOpenData(job))
  const source = start === null ? null : personSource(start.origin)
  if (words?.merge !== null && words?.merge !== undefined && source !== null) {
    changed = mergeAnswerIn(changed, id, { answer: words.merge, source, question: null, policyHash: null, at }, at)
  }
  if (start?.restatementSkipped !== null && start?.restatementSkipped !== undefined) {
    changed = withMark(changed, id, `restatement:${start.restatementSkipped}`, at)
  }
  return { book: changed, job: findJob(changed, id) ?? job, isExisting: false }
}

/** The merge source of a person's own words, by where they came from; a master's job takes none from words. */
export function personSource(origin: RexOriginKind): RexMergeAnswer['source'] | null {
  switch (origin) {
    case 'composer':
    case 'bridge':
      return 'person-prompt'
    case 'sdk':
      return 'sdk-prompt'
    case 'master':
      return null
    default:
      return assertNever(origin)
  }
}

function jobOpenData(job: RexJob): EventData {
  return {
    owner: { session: job.owner.session, builder: job.owner.builder },
    origin: job.origin,
    request: job.request,
    restatement: job.restatement,
    repo: job.repo,
    branch: job.branch,
    base: job.base,
    worktree: job.worktree,
    type: job.type,
    size: job.size,
    consequence: job.consequence,
    langs: job.langs,
    prefs: { duration: job.prefs.duration, budget: job.prefs.budget, autonomy: job.prefs.autonomy },
    limit_nano: job.limit.nano,
    limit_source: job.limit.source,
    tree: job.tree,
    depends_on: job.dependsOn,
    merge_after: job.mergeAfter,
  }
}

function tasksData(tasks: readonly RexTask[]): EventData[] {
  return tasks.map(task => ({
    id: task.id,
    role: task.role,
    skills: task.skills,
    open_when: task.openWhen.map(open => ({ skill: open.skill, file: open.file, when: open.when })),
  }))
}

function rolesData(roles: readonly RexRole[]): EventData[] {
  return roles.map(role => ({
    role: role.role,
    intelligence: role.ratings.intelligence,
    reliability: role.ratings.reliability,
    consequence: role.ratings.consequence,
    verification: role.ratings.verification,
    grid_agent: role.gridAgent,
    conflict: role.conflict,
    oms_version: role.omsVersion,
  }))
}

export function planJobIn(book: RexBook, jobId: string, plan: RexPlan, at: number): RexBook {
  const job = findJob(book, jobId)
  if (job === undefined || job.plan !== null) {
    return book
  }
  const planned = replaceJob(book, jobId, old => ({ ...old, plan, status: 'planned', updatedAt: at }))
  return queued(planned, at, 'job.plan', job.owner.builder, jobId, {
    orchestrator: plan.orchestrator,
    breakdown: plan.breakdown,
    tasks: tasksData(plan.tasks),
    roles: rolesData(plan.roles),
    manifest: plan.manifest.map(file => ({ skill: file.skill, file: file.file, hash: file.hash, source: file.source })),
  })
}

export type PlanAmendment = { readonly tasks: readonly RexTask[]; readonly roles: readonly RexRole[] }

export function amendPlanIn(book: RexBook, jobId: string, amendment: PlanAmendment, at: number): RexBook {
  const job = findJob(book, jobId)
  if (job?.plan === null || job?.plan === undefined) {
    return book
  }
  const plan = job.plan
  const taskIds = new Set(amendment.tasks.map(task => task.id))
  const roleNames = new Set(amendment.roles.map(role => role.role))
  const amended: RexPlan = {
    ...plan,
    tasks: [...plan.tasks.filter(task => !taskIds.has(task.id)), ...amendment.tasks],
    roles: [...plan.roles.filter(role => !roleNames.has(role.role)), ...amendment.roles],
  }
  const changed = replaceJob(book, jobId, old => ({ ...old, plan: amended, updatedAt: at }))
  return queued(changed, at, 'job.amend', job.owner.builder, jobId, { tasks: tasksData(amendment.tasks), roles: rolesData(amendment.roles) })
}

export type JobClose = {
  readonly status: Exclude<RexJobStatus, 'created' | 'planned'>
  readonly outcome: string
  readonly deliveries: readonly RexDelivery[]
}

export function closeJobIn(book: RexBook, jobId: string, close: JobClose, at: number): RexBook {
  const job = findJob(book, jobId)
  if (job === undefined) {
    return book
  }
  const isSame = job.status === close.status && job.outcome === close.outcome && JSON.stringify(job.deliveries) === JSON.stringify(close.deliveries)
  if (isSame) {
    return book
  }
  const closed = replaceJob(book, jobId, old => ({ ...old, status: close.status, outcome: close.outcome, deliveries: close.deliveries, updatedAt: at }))
  return queued(closed, at, 'job.close', job.owner.builder, jobId, {
    status: close.status,
    outcome: close.outcome,
    deliveries: close.deliveries.map(delivery => ({
      repo: delivery.repo,
      branch: delivery.branch,
      pr: delivery.pr,
      outcome: delivery.outcome,
      merge_commit: delivery.mergeCommit,
      note: delivery.note,
    })),
  })
}

const PERSON_SOURCES: readonly RexMergeAnswer['source'][] = ['dialog', 'person-prompt', 'sdk-prompt', 'question']

export function mergeAnswerIn(book: RexBook, jobId: string, answer: RexMergeAnswer, at: number): RexBook {
  const job = findJob(book, jobId)
  if (job === undefined) {
    return book
  }
  const isPersonHeld = job.merge !== null && PERSON_SOURCES.includes(job.merge.source)
  if (answer.source === 'policy' && isPersonHeld) {
    return book
  }
  const answered = replaceJob(book, jobId, old => ({ ...old, merge: answer, updatedAt: at }))
  return queued(answered, at, 'job.merge_answer', job.owner.builder, jobId, {
    answer: answer.answer,
    source: answer.source,
    question: answer.question,
    policy_hash: answer.policyHash,
  })
}

export function limitIn(book: RexBook, jobId: string, limit: RexLimit, at: number): RexBook {
  const job = findJob(book, jobId)
  if (job === undefined) {
    return book
  }
  const limited = replaceJob(book, jobId, old => ({ ...old, limit, updatedAt: at }))
  return queued(limited, at, 'job.limit', job.owner.builder, jobId, { limit_nano: limit.nano, source: limit.source })
}

export type DecisionRecorded = { readonly book: RexBook; readonly decision: RexDecision }

export function decisionIn(book: RexBook, jobId: string, decision: RexDecision, at: number): DecisionRecorded {
  const job = findJob(book, jobId)
  const stored = job?.decisions.find(old => old.assignmentId === decision.assignmentId)
  if (job === undefined || stored !== undefined) {
    return { book, decision: stored ?? decision }
  }
  const recorded = replaceJob(book, jobId, old => ({ ...old, decisions: [...old.decisions, decision], updatedAt: at }))
  const changed = queued(recorded, at, 'decision', job.owner.builder, jobId, {
    assignment_id: decision.assignmentId,
    subject: decision.subject,
    item: decision.item,
    value: decision.value,
    source: decision.source,
    reason: decision.reason,
    table_version: decision.tableVersion,
    overlay_version: decision.overlayVersion,
  })
  return { book: changed, decision }
}

export type QuestionInput = Omit<RexQuestion, 'id' | 'askedAt' | 'answer'>
export type QuestionAdded = { readonly book: RexBook; readonly question: RexQuestion }

export function questionIn(book: RexBook, input: QuestionInput, at: number): QuestionAdded {
  const question: RexQuestion = { ...input, id: questionIdOf(book.session, book.questions.length + 1), askedAt: at, answer: null }
  const asked = { ...book, questions: [...book.questions, question] }
  const withQuestion = input.mergeFor === null ? asked : replaceJob(asked, input.mergeFor, old => ({ ...old, mergeQuestion: question.id, updatedAt: at }))
  const changed = queued(withQuestion, at, 'question.ask', 'main', input.job ?? input.mergeFor, {
    id: question.id,
    tree: question.tree,
    question: question.question,
    options: question.options,
    recommendation: question.recommendation,
    category: question.category,
    merge_for: question.mergeFor,
  })
  return { book: changed, question }
}

export function answerIn(book: RexBook, id: string, answer: RexAnswer, at: number): RexBook {
  const question = book.questions.find(old => old.id === id)
  if (question === undefined || question.answer !== null) {
    return book
  }
  const answered = { ...book, questions: book.questions.map(old => (old.id === id ? { ...old, answer } : old)) }
  const changed = queued(answered, at, 'question.answer', 'main', question.job ?? question.mergeFor, { id, option: answer.option, source: answer.source })
  if (question.mergeFor === null) {
    return changed
  }
  const merge = answer.option === 'Merge' ? 'yes' : answer.option === 'Keep the branch' ? 'no' : null
  if (merge === null) {
    return changed
  }
  return mergeAnswerIn(changed, question.mergeFor, { answer: merge, source: 'question', question: id, policyHash: null, at }, at)
}

export function rebindIn(book: RexBook, jobId: string, owner: RexOwner, at: number): RexBook {
  const job = findJob(book, jobId)
  if (job === undefined) {
    return book
  }
  const rebound = replaceJob(book, jobId, old => ({ ...old, owner, updatedAt: at }))
  return queued(rebound, at, 'job.rebind', owner.builder, jobId, { owner: { session: owner.session, builder: owner.builder } })
}

export function doneCheckIn(book: RexBook, jobId: string, check: RexDoneCheck, at: number): RexBook {
  const job = findJob(book, jobId)
  if (job === undefined) {
    return book
  }
  const checked = replaceJob(book, jobId, old => ({ ...old, doneChecks: [...old.doneChecks.filter(done => done.check !== check.check), check], updatedAt: at }))
  return queued(checked, at, 'job.done_check', job.owner.builder, jobId, { check: check.check, result: check.result })
}

export type WakeReason = 'stuck' | 'retry'

export function wakeIn(book: RexBook, jobId: string, reason: WakeReason, at: number): RexBook {
  const job = findJob(book, jobId)
  if (job === undefined) {
    return book
  }
  const count = reason === 'stuck' ? job.stuckWakes + 1 : job.retries + 1
  const woken = replaceJob(book, jobId, old => (reason === 'stuck' ? { ...old, stuckWakes: count, updatedAt: at } : { ...old, retries: count, updatedAt: at }))
  return queued(woken, at, 'job.wake', job.owner.builder, jobId, { reason, count })
}

export type MergeDone = { readonly repo: string; readonly base: string; readonly branch: string; readonly mergeCommit: string }

/** A merge that `rex_merge` pushed: recorded as a merged delivery of the job, so its close can name it. */
export function mergeDoneIn(book: RexBook, jobId: string, done: MergeDone, at: number): RexBook {
  const job = findJob(book, jobId)
  if (job === undefined) {
    return book
  }
  const delivery: RexDelivery = { repo: done.repo, branch: done.branch, pr: null, outcome: 'merged', mergeCommit: done.mergeCommit, note: null }
  const isKnown = job.deliveries.some(old => old.outcome === 'merged' && old.mergeCommit === done.mergeCommit)
  const merged = isKnown ? book : replaceJob(book, jobId, old => ({ ...old, deliveries: [...old.deliveries, delivery], updatedAt: at }))
  return queued(merged, at, 'merge.done', job.owner.builder, jobId, { repo: done.repo, base: done.base, merge_commit: done.mergeCommit })
}

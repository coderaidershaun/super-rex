// The book's functions as other modules call them: each reads the clock, then applies one pure change to the session's state with a version check.
import { atom, read, update } from 'claude-code'
import type { EngineInterface } from 'claude-code'

import type {
  RexAnswer,
  RexBook,
  RexDecision,
  RexDoneCheck,
  RexEvent,
  RexJob,
  RexLimit,
  RexLintAttempt,
  RexMergeAnswer,
  RexOwner,
  RexPendingSpawn,
  RexPlan,
  RexQuestion,
  RexTree,
  RexTreeNote,
  RexTreeTick,
  RexVersions,
} from '../../types'
import { acked, emptyBook, nextBatch, withMark, withTranscript } from './events'
import { promptIn, sessionIn } from './session'
import type { PromptRecord, SessionSetup } from './session'
import { failIn } from './fail'
import { failureClass } from './failure'
import type { FailureClass, RexFailure } from './failure'
import { jobIdOf, newStreamKeys, randomNumber } from './ids'
import {
  amendPlanIn,
  answerIn,
  closeJobIn,
  decisionIn,
  doneCheckIn,
  limitIn,
  mergeAnswerIn,
  mergeDoneIn,
  openJobIn,
  planJobIn,
  questionIn,
  rebindIn,
  wakeIn,
} from './jobs'
import type { JobClose, MergeDone, OpenJobInput, PlanAmendment, QuestionInput, WakeReason } from './jobs'
import { endRunIn, lintAttemptIn, runOutputIn, skillOpenIn, startRunIn, stepRunIn } from './runs'
import type { EndInput, SkillOpen, StartRunInput, StepInput } from './runs'
import { linkIn, noteIn, pendingIn, tickIn, treeIn, treeStateIn } from './trees'
import type { TreeInput } from './trees'

type Dollar = EngineInterface

/** Change the tag when the book's shape changes: a reload then declines what older code stored. */
export const BOOK_SHAPE = 'book-2'
export const bookAtom = atom({ plugin: 'super-rex', key: 'book' } as const, emptyBook(), { shape: BOOK_SHAPE })

export function readBook($: Dollar): Promise<RexBook> {
  return read($, bookAtom)
}

async function change($: Dollar, apply: (book: RexBook, at: number) => RexBook): Promise<RexBook> {
  const at = await $.clock.now()
  return update($, bookAtom, book => apply(book, at))
}

/** Runs a change that also answers a value; the value of the try that was written is the one returned. */
async function changeWith<T>($: Dollar, apply: (book: RexBook, at: number) => { readonly book: RexBook; readonly value: T }): Promise<T> {
  const at = await $.clock.now()
  const results: T[] = []
  await update($, bookAtom, book => {
    const result = apply(book, at)
    results.push(result.value)
    return result.book
  })
  const last = results[results.length - 1]
  if (last === undefined) {
    throw new BookWriteError('the book was not written')
  }
  return last
}

export class BookWriteError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'BookWriteError'
  }
}

export type OpenedJob = { readonly job: RexJob; readonly isExisting: boolean }

export async function openJob($: Dollar, input: OpenJobInput): Promise<OpenedJob> {
  const random = randomNumber()
  return changeWith($, (book, at) => {
    const opened = openJobIn(book, input, jobIdOf(at, random), at)
    return { book: opened.book, value: { job: opened.job, isExisting: opened.isExisting } }
  })
}

export async function planJob($: Dollar, job: string, plan: RexPlan): Promise<void> {
  await change($, (book, at) => planJobIn(book, job, plan, at))
}

export async function amendPlan($: Dollar, job: string, amendment: PlanAmendment): Promise<void> {
  await change($, (book, at) => amendPlanIn(book, job, amendment, at))
}

export async function closeJob($: Dollar, job: string, close: JobClose): Promise<void> {
  await change($, (book, at) => closeJobIn(book, job, close, at))
}

export async function startRun($: Dollar, run: StartRunInput): Promise<void> {
  await change($, (book, at) => startRunIn(book, run, at))
}

export async function stepRun($: Dollar, agentId: string, step: StepInput): Promise<void> {
  await change($, (book, at) => stepRunIn(book, agentId, step, at))
}

export async function endRun($: Dollar, agentId: string, end: EndInput): Promise<void> {
  await change($, (book, at) => endRunIn(book, agentId, end, at))
}

/** Keeps the first brief of a lint key; answers the stored attempt when the key was refused before, else null. */
export function recordLintAttempt($: Dollar, attempt: Omit<RexLintAttempt, 'at'>): Promise<RexLintAttempt | null> {
  return changeWith($, (book, at) => {
    const recorded = lintAttemptIn(book, { ...attempt, at })
    return { book: recorded.book, value: recorded.stored }
  })
}

export async function recordMergeDone($: Dollar, job: string, done: MergeDone): Promise<void> {
  await change($, (book, at) => mergeDoneIn(book, job, done, at))
}

export async function recordSkillOpen($: Dollar, agentId: string, open: SkillOpen): Promise<void> {
  await change($, (book, at) => skillOpenIn(book, agentId, open, at))
}

export async function setRunOutput($: Dollar, agentId: string, file: string): Promise<void> {
  await change($, book => runOutputIn(book, agentId, file))
}

export async function markDegraded($: Dollar, job: string, mark: string): Promise<void> {
  await change($, (book, at) => withMark(book, job, mark, at))
}

export function recordDecision($: Dollar, job: string, decision: RexDecision): Promise<RexDecision> {
  return changeWith($, (book, at) => {
    const recorded = decisionIn(book, job, decision, at)
    return { book: recorded.book, value: recorded.decision }
  })
}

export async function setMergeAnswer($: Dollar, job: string, answer: Omit<RexMergeAnswer, 'at'>): Promise<void> {
  await change($, (book, at) => mergeAnswerIn(book, job, { ...answer, at }, at))
}

export async function setLimit($: Dollar, job: string, limit: RexLimit): Promise<void> {
  await change($, (book, at) => limitIn(book, job, limit, at))
}

export function addQuestion($: Dollar, question: QuestionInput): Promise<RexQuestion> {
  return changeWith($, (book, at) => {
    const added = questionIn(book, question, at)
    return { book: added.book, value: added.question }
  })
}

export async function answerQuestion($: Dollar, id: string, answer: Omit<RexAnswer, 'at'>): Promise<void> {
  await change($, (book, at) => answerIn(book, id, { ...answer, at }, at))
}

export async function addPendingSpawn($: Dollar, spawn: Omit<RexPendingSpawn, 'at'>): Promise<void> {
  await change($, (book, at) => pendingIn(book, { ...spawn, at }))
}

export async function rebindJob($: Dollar, job: string, owner: RexOwner): Promise<void> {
  await change($, (book, at) => rebindIn(book, job, owner, at))
}

export async function recordDoneCheck($: Dollar, job: string, check: Omit<RexDoneCheck, 'at'>): Promise<void> {
  await change($, (book, at) => doneCheckIn(book, job, { ...check, at }, at))
}

export async function countWake($: Dollar, job: string, reason: WakeReason): Promise<void> {
  await change($, (book, at) => wakeIn(book, job, reason, at))
}

/** Does the book's part of a failure and shows its status line; the caller does the rest by the class it answers. */
export async function fail($: Dollar, failure: RexFailure): Promise<FailureClass> {
  const keys = newStreamKeys()
  const status = await changeWith($, (book, at) => {
    const failed = failIn(book, failure, at, keys)
    return { book: failed.book, value: failed.status }
  })
  if (status !== null) {
    $.ui.status(status)
  }
  return failureClass(failure.kind)
}

export async function openTree($: Dollar, tree: TreeInput): Promise<void> {
  await change($, (book, at) => treeIn(book, tree, at))
}

export async function linkBuilder($: Dollar, tree: string, job: string, agentId: string): Promise<void> {
  await change($, (book, at) => linkIn(book, tree, job, agentId, at))
}

export async function noteTree($: Dollar, tree: string, note: Omit<RexTreeNote, 'at'>): Promise<void> {
  await change($, (book, at) => noteIn(book, tree, { ...note, at }, at))
}

export async function tickTree($: Dollar, tree: string, tick: Omit<RexTreeTick, 'at'>): Promise<void> {
  await change($, (book, at) => tickIn(book, tree, { ...tick, at }))
}

export async function closeTree($: Dollar, tree: string, state: RexTree['state'], reportFile: string | null): Promise<void> {
  await change($, (book, at) => treeStateIn(book, tree, state, reportFile, at))
}

/** The waiting events that fit in one batch, with the stream they go on. */
export type Batch = { readonly session: string; readonly stream: string; readonly secret: string; readonly events: readonly RexEvent[] }

export async function nextEvents($: Dollar, max: number): Promise<Batch | null> {
  const book = await readBook($)
  if (book.ingest === null || book.storage === 'off') {
    return null
  }
  return { session: book.session, stream: book.ingest.stream, secret: book.ingest.secret, events: nextBatch(book, max) }
}

export async function ackEvents($: Dollar, lastSequence: number): Promise<void> {
  await change($, book => acked(book, lastSequence))
}

export async function recordPrompt($: Dollar, prompt: PromptRecord): Promise<void> {
  await change($, (book, at) => promptIn(book, prompt, at))
}

export async function setPromptMode($: Dollar, mode: RexBook['prompts']): Promise<void> {
  await change($, book => ({ ...book, prompts: mode }))
}

/** Counts a permission ask the engine raised this session, for the band. */
export async function countPermissionAsk($: Dollar): Promise<void> {
  await change($, book => ({ ...book, permissionAsks: book.permissionAsks + 1 }))
}

export async function setVersions($: Dollar, versions: Partial<RexVersions>): Promise<void> {
  await change($, book => ({ ...book, versions: { ...book.versions, ...versions } }))
}

export async function noteTranscript($: Dollar, path: string): Promise<void> {
  await change($, book => withTranscript(book, path))
}

/**
 * Starts the book for this session, or keeps it on a reload of the same
 * session, whose stream also stays.
 */
export async function setUpSession($: Dollar, setup: SessionSetup): Promise<RexBook> {
  const keys = newStreamKeys()
  return change($, (old, at) => sessionIn(old, setup, at, keys))
}

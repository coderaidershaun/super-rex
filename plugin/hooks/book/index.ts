// The live record of jobs, runs, questions and master trees, kept in the session's state so a reload loses nothing; the only writer of that record.
import type { On } from 'claude-code'

import type { Parts } from '../parts'
import { fail, setUpSession } from './api'
import { pluginVersion, readConfig } from './config'

export {
  ackEvents,
  addPendingSpawn,
  addQuestion,
  amendPlan,
  answerQuestion,
  applyAutoMerge,
  bookWrites,
  closeJob,
  closeTree,
  dropTreeStart,
  countPermissionAsk,
  countWake,
  endRun,
  fail,
  linkBuilder,
  markDegraded,
  nextEvents,
  noteTranscript,
  noteTree,
  openJob,
  openTree,
  planJob,
  readBook,
  rebindJob,
  recordDecision,
  recordDoneCheck,
  recordLintAttempt,
  recordMergeDone,
  recordPrompt,
  recordSkillOpen,
  repinTree,
  resetWakes,
  restoreTree,
  setLimit,
  setMergeAnswer,
  setPromptMode,
  setRunOutput,
  setTreeLimits,
  setVersions,
  startRun,
  stepMain,
  stepRun,
  tickTree,
} from './api'
export type { Batch } from './api'
export type { PromptRecord } from './session'
export { bandJob, isOpen, mainJob, newestOpenJob, openJobs } from './events'
export { assertNever, errorText, failureClass } from './failure'
export type { FailureClass, RexFailure } from './failure'
export { jobWorktree } from './jobs'
export type { JobClose, MergeDone, OpenJobInput, PlanAmendment, QuestionInput, Restored, WakeReason } from './jobs'
export { mainTree } from './runs'
export type { EndInput, SkillOpen, StartRunInput, StepInput } from './runs'
export { MAIN_MASTER } from './trees'
export type { TreeInput, TreeLimits } from './trees'
export { pluginVersion } from './config'
export { limitIgnoredLine, MAX_LIMIT_USD, readPersonWords } from './words'

export function register(on: On, _parts: Parts): void {
  on('session.start', async ($, e, next) => {
    const root = await $.session.root()
    const session = await $.session.id()
    const read = await readConfig($, root)
    const engine = await $.session.version()
    const plugin = await pluginVersion($)
    const config = read.kind === 'read' ? read.config : { storage: 'off' as const, jobsFromSdk: true, autoMerge: false }
    const inert = read.kind === 'missing' ? 'no .rex/config.json in the session root' : null
    await setUpSession($, { session, storage: config.storage, jobsFromSdk: config.jobsFromSdk, autoMerge: config.autoMerge, inert, claudeCode: engine.version, plugin })
    if (read.kind === 'unreadable') {
      await fail($, { kind: 'config-unreadable', reason: read.reason })
    }
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'session.start', module: 'book', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })
}

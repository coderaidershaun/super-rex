// The live snapshot: the book's view for other programs, in the form of fixtures/shared/live/snapshot.json, which `rex watch` reads with no database.
// Pure, and it names each field it keeps: the ingest stream and its secret, prompt texts and briefs never reach the file. A tree's spend is all it spent against its cap.
import type { RexBook, RexJob, RexRun, RexTokens } from '../../types'
import { treeSpentNano } from '../master/gates'
import { cut } from '../pane/outline'

type Json = string | number | boolean | null | readonly Json[] | { readonly [key: string]: Json }
type JsonObject = { readonly [key: string]: Json }

/** A reader shows a charter or a request in a few lines, so a pasted spec of many pages stays out of a file written every few seconds. */
const TEXT_MAX = 1_000

function iso(ms: number): string {
  return new Date(ms).toISOString()
}

function isoOrNull(ms: number | null): string | null {
  return ms === null ? null : iso(ms)
}

function tokens(counts: RexTokens): JsonObject {
  return { input: counts.input, output: counts.output, cache_read: counts.cacheRead, cache_creation: counts.cacheCreation }
}

function jobOf(job: RexJob): JsonObject {
  return {
    id: job.id,
    tree: job.tree,
    builder: job.owner.builder,
    origin: job.origin,
    status: job.status,
    request: cut(job.request, TEXT_MAX),
    type: job.type,
    size: job.size,
    consequence: job.consequence,
    langs: job.langs,
    orchestrator: job.plan?.orchestrator ?? null,
    merge: job.merge?.answer ?? null,
    merge_source: job.merge?.source ?? null,
    repo: job.repo,
    branch: job.branch,
    base: job.base,
    worktree: job.worktree,
    limit_nano: job.limit.nano,
    spent_nano: job.spentNano,
    tokens: tokens(job.tokens),
    degraded: job.degraded,
    deliveries: job.deliveries.map(delivery => ({ repo: delivery.repo, outcome: delivery.outcome, merge_commit: delivery.mergeCommit, note: delivery.note })),
    done_checks: job.doneChecks.map(check => ({ check: check.check, result: check.result, at: iso(check.at) })),
    outcome: job.outcome,
    created_at: iso(job.createdAt),
    updated_at: iso(job.updatedAt),
  }
}

/** The book's role, except that the run of a tree's master is `master` and the run of a job's builder is `builder`. */
function roleOf(book: RexBook, run: RexRun): string | null {
  if (book.trees.some(tree => tree.master === run.agentId)) {
    return 'master'
  }
  if (book.jobs.some(job => job.id === run.job && job.owner.builder === run.agentId)) {
    return 'builder'
  }
  return run.role
}

function runOf(book: RexBook, run: RexRun): JsonObject {
  return {
    agent_id: run.agentId,
    job: run.job,
    tree: run.tree,
    parent: run.parent,
    depth: run.depth,
    role: roleOf(book, run),
    task: run.task,
    agent_type: run.agentType,
    planned: run.planned,
    model: run.ran?.model ?? null,
    effort: run.ran?.effort ?? null,
    state: run.state,
    end_reason: run.endReason,
    started_at: iso(run.startedAt),
    ended_at: isoOrNull(run.endedAt),
    tokens: tokens(run.tokens),
    spent_nano: run.spentNano,
    marks: run.marks,
    lint: run.lint,
    skills_attached: run.skillsAttached,
    skills_opened: run.skillsOpened,
    output_file: run.outputFile,
  }
}

export function snapshotOf(book: RexBook, now: number): JsonObject {
  return {
    version: 1,
    session: book.session,
    written_at: iso(now),
    storage: book.storage,
    versions: { mod: book.versions.mod, binary: book.versions.binary, claude_code: book.versions.claudeCode },
    trees: book.trees.map(tree => ({
      id: tree.id,
      state: tree.state,
      master: tree.master,
      charter: cut(tree.charter, TEXT_MAX),
      spent_nano: treeSpentNano(book, tree),
      cap_nano: tree.capNano,
      started_at: iso(tree.startedAt),
      closed_at: isoOrNull(tree.closedAt),
      report_file: tree.reportFile,
      notes: tree.notes.map(note => ({ at: iso(note.at), job: note.job, text: note.text })),
    })),
    jobs: book.jobs.map(jobOf),
    runs: book.runs.map(run => runOf(book, run)),
    questions: book.questions.map(question => ({
      id: question.id,
      tree: question.tree,
      job: question.job,
      question: question.question,
      options: question.options,
      recommendation: question.recommendation,
      category: question.category,
      asked_at: iso(question.askedAt),
      answer: question.answer === null ? null : { option: question.answer.option, source: question.answer.source, at: iso(question.answer.at) },
    })),
  }
}

// rex_job_open: the person's job-start prompt or a master's builder opens one job, whose places are fixed here from git and never taken from an argument, and whose worktree the mod makes here.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob, RexLevel } from '../../types'
import { closeJob, fail, openJob, readBook } from '../book/index'
import { argsOf, isRefused, optionalObject, optionalTextList, optionalWord, textArg } from './args'
import type { Args, Refused } from './args'
import { callerIn } from './caller'
import { defaultLimitNano } from './limits'
import { firstCommit, firstCommitRefusal, repoPlaces, uncommittedCount } from './places'
import type { RepoPlaces } from './places'
import { answer, deny, usd } from './text'
import { addWorktree, baseStart } from './worktree'

const TOOL = 'rex_job_open'
const LEVELS: readonly RexLevel[] = ['low', 'medium', 'high']
const TYPES = ['feature', 'fix', 'refactor', 'chore', 'research', 'other'] as const
const SIZES = ['small', 'medium', 'large'] as const

/** The answer of rex_job_open: the job's record with its stored classification and preferences (a builder a master started takes them from here), then the repo's mode and the changes in the main checkout that the job does not see, both read from git now. */
async function jobLines($: EngineInterface, book: RexBook, job: RexJob, places: RepoPlaces, isExisting: boolean, isCommitMade: boolean): Promise<readonly string[]> {
  const uncommitted = await uncommittedCount($, job.repo)
  return [
    `job=${job.id}`,
    `status=${job.status}`,
    `existing=${isExisting ? 'yes' : 'no'}`,
    `merge=${job.merge?.answer ?? 'unknown'}`,
    `storage=${book.storage}`,
    `limit_usd=${usd(job.limit.nano)}`,
    `type=${job.type ?? 'none'}`,
    `size=${job.size ?? 'none'}`,
    `consequence=${job.consequence ?? 'none'}`,
    `langs=${job.langs.length === 0 ? 'none' : job.langs.join(',')}`,
    `duration=${job.prefs.duration ?? 'none'}`,
    `budget=${job.prefs.budget ?? 'none'}`,
    `autonomy=${job.prefs.autonomy ?? 'none'}`,
    `repo=${job.repo}`,
    `branch=${job.branch}`,
    `base=${job.base}`,
    `remote=${places.remote}`,
    `uncommitted=${String(uncommitted)}`,
    ...(isCommitMade ? ['base_commit=made'] : []),
    `worktree=${job.worktree ?? ''}`,
  ]
}

type OpenArgs = {
  readonly request: string
  readonly type: RexJob['type']
  readonly size: RexJob['size']
  readonly consequence: RexLevel | null
  readonly langs: readonly string[]
  readonly prefs: RexJob['prefs']
}

function prefsOf(args: Args | null): RexJob['prefs'] | Refused {
  if (args === null) {
    return { duration: null, budget: null, autonomy: null }
  }
  const duration = optionalWord(`${TOOL} prefs`, args, 'duration', LEVELS)
  const budget = optionalWord(`${TOOL} prefs`, args, 'budget', LEVELS)
  const autonomy = optionalWord(`${TOOL} prefs`, args, 'autonomy', LEVELS)
  if (isRefused(duration)) return duration
  if (isRefused(budget)) return budget
  if (isRefused(autonomy)) return autonomy
  return { duration, budget, autonomy }
}

function openArgs(args: Args): OpenArgs | Refused {
  const request = textArg(TOOL, args, 'request', "call it again with the person's request as text")
  const type = optionalWord(TOOL, args, 'type', TYPES)
  const size = optionalWord(TOOL, args, 'size', SIZES)
  const consequence = optionalWord(TOOL, args, 'consequence', LEVELS)
  const langs = optionalTextList(TOOL, args, 'langs')
  const prefsArg = optionalObject(TOOL, args, 'prefs')
  const prefs = isRefused(prefsArg) ? prefsArg : prefsOf(prefsArg)
  if (isRefused(request)) return request
  if (isRefused(type)) return type
  if (isRefused(size)) return size
  if (isRefused(consequence)) return consequence
  if (isRefused(langs)) return langs
  if (isRefused(prefs)) return prefs
  return { request, type, size, consequence, langs, prefs }
}

/** The session root's real path, as the repo of the job. */
async function sessionRepo($: EngineInterface): Promise<string> {
  const root = await $.session.root()
  const stat = await $.fs.stat(root, { resolve: true })
  return stat.realPath ?? root
}

export async function serveJobOpen($: EngineInterface, event: { readonly agentId?: string }): Promise<ToolCallResult> {
  const book = await readBook($)
  const caller = callerIn(book, event.agentId)
  if (caller.kind === 'builder' && caller.job !== null) {
    return answer(await jobLines($, book, caller.job, await repoPlaces($, caller.job.repo), true, false))
  }
  if (caller.kind === 'master') {
    // The main loop is a master here: it starts builders with rex_build, and builds no job of its own beside its tree.
    return deny(`${TOOL}: the main loop runs a master tree in this session, got a job start; end the tree with rex_report first, or start the job in another session`)
  }
  if (caller.kind !== 'main' || book.jobStartPrompt === null) {
    await fail($, { kind: 'origin-not-person', action: TOOL, origin: caller.kind })
    return deny(`${TOOL}: only a builder opens a job: the main loop right after your /orchestration-builder prompt, or a builder that rex_build started; you are ${caller.kind}`)
  }
  const start = book.jobStartPrompt
  const parsed = openArgs(argsOf(event))
  if (isRefused(parsed)) {
    await fail($, { kind: 'bad-argument', tool: TOOL, argument: 'arguments', value: parsed.refused })
    return deny(parsed.refused)
  }
  const repo = await sessionRepo($)
  const made = await firstCommit($, repo)
  const noCommit = firstCommitRefusal(TOOL, repo, made, 'start')
  if (noCommit !== null) {
    return deny(noCommit)
  }
  const places = await repoPlaces($, repo)
  const from = await baseStart($, places)
  if (from.kind === 'failed') {
    return deny(`${TOOL}: ${from.command} failed for ${repo}, got exit ${String(from.exitCode)}: ${from.firstError}; make sure git -C ${repo} ${from.command.slice('git '.length)} works, then start the job again`)
  }
  const opened = await openJob($, {
    builder: 'main',
    tree: null,
    origin: start.origin,
    request: parsed.request,
    repo: places.repo,
    base: places.base,
    type: parsed.type,
    size: parsed.size,
    consequence: parsed.consequence,
    langs: parsed.langs,
    prefs: parsed.prefs,
    limit: { nano: defaultLimitNano(), source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  })
  const job = opened.job
  const added = opened.isExisting || job.worktree === null ? null : await addWorktree($, { repo: job.repo, branch: job.branch, worktree: job.worktree }, from.commit)
  if (added !== null) {
    await closeJob($, job.id, { status: 'cancelled', outcome: `${added.command} failed: ${added.firstError}`, deliveries: [] })
    return deny(`${TOOL}: ${added.command} failed for job ${job.id}, got exit ${String(added.exitCode)}: ${added.firstError}; the job is cancelled, start the job again`)
  }
  return answer([...(await jobLines($, await readBook($), job, places, opened.isExisting, made.kind === 'made')), ...(from.isStale ? ['base_stale=yes'] : [])])
}

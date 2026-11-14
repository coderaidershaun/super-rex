// rex_job_open: the person's job-start prompt or a master's builder opens one job, whose places are fixed here and never taken from an argument.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob, RexLevel } from '../../types'
import { fail, openJob, readBook } from '../book/index'
import { argsOf, isRefused, optionalObject, optionalTextList, optionalWord, textArg } from './args'
import type { Args, Refused } from './args'
import { callerIn } from './caller'
import { defaultLimitNano } from './limits'
import { answer, deny, usd } from './text'

const TOOL = 'rex_job_open'
const LEVELS: readonly RexLevel[] = ['low', 'medium', 'high']
const TYPES = ['feature', 'fix', 'refactor', 'chore', 'research', 'other'] as const
const SIZES = ['small', 'medium', 'large'] as const

export function jobLines(book: RexBook, job: RexJob, isExisting: boolean): readonly string[] {
  return [
    `job=${job.id}`,
    `status=${job.status}`,
    `existing=${isExisting ? 'yes' : 'no'}`,
    `merge=${job.merge?.answer ?? 'unknown'}`,
    `storage=${book.storage}`,
    `limit_usd=${usd(job.limit.nano)}`,
    `repo=${job.repo}`,
    `branch=${job.branch}`,
    `base=${job.base}`,
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

/** The session root's real path, and the branch that origin's HEAD names, else `main`. */
async function jobPlaces($: EngineInterface): Promise<{ readonly repo: string; readonly base: string }> {
  const root = await $.session.root()
  const stat = await $.fs.stat(root, { resolve: true })
  const repo = stat.realPath ?? root
  const ran = await $.process.run(['git', '-C', repo, 'symbolic-ref', '--short', 'refs/remotes/origin/HEAD'], { cwd: repo })
  const named = ran.exitCode === 0 ? ran.stdout.trim() : ''
  return { repo, base: named.startsWith('origin/') ? named.slice('origin/'.length) : 'main' }
}

export async function serveJobOpen($: EngineInterface, event: { readonly agentId?: string }): Promise<ToolCallResult> {
  const book = await readBook($)
  const caller = callerIn(book, event.agentId)
  if (caller.kind === 'builder' && caller.job !== null) {
    return answer(jobLines(book, caller.job, true))
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
  const places = await jobPlaces($)
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
    limit: { nano: defaultLimitNano(parsed.size, parsed.prefs.budget), source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  })
  return answer(jobLines(await readBook($), opened.job, opened.isExisting))
}

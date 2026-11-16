// rex_build: the master of a running tree opens one job and starts its builder in the job's own worktree, after every policy rule passed.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob, RexLevel, RexLimit, RexRating, RexRatings, RexTree } from '../../types'
import { addPendingSpawn, closeJob, fail, jobWorktree, openJob, readBook, rebindJob, setMergeAnswer } from '../book/index'
import { addWorktree, answer, baseRef, baseStart, callerOf, callerRefusal, defaultLimitNano, deny, firstCommit, firstCommitRefusal, git, hasWorktree, removeWorktree, repoPlaces, runOms } from '../tools/index'
import type { RepoPlaces } from '../tools/index'
import { BUILDER_BRIEF_FILE, readPluginFile } from './agent-type'
import { field, isRefused, optionalNumber, optionalObject, optionalText, refused, text, textList, word } from './args'
import type { Refused } from './args'
import { builderBrief } from './brief'
import { BUILD_TOOL } from './entries'
import { maxJobsRefusal, rateLimitRefusal, treeLedgerRefusal } from './gates'
import { masterMoved, wakeAfterRateLimit } from './idle'
import { isTier, levelRank, tierRank } from './policy'
import type { Policy, Tier } from './policy'
import { pinnedPolicy, treeOfMaster } from './pin'
import { realRoot } from './store'
import { spawnOwn } from './spawn'

const TOOL = 'rex_build'
const LEVELS: readonly RexLevel[] = ['low', 'medium', 'high']
const RATINGS: readonly RexRating[] = ['low', 'medium', 'high', 'none']
const SIZES = ['small', 'medium', 'large'] as const
const TYPES = ['feature', 'fix', 'refactor', 'chore', 'research', 'other'] as const
const MAX_USD = 9_000_000
/** A tree job runs with no person to ask, which `rex oms` reads as high autonomy. */
const TREE_PREFS: RexJob['prefs'] = { duration: null, budget: null, autonomy: 'high' }

type BuildArgs = {
  readonly request: string
  readonly type: NonNullable<RexJob['type']>
  readonly size: NonNullable<RexJob['size']>
  readonly consequence: RexLevel
  readonly langs: readonly string[]
  readonly repo: string | null
  readonly dependsOn: readonly string[]
  readonly mergeAfter: readonly string[]
  readonly budgetUsd: number | null
  readonly ratings: RexRatings
}

function ratingsOf(given: object | null, consequence: RexLevel): RexRatings | Refused {
  if (given === null) {
    return { intelligence: 'high', reliability: 'medium', consequence, verification: 'medium' }
  }
  const intelligence = word(`${TOOL} ratings`, given, 'intelligence', RATINGS)
  const reliability = word(`${TOOL} ratings`, given, 'reliability', RATINGS)
  const rated = word(`${TOOL} ratings`, given, 'consequence', RATINGS)
  const verification = word(`${TOOL} ratings`, given, 'verification', RATINGS)
  if (isRefused(intelligence)) return intelligence
  if (isRefused(reliability)) return reliability
  if (isRefused(rated)) return rated
  if (isRefused(verification)) return verification
  return { intelligence, reliability, consequence: rated, verification }
}

/** The languages the job changes, as lower-case words; a job that changes no code gives []. */
function langsOf(event: object): readonly string[] | Refused {
  const value = field(event, 'langs')
  const isWordList = Array.isArray(value) && value.every((item: unknown) => typeof item === 'string' && item.trim() !== '')
  return isWordList ? value.map(item => String(item).trim().toLowerCase()) : refused(TOOL, 'langs must be a list of languages', value, 'call it again with langs such as ["rust"], or [] for none')
}

function buildArgs(event: object): BuildArgs | Refused {
  const request = text(TOOL, event, 'request', 'call it again with the request of the job')
  const type = word(TOOL, event, 'type', TYPES)
  const size = word(TOOL, event, 'size', SIZES)
  const consequence = word(TOOL, event, 'consequence', LEVELS)
  const langs = langsOf(event)
  const repo = optionalText(TOOL, event, 'repo')
  const dependsOn = textList(TOOL, event, 'depends_on')
  const mergeAfter = textList(TOOL, event, 'merge_after')
  const budgetUsd = optionalNumber(TOOL, event, 'budget_usd', MAX_USD)
  const given = optionalObject(TOOL, event, 'ratings')
  if (isRefused(request)) return request
  if (isRefused(type)) return type
  if (isRefused(size)) return size
  if (isRefused(consequence)) return consequence
  if (isRefused(langs)) return langs
  if (isRefused(repo)) return repo
  if (isRefused(dependsOn)) return dependsOn
  if (isRefused(mergeAfter)) return mergeAfter
  if (isRefused(budgetUsd)) return budgetUsd
  if (isRefused(given)) return given
  const ratings = ratingsOf(given, consequence)
  if (isRefused(ratings)) return ratings
  return { request, type, size, consequence, langs, repo, dependsOn, mergeAfter, budgetUsd, ratings }
}

/** The limit of a tree job: the master's budget up to `per_job_usd`, else `per_job_usd`, else the default by size. */
function limitOf(policy: Policy, args: BuildArgs): RexLimit | Refused {
  const cap = policy.per_job_usd
  if (args.budgetUsd !== null && cap !== null && args.budgetUsd > cap) {
    return refused(TOOL, `budget_usd must be at most per_job_usd of the policy, ${String(cap)}`, args.budgetUsd, 'call it again with a smaller budget or none')
  }
  const usd = args.budgetUsd ?? cap
  return usd === null ? { nano: defaultLimitNano(args.size, null), source: 'default' } : { nano: Math.round(usd * 1e9), source: 'policy' }
}

/** A grid agent above the policy's tier cap runs at the cap, with its effort kept. */
export function cappedAgent(agent: string, cap: Tier): string {
  const grid = /^model-([a-z]+)-effort-([a-z]+)$/.exec(agent)
  const tier = grid?.[1] ?? ''
  if (grid === null || !isTier(tier) || tierRank(tier) <= tierRank(cap)) {
    return agent
  }
  return `model-${cap}-effort-${grid[2] ?? ''}`
}

/** A closed check that throws refuses, and says which check it was. */
async function closedCheck($: EngineInterface, check: string, run: () => Promise<string | null>): Promise<string | null> {
  try {
    return await run()
  } catch (error) {
    const name = error instanceof Error ? error.name : 'Error'
    await fail($, { kind: 'authority-check-failed', check, error: error instanceof Error ? `${name}: ${error.message}` : String(error) })
    return `rex gate: ${check} failed (${name}); refused`
  }
}

async function gateRefusal($: EngineInterface, book: RexBook, tree: RexTree, policy: Policy, args: BuildArgs): Promise<string | null> {
  const ledger = await closedCheck($, 'tree-ledger', () => Promise.resolve(treeLedgerRefusal(book, tree)))
  if (ledger !== null) {
    await fail($, { kind: 'tree-ledger', tree: tree.id, spentNano: tree.ledgerNano, capNano: tree.capNano })
    return ledger
  }
  const rate = await closedCheck($, 'rate-limit-pause', async () => rateLimitRefusal((await $.session.usage()).rateLimits, policy))
  if (rate !== null) {
    // The refusal tells the master to end its turn; with no job open nothing else would wake it.
    wakeAfterRateLimit($, tree, policy)
    return rate
  }
  return closedCheck($, 'max-jobs', () => Promise.resolve(maxJobsRefusal(book, tree, policy, { consequence: args.consequence, size: args.size })))
}

function dependsRefusal(book: RexBook, dependsOn: readonly string[]): string | null {
  for (const id of dependsOn) {
    const job = book.jobs.find(each => each.id === id)
    if (job?.status !== 'delivered') {
      return `${TOOL}: depends_on job ${id} is not delivered, got ${job?.status ?? 'no such job'}; build this job after that one delivers`
    }
  }
  return null
}

/** The repo of the job: the session root, or a path the policy names, each as the file system resolves it. */
async function repoOf($: EngineInterface, policy: Policy, asked: string | null): Promise<string | Refused> {
  const root = await realRoot($)
  if (asked === null) {
    return root
  }
  const resolve = async (path: string): Promise<string> => (await $.fs.stat(path.startsWith('/') ? path : `${root}/${path}`, { resolve: true })).realPath ?? path
  const wanted = await resolve(asked)
  const allowed = [root, ...(await Promise.all(policy.repos.map(resolve)))]
  return allowed.includes(wanted) ? wanted : refused(TOOL, `repo ${asked} is not the session root or a repo of the policy`, asked, 'add it with /rex policy set repos')
}

/** The policy's merge answer for a new job; `ask` leaves the person to answer. */
async function setPolicyAnswer($: EngineInterface, job: RexJob, policy: Policy, consequence: RexLevel, policyHash: string): Promise<void> {
  if (policy.merge === 'never') {
    await setMergeAnswer($, job.id, { answer: 'no', source: 'policy', question: null, policyHash })
  }
  if (policy.merge === 'allowed_if_checks_pass' && levelRank(consequence) <= levelRank(policy.merge_max_consequence)) {
    await setMergeAnswer($, job.id, { answer: 'yes', source: 'policy', question: null, policyHash })
  }
}

async function cancel($: EngineInterface, job: RexJob, outcome: string): Promise<void> {
  await closeJob($, job.id, { status: 'cancelled', outcome, deliveries: [] })
}

/** Spawns the builder of a job in its worktree with the fixed brief; a restart tells it to go on from its record. */
async function spawnBuilder($: EngineInterface, job: RexJob, tree: RexTree, agent: string, request: string, worktree: string, isRestart: boolean, places: RepoPlaces) {
  const description = `rex builder ${job.id}`
  await addPendingSpawn($, { description, kind: 'builder', tree: tree.id, job: job.id })
  const prompt = builderBrief(await readPluginFile($, BUILDER_BRIEF_FILE), job, request, isRestart, baseRef(places))
  return spawnOwn($, { subagentType: `super-rex:${agent}`, description, prompt, cwd: worktree })
}

type Started = { readonly places: RepoPlaces; readonly commit: string; readonly isStale: boolean; readonly isCommitMade: boolean }

async function startBuilder($: EngineInterface, job: RexJob, tree: RexTree, agent: string, request: string, started: Started): Promise<ToolCallResult> {
  const worktree = job.worktree ?? jobWorktree(job.repo, job.id)
  const added = await addWorktree($, { repo: job.repo, branch: job.branch, worktree }, started.commit)
  if (added !== null) {
    await cancel($, job, `${added.command} failed: ${added.firstError}`)
    return deny(`${TOOL}: ${added.command} failed for job ${job.id}, got exit ${String(added.exitCode)}: ${added.firstError}; the job is cancelled, call rex_build again`)
  }
  const spawned = await spawnBuilder($, job, tree, agent, request, worktree, false, started.places)
  if (spawned.deny !== undefined || spawned.agentId === undefined) {
    const reason = spawned.deny ?? 'no agent id'
    await cancel($, job, `the builder did not start: ${reason}`)
    await removeWorktree($, job.repo, worktree)
    return deny(`${TOOL}: the builder of job ${job.id} did not start, got ${reason}; the job is cancelled, call rex_build again`)
  }
  const notes = [...(started.isCommitMade ? ['base_commit=made'] : []), ...(started.isStale ? ['base_stale=yes'] : [])]
  return answer([`job=${job.id}`, `builder=${spawned.agentId}`, 'status=created', `worktree=${worktree}`, ...notes])
}

export type Restarted = { readonly agentId: string } | { readonly refused: string }

/** Where a restarted builder's worktree starts: the tip of its job branch, which holds the job's work, else the base when the branch is gone too. */
async function restartStart($: EngineInterface, places: RepoPlaces, branch: string): ReturnType<typeof baseStart> {
  const tip = await git($, places.repo, ['rev-parse', '--verify', '--quiet', `refs/heads/${branch}`])
  return tip.exitCode === 0 && tip.stdout !== '' ? { kind: 'start', commit: tip.stdout, isStale: false } : baseStart($, places)
}

/**
 * A new builder for an open job of a resumed tree, whose builder died with the
 * last process: the same job and worktree, the grid agent of the default
 * ratings, and the owner key moved to this session. Nothing is cancelled when
 * it fails: the job and its work stay for the master or the person.
 */
export async function restartBuilder($: EngineInterface, job: RexJob, tree: RexTree, policy: Policy): Promise<Restarted> {
  const ratings = ratingsOf(null, job.consequence ?? 'medium')
  if (isRefused(ratings)) {
    return { refused: ratings.refused }
  }
  const oms = await runOms($, { prefs: TREE_PREFS, ratings })
  if (!oms.ok) {
    return { refused: `rex oms gave no grid agent, got ${oms.line}` }
  }
  const worktree = job.worktree ?? jobWorktree(job.repo, job.id)
  const places = await repoPlaces($, job.repo)
  if (!(await hasWorktree($, job.repo, worktree))) {
    const start = await restartStart($, places, job.branch)
    const added = start.kind === 'failed' ? start : await addWorktree($, { repo: job.repo, branch: job.branch, worktree }, start.commit)
    if (added !== null) {
      return { refused: `${added.command} failed, got exit ${String(added.exitCode)}: ${added.firstError}` }
    }
  }
  const spawned = await spawnBuilder($, job, tree, cappedAgent(oms.agent, policy.tier_cap), job.request, worktree, true, places)
  if (spawned.deny !== undefined || spawned.agentId === undefined) {
    return { refused: `the builder did not start, got ${spawned.deny ?? 'no agent id'}` }
  }
  await rebindJob($, job.id, { session: (await readBook($)).session, builder: spawned.agentId })
  return { agentId: spawned.agentId }
}

export async function serveBuild($: EngineInterface, event: { readonly agentId?: string }): Promise<ToolCallResult> {
  const caller = await callerOf($, event.agentId)
  const book = await readBook($)
  const tree = caller.kind === 'master' ? treeOfMaster(book, caller.agentId) : undefined
  if (tree === undefined) {
    await fail($, { kind: 'caller-not-allowed', tool: TOOL, caller: caller.kind })
    return deny(callerRefusal(BUILD_TOOL, caller) ?? `${TOOL}: only the master of a running tree may call it, got the master of a closed tree; start a tree with /rex master`)
  }
  const pinned = await pinnedPolicy($, tree)
  if (pinned.kind === 'refused') {
    return deny(pinned.text)
  }
  const { policy, hash } = pinned
  const args = buildArgs(event)
  if (isRefused(args)) {
    await fail($, { kind: 'bad-argument', tool: TOOL, argument: 'arguments', value: args.refused })
    return deny(args.refused)
  }
  const gate = await gateRefusal($, book, tree, policy, args)
  if (gate !== null) {
    return deny(gate)
  }
  const waiting = dependsRefusal(book, args.dependsOn)
  const limit = limitOf(policy, args)
  const repo = waiting === null && !isRefused(limit) ? await repoOf($, policy, args.repo) : null
  const problem = waiting ?? (isRefused(limit) ? limit.refused : null) ?? (isRefused(repo) ? repo.refused : null)
  if (problem !== null || repo === null || isRefused(repo) || isRefused(limit)) {
    await fail($, { kind: 'bad-argument', tool: TOOL, argument: 'arguments', value: problem ?? '' })
    return deny(problem ?? `${TOOL}: the job could not be placed; call it again`)
  }
  const oms = await runOms($, { prefs: TREE_PREFS, ratings: args.ratings })
  if (!oms.ok) {
    return deny(`${TOOL}: rex oms gave no grid agent, got ${oms.line}; call it again`)
  }
  const made = await firstCommit($, repo)
  const noCommit = firstCommitRefusal(TOOL, repo, made, 'call')
  if (noCommit !== null) {
    return deny(noCommit)
  }
  const places = await repoPlaces($, repo)
  const start = await baseStart($, places)
  if (start.kind === 'failed') {
    return deny(`${TOOL}: ${start.command} failed for ${repo}, got exit ${String(start.exitCode)}: ${start.firstError}; ask the person to make sure git -C ${repo} ${start.command.slice('git '.length)} works, then call rex_build again`)
  }
  const opened = await openJob($, {
    builder: 'pending',
    tree: tree.id,
    origin: 'master',
    request: args.request,
    repo,
    base: places.base,
    type: args.type,
    size: args.size,
    consequence: args.consequence,
    langs: args.langs,
    prefs: TREE_PREFS,
    limit,
    dependsOn: args.dependsOn,
    mergeAfter: args.mergeAfter,
  })
  await setPolicyAnswer($, opened.job, policy, args.consequence, hash)
  const started = await startBuilder($, opened.job, tree, cappedAgent(oms.agent, policy.tier_cap), args.request, { places, commit: start.commit, isStale: start.isStale, isCommitMade: made.kind === 'made' })
  if (started.deny === undefined) {
    masterMoved(tree.id)
  }
  return started
}

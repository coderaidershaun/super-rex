// Proves rex_build: only the tree's master builds a classified job, every policy rule refuses before anything opens, and the builder starts in its own worktree with the fixed brief, linked to its job.
import { expect, test } from 'claude-code/testing'

import type { ProcessCall } from '../book/kit'
import { ROOT } from '../book/kit'
import type { Policy } from './policy'
import { BUILDER_TEXT, buildRules, byRule, confirmedPolicy, firstCommitRules, KIND, lineOf, MASTER, masterWorld, POLICY_KEY, ran, resultText, settleLong, settleUntil, START, START_COMMIT, TREE, treeBook } from './master-kit'
import type { MasterOptions, Rule } from './master-kit'

const BUILD = 'mcp__super-rex__rex_build' as const
const MINUTE = 60_000
/** The git calls that read the places of the session root in mode `origin`: for the job's base, for its worktree, and for the base the brief names. */
const PLACES = [
  ['git', '-C', ROOT, 'symbolic-ref', '--quiet', '--short', 'HEAD'],
  ['git', '-C', ROOT, 'remote'],
  ['git', '-C', ROOT, 'symbolic-ref', '--short', 'refs/remotes/origin/HEAD'],
  ['git', '-C', ROOT, 'for-each-ref', '--format=%(refname)', 'refs/remotes/origin'],
]

async function builtWorld(on: Parameters<typeof masterWorld>[0], keys: Partial<Policy>, rules: readonly Rule[], calls: ProcessCall[], options: MasterOptions = {}) {
  const policy = await confirmedPolicy(keys)
  return masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(rules, calls), ...options })
}

test('rex_build_spawns_builder_with_fixed_brief', async ($, on) => {
  const calls: ProcessCall[] = []
  const session = await builtWorld(on, {}, buildRules('model-high-effort-high'), calls)
  await $.session.start(START)

  const built = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'Fix the parser; cargo test passes.', consequence: 'low', size: 'small' })

  const job = session.book().jobs[0]
  const id = job?.id ?? ''
  const worktree = `${ROOT}.worktrees/${id}`
  expect(resultText(built)).toBe(`job=${id}\nbuilder=b1\nstatus=created\nworktree=${worktree}\n`)
  expect(calls.map(call => call.argv)).toEqual([
    ['rex', 'oms', '--json', '--duration', 'none', '--budget', 'none', '--autonomy', 'high', '--intelligence', 'high', '--reliability', 'medium', '--consequence', 'low', '--verification', 'medium'],
    ['git', '-C', ROOT, 'rev-parse', '--verify', '--quiet', 'HEAD'],
    ...PLACES,
    ['git', '-C', ROOT, 'fetch', 'origin', 'main'],
    ['git', '-C', ROOT, 'rev-parse', 'origin/main'],
    ['rex', 'worktree', 'add', ROOT, worktree, START_COMMIT, '--branch', `rex/${id}`],
  ])
  const spawn = session.spawned[0]
  expect([spawn?.type, spawn?.description, spawn?.cwd]).toEqual(['super-rex:model-high-effort-high', `rex builder ${id}`, worktree])
  expect(spawn?.prompt.startsWith(BUILDER_TEXT)).toBe(true)
  expect(spawn?.prompt).toContain(`Rex-Job: ${id}\nRex-Tree: ${TREE}`)
  expect(spawn?.prompt).toContain('<request>\nFix the parser; cargo test passes.\n</request>')
  expect(spawn?.prompt).toContain(`on branch rex/${id} over origin/main.`)
  expect([job?.owner.builder, job?.tree, job?.origin, job?.status, job?.worktree]).toEqual(['b1', TREE, 'master', 'created', worktree])
  const run = session.book().runs.find(each => each.agentId === 'b1')
  expect([run?.parent, run?.parentSource, run?.job, run?.depth]).toEqual([MASTER, 'mod', id, 1])
  expect(session.book().trees[0]?.builders).toEqual([{ job: id, agentId: 'b1' }])
})

test('rex_build in mode none makes the worktree from the local base with no fetch', async ($, on) => {
  const calls: ProcessCall[] = []
  const session = await builtWorld(on, {}, buildRules('model-high-effort-high', 'none'), calls)
  await $.session.start(START)

  const built = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'Fix the parser', consequence: 'low' })

  const id = session.book().jobs[0]?.id ?? ''
  expect(lineOf(built, 'status')).toBe('created')
  expect(session.book().jobs[0]?.base).toBe('main')
  expect(session.spawned[0]?.prompt).toContain(`on branch rex/${id} over main.`)
  expect(calls.map(call => call.argv.slice(3).join(' ')).filter(text => text.startsWith('fetch') || text.startsWith('rev-parse main'))).toEqual(['rev-parse main'])
  expect(calls.find(call => call.argv[1] === 'worktree')?.argv).toEqual(['rex', 'worktree', 'add', ROOT, `${ROOT}.worktrees/${id}`, START_COMMIT, '--branch', `rex/${id}`])
})

test('rex_build in a repo with no commit makes the first commit before the worktree, and says so', async ($, on) => {
  const calls: ProcessCall[] = []
  const session = await builtWorld(on, {}, [...firstCommitRules(), ...buildRules('model-high-effort-high', 'none')], calls)
  await $.session.start(START)

  const built = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'one', consequence: 'low' })

  const steps = calls.map(call => call.argv.slice(0, 4).join(' '))
  expect(lineOf(built, 'base_commit')).toBe('made')
  expect(steps.indexOf(`git -C ${ROOT} commit`)).toBeLessThan(steps.indexOf('rex worktree add /home/person/repo'))
  expect(session.statuses).toContain(`rex: made the first commit of ${ROOT}, the base of rex jobs`)
})

test('rex_build refuses when the first commit fails, with git first error line and the commands to run, restores the index, and opens no job', async ($, on) => {
  const calls: ProcessCall[] = []
  const gpg = { exitCode: 128, stdout: '', stderr: 'error: gpg failed to sign the data\nfatal: failed to write commit object\n' }
  const session = await builtWorld(on, {}, [...firstCommitRules(gpg), ...buildRules('model-high-effort-high', 'none')], calls)
  await $.session.start(START)

  const refused = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'one', consequence: 'low' })

  expect(refused.deny).toBe(
    `rex_build: the first commit of ${ROOT} failed, got git commit exit 128: error: gpg failed to sign the data; ask the person to correct what it names and commit the files with git -C ${ROOT} add --all and git -C ${ROOT} commit, then call rex_build again`,
  )
  expect(calls.map(call => call.argv.slice(3).join(' ')).at(-1)).toBe('read-tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904')
  expect(session.book().jobs).toEqual([])
})

test('rex_build in a session root that no git repo holds refuses with git init, and opens no job', async ($, on) => {
  const noRepo = { starts: ['git', '-C', ROOT, 'rev-parse', '--verify', '--quiet', 'HEAD'], answer: { exitCode: 128, stdout: '', stderr: 'fatal: not a git repository (or any of the parent directories): .git\n' } }
  const session = await builtWorld(on, {}, [noRepo, ...buildRules('model-high-effort-high')], [])
  await $.session.start(START)

  const refused = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'one', consequence: 'low' })

  expect(refused.deny).toBe(`rex_build: ${ROOT} is not a git repo, got fatal: not a git repository (or any of the parent directories): .git; run git init there, then start again`)
  expect(session.book().jobs).toEqual([])
})

test('rex_build in mode origin with a fetch that fails builds from origin/main as last fetched, and says the base may be stale', async ($, on) => {
  const gone = { starts: ['git', '-C', ROOT, 'fetch'], answer: { exitCode: 128, stdout: '', stderr: "fatal: '/home/person/origin.git' does not appear to be a git repository\n" } }
  const session = await builtWorld(on, {}, [gone, ...buildRules('model-high-effort-high')], [])
  await $.session.start(START)

  const built = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'one', consequence: 'low' })

  expect([lineOf(built, 'status'), lineOf(built, 'base_stale')]).toEqual(['created', 'yes'])
  expect(session.statuses.some(line => line.startsWith('rex: git fetch origin main failed, got fatal:'))).toBe(true)
})

test('rex_build_caps_builder_tier_by_policy', async ($, on) => {
  const session = await builtWorld(on, { tier_cap: 'high' }, buildRules('model-xhigh-effort-max'), [])
  await $.session.start(START)

  await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'Fix the parser', consequence: 'high', ratings: { intelligence: 'high', reliability: 'high', consequence: 'high', verification: 'low' } })

  expect(session.spawned.map(spawn => spawn.type)).toEqual(['super-rex:model-high-effort-max'])
})

test('rex_build refuses every caller but the master of a running tree', async ($, on) => {
  const session = await builtWorld(on, {}, buildRules('model-high-effort-high'), [])
  await $.session.start(START)

  const main = await $.tool.call({ tool: BUILD, ...KIND, request: 'x', consequence: 'low' })
  const agent = await $.tool.call({ tool: BUILD, ...KIND, agentId: 'a9', request: 'x', consequence: 'low' })

  expect(main.deny).toBe('rex_build: only the master may call it, got main')
  expect(agent.deny).toBe('rex_build: only the master may call it, got agent')
  expect(session.spawned).toEqual([])
  expect(session.book().jobs).toEqual([])
})

test('rex_build_refuses_over_policy: the tree ledger at its cap', async ($, on) => {
  const policy = await confirmedPolicy({ tree_usd: 10 })
  const seeded = await treeBook(policy)
  const spent = { ...seeded, trees: seeded.trees.map(tree => ({ ...tree, ledgerNano: 10_000_000_000 })) }
  const session = masterWorld(on, { seed: spent, store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)

  const built = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'low' })

  expect(built.deny).toBe(`rex gate: master tree ${TREE} has spent $10.00 of its $10.00; park a spend question for the person with rex_ask_owner, and end your turn`)
  expect(session.book().jobs).toEqual([])
})

test('rex_build_refuses_over_policy: a budget over per_job_usd, and per_job_usd as the limit', async ($, on) => {
  const session = await builtWorld(on, { per_job_usd: 20 }, buildRules('model-high-effort-high'), [])
  await $.session.start(START)

  const over = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'low', budget_usd: 30 })
  const built = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'low' })

  expect(over.deny).toBe('rex_build: budget_usd must be at most per_job_usd of the policy, 20, got 30; call it again with a smaller budget or none')
  expect(lineOf(built, 'status')).toBe('created')
  expect(session.book().jobs.map(job => job.limit)).toEqual([{ nano: 20_000_000_000, source: 'policy' }])
})

test('rex_build refuses at the rate limit of the policy, and the mod wakes the master once, retry.then_minutes later', { timeoutMs: 20_000 }, async ($, on) => {
  const session = await builtWorld(on, { pause_at_rate_limit_pct: 85 }, buildRules('model-high-effort-high'), [], { rateLimits: [{ kind: 'five_hour', percentUsed: 91.5 }] })
  await $.session.start(START)

  const built = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'low' })
  const again = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'y', consequence: 'low' })
  await $.turn.complete({ agentId: MASTER, reason: 'answer', answer: 'waiting for the rate limit', durationMs: 1, isAborted: false, turnId: 'turn-m1' })
  await session.clock.advance(9 * MINUTE)
  await settleLong(session.clock)
  const before = session.sent.length
  await session.clock.advance(MINUTE)
  await settleUntil(session.clock, () => session.sent.length > 0)

  expect(built.deny).toBe('rex gate: plan usage at 91.5% (five_hour); new builders wait until it falls; end your turn, and the mod wakes you in 10 minutes')
  expect(again.deny).toBe(built.deny)
  expect(session.book().jobs).toEqual([])
  expect(before).toBe(0)
  expect(session.sent.map(sent => [sent.to, sent.text])).toEqual([[MASTER, 'rex: plan usage was over the pause point 10 minutes ago. Call rex_build again for the steps that wait.']])
  expect(session.book().trees[0]?.state).toBe('running')
})

test('rex_build refuses a job with no type, size or langs, and stores them on the job it opens', async ($, on) => {
  const session = await builtWorld(on, {}, buildRules('model-high-effort-high'), [])
  await $.session.start(START)

  const noType = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'x', consequence: 'low', size: 'small', langs: [] })
  const noSize = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'x', consequence: 'low', type: 'fix', langs: [] })
  const noLangs = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'x', consequence: 'low', type: 'fix', size: 'small' })
  const before = session.book().jobs.length
  const built = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'x', consequence: 'low', type: 'feature', size: 'large', langs: ['Rust', 'typescript'] })

  expect(noType.deny).toBe('rex_build: type must be one of feature, fix, refactor, chore, research, other, got none; call it again with one of them')
  expect(noSize.deny).toBe('rex_build: size must be one of small, medium, large, got none; call it again with one of them')
  expect(noLangs.deny).toBe('rex_build: langs must be a list of languages, got none; call it again with langs such as ["rust"], or [] for none')
  expect(before).toBe(0)
  expect(lineOf(built, 'status')).toBe('created')
  expect(session.book().jobs.map(job => [job.type, job.size, job.langs])).toEqual([['feature', 'large', ['rust', 'typescript']]])
})

test('rex_build refuses past max_jobs and over the consequence ceiling', async ($, on) => {
  await builtWorld(on, { max_jobs: 1, consequence_ceiling: 'medium', max_parallel_builders: 3 }, buildRules('model-high-effort-high'), [])
  await $.session.start(START)

  const high = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'high' })
  const first = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'low' })
  const second = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'y', consequence: 'low' })

  expect(high.deny).toBe('rex gate: consequence_ceiling of the policy, got high')
  expect(lineOf(first, 'status')).toBe('created')
  expect(second.deny).toBe('rex gate: max_jobs of the policy, got 1')
})

test('parallel_builders_auto_by_size', async ($, on) => {
  const session = await builtWorld(on, { max_parallel_builders: 'auto' }, buildRules('model-high-effort-high'), [])
  await $.session.start(START)

  const small = [1, 2, 3, 4].map(n => ({ tool: BUILD, ...KIND, agentId: MASTER, request: `small ${String(n)}`, consequence: 'low', size: 'small' }))
  const answers = []
  for (const call of small) {
    answers.push(await $.tool.call(call))
  }
  const large = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'large', consequence: 'low', size: 'large' })

  expect(answers.map(answer => answer.deny ?? lineOf(answer, 'status'))).toEqual(['created', 'created', 'created', 'rex gate: max_parallel_builders of the policy, got 3 open jobs'])
  expect(large.deny).toBe('rex gate: max_parallel_builders of the policy, got 3 open jobs')
  expect(session.book().jobs).toHaveLength(3)
})

test('rex_build refuses a job whose depends_on is not delivered, and a repo outside the policy', async ($, on) => {
  const session = await builtWorld(on, {}, buildRules('model-high-effort-high'), [])
  await $.session.start(START)

  const first = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'low' })
  const id = lineOf(first, 'job') ?? ''
  const waiting = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'y', consequence: 'low', depends_on: [id] })
  const elsewhere = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'y', consequence: 'low', repo: '/elsewhere' })

  expect(waiting.deny).toBe(`rex_build: depends_on job ${id} is not delivered, got created; build this job after that one delivers`)
  expect(elsewhere.deny).toBe('rex_build: repo /elsewhere is not the session root or a repo of the policy, got /elsewhere; add it with /rex policy set repos')
  expect(session.book().jobs).toHaveLength(1)
})

test('rex_build refuses bad arguments before anything changes', async ($, on) => {
  const session = await builtWorld(on, {}, buildRules('model-high-effort-high'), [])
  await $.session.start(START)

  const noRequest = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, consequence: 'low' })
  const badLevel = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'huge' })

  expect(noRequest.deny).toBe('rex_build: request must be text, got none; call it again with the request of the job')
  expect(badLevel.deny).toBe('rex_build: consequence must be one of low, medium, high, got huge; call it again with one of them')
  expect(session.book().jobs).toEqual([])
})

test('a refused builder spawn cancels its job and removes its worktree', async ($, on) => {
  const calls: ProcessCall[] = []
  const session = await builtWorld(on, {}, buildRules('model-high-effort-high'), calls, { spawnRefusal: () => 'no slots left' })
  await $.session.start(START)

  const built = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'low' })

  const id = session.book().jobs[0]?.id ?? ''
  expect(built.deny).toBe(`rex_build: the builder of job ${id} did not start, got no slots left; the job is cancelled, call rex_build again`)
  expect(session.book().jobs[0]?.status).toBe('cancelled')
  expect(calls.at(-1)?.argv).toEqual(['rex', 'worktree', 'remove', ROOT, `${ROOT}.worktrees/${id}`])
})

test('a worktree that rex worktree add cannot make cancels the job and starts no builder', async ($, on) => {
  const rules: readonly Rule[] = [{ starts: ['rex', 'worktree', 'add'], answer: { exitCode: 1, stdout: '', stderr: 'rex worktree: a branch named rex/x already exists\n' } }, ...buildRules('model-high-effort-high')]
  const session = await builtWorld(on, {}, rules, [])
  await $.session.start(START)

  const built = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'low' })

  const id = session.book().jobs[0]?.id ?? ''
  expect(built.deny).toBe(`rex_build: rex worktree add failed for job ${id}, got exit 1: rex worktree: a branch named rex/x already exists; the job is cancelled, call rex_build again`)
  expect(session.book().jobs[0]?.status).toBe('cancelled')
  expect(session.spawned).toEqual([])
})

test('authority_guard_that_throws_refuses: a closed check of rex_build that throws refuses the build', async ($, on) => {
  const session = await builtWorld(on, {}, buildRules('model-high-effort-high'), [], { usageRefusal: 'usage unreadable' })
  await $.session.start(START)

  const built = await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'low' })

  expect(built.deny).toBe('rex gate: rate-limit-pause failed (HooksError); refused')
  expect(session.book().jobs).toEqual([])
})

test('a policy answer is set at rex_build by merge and merge_max_consequence', async ($, on) => {
  const session = await builtWorld(on, { merge: 'allowed_if_checks_pass', merge_max_consequence: 'medium', merge_checks: [['cargo', 'test']] }, [...buildRules('model-high-effort-high'), { starts: ['cargo'], answer: ran() }], [])
  await $.session.start(START)

  await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'x', consequence: 'medium' })
  await $.tool.call({ tool: BUILD, ...KIND, agentId: MASTER, request: 'y', consequence: 'high' })

  const tree = session.book().trees[0]
  expect(session.book().jobs.map(job => [job.consequence, job.merge?.answer ?? null, job.merge?.source ?? null, job.merge?.policyHash ?? null])).toEqual([
    ['medium', 'yes', 'policy', tree?.policyHash],
    ['high', null, null, null],
  ])
})

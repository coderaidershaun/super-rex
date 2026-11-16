// Proves the gate: spend at the Agent call and at the spawn, the limit a person set at a shell, depth outside a master tree, forks in a job, agents of a job that would not start as its sub-agents, the merge tripwire, and /rex limit.
import { expect, test } from 'claude-code/testing'
import type { CommandRunInput, PromptOrigin } from 'claude-code'

import type { RexBook } from '../../types'
import { startRunIn } from '../book/runs'
import { CONFIG_ON, ROOT, START_MS, spawnInput, world } from '../book/kit'
import type { ProcessCall } from '../book/kit'
import { sharedJson } from '../book/shared'
import { createParts } from '../parts'
import { callFile, files, JOB, ok, scripted, seededBook } from '../tools/act-kit'
import { GATE_CHECKS, isMergeCommand, isOwnSpawn, refusedAgentArg } from './index'
import { spawnCase } from '../pipeline/brief'
import { AUTHOR } from '../pipeline/assembly.fixture'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, MASTER, masterWorld, POLICY_KEY, treeBook } from '../master/master-kit'
import { registerProbeCheck } from './probe-check.fixture'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const SPENT = 'rex gate: job 0gaq820001 has spent $12.00 of its $10.00 limit; a person raises it with /rex limit 0gaq820001 <usd>'

function spent(book: RexBook, nano: number): RexBook {
  return { ...book, jobs: book.jobs.map(job => ({ ...job, spentNano: nano })) }
}

function engineAnswers(on: Parameters<typeof world>[0]): string[] {
  const started: string[] = []
  on('agent.spawn', (_$, e) => {
    started.push(e.description)
    return { model: 'claude-sonnet-5-5', agentId: `agent-${e.description}` }
  })
  on('tool.call', (_$, e) => ({ result: `ran ${e.tool}` }))
  return started
}

test('spend_over_limit_refused_at_tool_call_and_spawn', async ($, on) => {
  files(on, {})
  const started = engineAnswers(on)
  world(on, { seed: spent(seededBook(), 12_000_000_000) })
  await $.session.start(START)

  const call = await $.tool.call({ tool: 'Agent', description: 'worker', prompt: 'Task: T2' })
  const spawn = await $.agent.spawn(spawnInput({ description: 'worker' }))

  expect(call.deny).toBe(SPENT)
  expect(spawn.deny).toBe(SPENT)
  expect(started).toEqual([])
})

test('the spend check reads the limit a person set at a shell before it refuses', async ($, on) => {
  const limit = callFile(sharedJson('fixtures/shared/calls/limit.json'))
  files(on, {})
  engineAnswers(on)
  const calls: ProcessCall[] = []
  const raised = '{"job":"0gaq820001","spent_usd":12.0,"limit_usd":20.0,"source":"set"}\n'
  const session = world(on, { config: CONFIG_ON, seed: spent(seededBook({ storage: 'on' }), 12_000_000_000), processAnswer: scripted([{ argv: limit.argv, answer: ok(limit.stdout) }, { argv: limit.argv, answer: ok(raised) }], calls) })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: 'Agent', description: 'worker', prompt: 'x' })
  const passed = await $.tool.call({ tool: 'Agent', description: 'worker', prompt: 'x' })

  expect(refused.deny).toBe(SPENT)
  expect(passed.deny).toBeUndefined()
  expect(calls.map(call => call.argv)).toEqual([limit.argv, limit.argv])
  expect(session.book().jobs[0]?.limit).toEqual({ nano: 20_000_000_000, source: 'set' })
})

test('a stored limit above $9,000,000 is not taken: the spend check still refuses and the limit stays', async ($, on) => {
  const limit = callFile(sharedJson('fixtures/shared/calls/limit.json'))
  files(on, {})
  engineAnswers(on)
  const huge = '{"job":"0gaq820001","spent_usd":12.0,"limit_usd":9000001.0,"source":"set"}\n'
  const session = world(on, { config: CONFIG_ON, seed: spent(seededBook({ storage: 'on' }), 12_000_000_000), processAnswer: scripted([{ argv: limit.argv, answer: ok(huge) }]) })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: 'Agent', description: 'worker', prompt: 'x' })

  expect(refused.deny).toBe(SPENT)
  expect(session.book().jobs[0]?.limit).toEqual({ nano: 10_000_000_000, source: 'default' })
})

function chain(book: RexBook, depth: number): RexBook {
  let changed = book
  for (let level = 1; level <= depth; level += 1) {
    const run = { agentId: `d${String(level)}`, toolUseId: null, parentAgentId: level === 1 ? null : `d${String(level - 1)}`, agentType: 'super-rex:model-high-effort-high', description: `d${String(level)}`, isOwnSpawn: false }
    changed = startRunIn(changed, run, START_MS)
  }
  return changed
}

test('depth 4 refused outside a master tree, and allowed below a builder of one', async ($, on) => {
  files(on, {})
  const started = engineAnswers(on)
  const outside = chain(seededBook(), 3)
  const withTree: RexBook = { ...outside, trees: [{ id: 't1', session: outside.session, master: 'm1', charter: 'c', charterHash: 'h', policyHash: 'p', state: 'running', ledgerNano: 0, capNano: 1, builders: [{ job: JOB, agentId: 'd1' }], ticks: [], notes: [], startedAt: START_MS, closedAt: null, reportFile: null }] }
  world(on, { seed: outside })
  await $.session.start(START)

  const deep = await $.agent.spawn(spawnInput({ description: 'deep', parentAgentId: 'd3' }))
  const shallow = await $.agent.spawn(spawnInput({ description: 'shallow', parentAgentId: 'd2', prompt: AUTHOR }))

  expect(deep.deny).toBe('rex gate: depth 4 is for master trees only; start this work one layer up')
  expect(shallow.deny).toBeUndefined()
  expect(started).toEqual(['shallow'])
  expect(withTree.trees).toHaveLength(1)
})

test('depth 4 below a builder of a master tree passes', async ($, on) => {
  files(on, {})
  const started = engineAnswers(on)
  const book = chain(seededBook(), 3)
  const tree = { id: 't1', session: book.session, master: 'm1', charter: 'c', charterHash: 'h', policyHash: 'p', state: 'running', ledgerNano: 0, capNano: 1, builders: [{ job: JOB, agentId: 'd1' }], ticks: [], notes: [], startedAt: START_MS, closedAt: null, reportFile: null } as const
  world(on, { seed: { ...book, trees: [tree] } })
  await $.session.start(START)

  const deep = await $.agent.spawn(spawnInput({ description: 'fanout', parentAgentId: 'd3', prompt: AUTHOR }))

  expect(deep.deny).toBeUndefined()
  expect(started).toEqual(['fanout'])
})

test('depth unknown: a child of an agent the book does not hold starts outside a master tree, and the job is marked', async ($, on) => {
  files(on, {})
  const started = engineAnswers(on)
  const session = world(on, { seed: seededBook() })
  await $.session.start(START)

  const child = await $.agent.spawn(spawnInput({ description: 'child', parentAgentId: 'unrecorded' }))

  expect(child.deny).toBeUndefined()
  expect(started).toEqual(['child'])
  expect(session.book().jobs[0]?.degraded).toEqual(['gate:depth:unknown-parent'])
})

test("the gate takes a spawn as the mod's own only with its pending entry, as the pipeline does", () => {
  const book = seededBook()
  const pending: RexBook = { ...book, pendingSpawns: [{ description: 'rex builder 0gaq820001', kind: 'builder', tree: 't1', job: JOB, at: START_MS }] }
  const own = spawnInput({ description: 'rex builder 0gaq820001' })
  const cases = [
    [book, own, 'super-rex'],
    [pending, own, 'super-rex'],
    [pending, own, 'engine'],
    [pending, { ...own, parentAgentId: 'a1' }, 'super-rex'],
    [pending, { ...own, description: 'rex builder 0gb0000000' }, 'super-rex'],
  ] as const

  expect(cases.map(([state, spawn, origin]) => isOwnSpawn(state, spawn, origin))).toEqual([false, true, false, false, false])
  expect(cases.map(([state, spawn, origin]) => spawnCase(state, spawn, origin, undefined) === 'own')).toEqual([false, true, false, false, false])
})

test('a fork inside a job is refused with the grid agent to start, and a fork outside any job passes', async ($, on) => {
  files(on, {})
  const started = engineAnswers(on)
  world(on, { seed: seededBook() })
  await $.session.start(START)

  const inJob = await $.agent.spawn(spawnInput({ description: 'forked', fork: true }))

  expect(inJob.deny).toBe('rex: a fork cannot change model; start a grid agent instead, subagent_type super-rex:model-high-effort-high')
  expect(started).toEqual([])
})

const DECISION = { tool: 'Agent', subagent_type: 'super-rex:model-high-effort-high', description: 'Decision agent for job', prompt: 'Task: decision-maker' } as const
const REFUSED_ARGS = [
  ['name', 'decision-maker', 'address it by the agent id in the result'],
  ['team_name', 'rex', 'address it by the agent id in the result'],
  ['model', 'opus', 'let the grid agent of subagent_type set its model'],
  ['effort', 'xhigh', 'let the grid agent of subagent_type set its effort'],
  ['mode', 'bypassPermissions', 'let it run in the permission mode of the job'],
  ['isolation', 'worktree', 'let it work where the brief says'],
] as const

function argRefusal(argument: string, value: string, instead: string): string {
  return `rex gate: an agent of a job starts with subagent_type, description and prompt only, got ${argument}=${value}; start it again without ${argument}, and ${instead}`
}

test('inside a job, an Agent call that names, overrides or moves its agent is refused with what to change, and the same call without it passes', async ($, on) => {
  files(on, {})
  const started = engineAnswers(on)
  world(on, { seed: seededBook() })
  await $.session.start(START)

  const refused: (string | undefined)[] = []
  for (const [argument, value] of REFUSED_ARGS) {
    refused.push((await $.tool.call({ ...DECISION, [argument]: value })).deny)
  }
  const plain = await $.tool.call(DECISION)

  expect(refused).toEqual(REFUSED_ARGS.map(([argument, value, instead]) => argRefusal(argument, value, instead)))
  expect(refused[0]).toBe('rex gate: an agent of a job starts with subagent_type, description and prompt only, got name=decision-maker; start it again without name, and address it by the agent id in the result')
  expect(plain.deny).toBeUndefined()
  expect(started).toEqual([])
  expect(refusedAgentArg({ ...DECISION, cwd: '/elsewhere' })).toEqual({ argument: 'cwd', value: '/elsewhere' })
  expect(refusedAgentArg({ ...DECISION, run_in_background: true })).toBeUndefined()
})

test("a lead's Agent call is checked like the builder's, and outside any job the same call passes", async ($, on) => {
  files(on, {})
  engineAnswers(on)
  world(on, { seed: chain(seededBook(), 1) })
  await $.session.start(START)

  const lead = await $.tool.call({ ...DECISION, agentId: 'd1', name: 'scout' })
  const stranger = await $.tool.call({ ...DECISION, agentId: 'outsider', name: 'scout' })

  expect(lead.deny).toBe(argRefusal('name', 'scout', 'address it by the agent id in the result'))
  expect(stranger.deny).toBeUndefined()
})

test('the main loop with no open job starts a named agent unchecked', async ($, on) => {
  files(on, {})
  engineAnswers(on)
  world(on)
  await $.session.start(START)

  const named = await $.tool.call({ ...DECISION, name: 'decision-maker', model: 'opus' })

  expect(named.deny).toBeUndefined()
})

test('inside a job a teammate or a workflow agent is refused at the spawn, and outside any job it starts', async ($, on) => {
  files(on, {})
  const started = engineAnswers(on)
  world(on, { seed: chain(seededBook(), 1) })
  await $.session.start(START)

  const teammate = await $.agent.spawn(spawnInput({ description: 'mate', subagentType: 'decision-maker', isTeammate: true, parentAgentId: 'd1' }))
  const workflow = await $.agent.spawn(spawnInput({ description: 'step', workflow: { runId: 'wf_1', agentIndex: 1 } }))
  const outside = await $.agent.spawn(spawnInput({ description: 'free', isTeammate: true, parentAgentId: 'outsider', prompt: AUTHOR }))

  expect(teammate.deny).toBe('rex gate: an agent of a job starts as a sub-agent of the Agent tool, got a teammate; start it with the Agent tool and subagent_type, description and prompt only')
  expect(workflow.deny).toBe('rex gate: an agent of a job starts as a sub-agent of the Agent tool, got an agent of workflow run wf_1; start it with the Agent tool and subagent_type, description and prompt only')
  expect(outside.deny).toBeUndefined()
  expect(started).toEqual(['free'])
})

test("the mod's own spawn of a builder starts in its worktree, a cwd that an agent of a job may not give", async ($, on) => {
  const policy = await confirmedPolicy({})
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)

  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, agentId: MASTER, request: 'Fix the parser; cargo test passes.', consequence: 'low', size: 'small' })

  const id = session.book().jobs[0]?.id ?? ''
  expect(lineOf(built, 'status')).toBe('created')
  expect(session.spawned.map(spawn => spawn.cwd)).toEqual([`${ROOT}.worktrees/${id}`])
})

test('the job-agent check that cannot read the book refuses the call it checks, and lets a plain call pass unread', async ($, on) => {
  files(on, {})
  const started = engineAnswers(on)
  const session = world(on, { seed: seededBook() })
  await $.session.start(START)

  session.failNextBookRead()
  const named = await $.tool.call({ ...DECISION, name: 'decision-maker' })
  session.failNextBookRead()
  const teammate = await $.agent.spawn(spawnInput({ description: 'mate', isTeammate: true }))

  expect(named.deny).toBe('rex gate: job-agent failed (HooksError); refused')
  expect(teammate.deny).toBe('rex gate: job-agent failed (HooksError); refused')
  expect(started).toEqual([])
})

test('authority_guard_that_throws_refuses: a closed check that throws refuses, an open one passes', async ($, on) => {
  files(on, {})
  const started = engineAnswers(on)
  const chained = chain(spent(seededBook(), 0), 3)
  const broken: RexBook = { ...chained, runs: chained.runs.filter(run => run.agentId !== 'd2') }
  const session = world(on, { seed: broken })
  await $.session.start(START)

  session.failNextBookRead()
  const spend = await $.tool.call({ tool: 'Agent', description: 'a', prompt: 'x' })
  const depth = await $.agent.spawn(spawnInput({ description: 'orphan', parentAgentId: 'd3' }))
  session.failNextBookRead()
  const bash = await $.tool.call({ tool: 'Bash', command: 'ls' })

  expect(spend.deny).toBeUndefined()
  expect(depth.deny).toBe('rex gate: depth failed (UnknownParentError); refused')
  expect(bash.deny).toBe('rex gate: merge-tripwire failed (HooksError); refused')
  expect(started).toEqual([])
  expect(GATE_CHECKS.filter(check => check.failure === 'closed').map(check => check.name)).toEqual(['depth', 'fork', 'job-agent', 'merge-tripwire'])
})

test('the merge tripwire refuses each way round rex_merge, and passes other pushes', async ($, on) => {
  files(on, {})
  engineAnswers(on)
  world(on, { seed: seededBook() })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: 'Bash', command: 'cd /repo && GIT_TRACE=1 git push origin HEAD:main' })
  const passed = await $.tool.call({ tool: 'Bash', command: `git push origin rex/${JOB}` })

  expect(refused.deny).toBe('rex gate: merges reach main only through rex_merge, got cd /repo && GIT_TRACE=1 git push origin HEAD:main')
  expect(passed.deny).toBeUndefined()
  const merges = ['git push origin main', 'git push -f origin +x:main', 'git -C /r push origin rex/a:refs/heads/main', 'git push --force-with-lease origin HEAD:refs/heads/main', 'git status && git push origin main', 'gh pr merge 12 --squash', 'gh api -X PUT repos/o/r/pulls/1/merge', 'gh api repos/o/r/merges -f base=main', 'git push --mirror origin']
  const others = ['git push origin rex/0gaq820001', 'git push -u origin feature/x', 'git log main', 'gh pr view 1', 'echo merge main']
  expect(merges.filter(command => !isMergeCommand(command, 'main', `rex/${JOB}`))).toEqual([])
  expect(others.filter(command => isMergeCommand(command, 'main', `rex/${JOB}`))).toEqual([])
})

test('the merge tripwire refuses a move of the local base by hand, and passes the merges of the skill', async ($, on) => {
  files(on, {})
  engineAnswers(on)
  world(on, { seed: seededBook() })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: 'Bash', command: `git -C /home/person/repo merge --ff-only rex/${JOB}` })

  expect(refused.deny).toBe(`rex gate: merges reach main only through rex_merge, got git -C /home/person/repo merge --ff-only rex/${JOB}`)
  const moves = [
    `git merge rex/${JOB}`,
    `git merge --squash rex/${JOB}`,
    `git -C /home/person/repo merge --ff-only rex/${JOB}--work`,
    'git branch -f main rex/x',
    'git branch --force main HEAD',
    'git branch -m main old',
    'git branch -M rex/x main',
    'git branch -C rex/x main',
    'git -C /home/person/repo branch -D main',
    'git update-ref refs/heads/main c1',
    'git push . rex/x:main',
    'git fetch . rex/x:main',
    'git fetch . +HEAD:refs/heads/main',
  ]
  const others = ['git log main', 'git branch -D rex/x', 'git update-ref refs/heads/rex/x c1', 'git fetch origin main']
  expect(moves.filter(command => !isMergeCommand(command, 'main', `rex/${JOB}`))).toEqual([])
  expect(others.filter(command => isMergeCommand(command, 'main', `rex/${JOB}`))).toEqual([])
})

test('the merge tripwire passes each git command the rex-deliver skill gives, as written there and with its values filled in, in both modes', () => {
  const branch = `rex/${JOB}`
  const repo = '/home/person/repo'
  const job = `${repo}.worktrees/${JOB}`
  // The commands of rex-deliver start.md, end.md, clean-up.md and parallel-writers.md, word for word, then as an agent writes them out in mode `local` with base `master`.
  const written = [
    'git -C "$REPO_DIR" fetch origin "$MAIN"',
    'START_COMMIT=$(git -C "$JOB_DIR" rev-parse "$BRANCH")',
    'START_COMMIT=$(git -C "$REPO_DIR" rev-parse "$BASE_REF")',
    'git -C "$JOB_DIR" fetch origin "$MAIN"',
    'git -C "$JOB_DIR" merge --no-edit "$BASE_REF"',
    'WORK_TIP=$(git -C "$JOB_DIR" rev-parse "$BRANCH")',
    'git -C "$JOB_DIR" branch -f "$BRANCH--work" "$WORK_TIP"',
    'git -C "$JOB_DIR" reset --soft "$BASE_REF"',
    'git -C "$JOB_DIR" rebase "$BASE_REF"',
    'git -C "$PARENT_DIR" merge --no-ff --no-edit "$UNIT_BRANCH"',
    'git -C "$REPO_DIR" fetch --prune origin',
    'git -C "$REPO_DIR" merge --ff-only "origin/$MAIN"',
    'git -C "$REPO_DIR" fetch origin "$MAIN:$MAIN"',
    'for B in $(git -C "$REPO_DIR" branch --list --format=\'%(refname:short)\' "rex/$JOB" "rex/$JOB--*"); do',
    '  if git -C "$REPO_DIR" merge-base --is-ancestor "$B" "$WORK_TIP" || git -C "$REPO_DIR" merge-base --is-ancestor "$B" "$BASE_REF"; then git -C "$REPO_DIR" branch -D "$B"; else echo "KEEP $B"; fi',
    `git -C "$REPO_DIR" push origin --delete rex/${JOB}`,
  ]
  const filled = [
    `git -C ${job} merge --no-edit master`,
    `git -C ${job} branch -f ${branch}--work 5a1f0c9e2b7d4a6f8c3e1b0d9a7f5c3e1b2d4f6a`,
    `git -C ${job} reset --soft master`,
    `git -C ${job} rebase master`,
    `git -C ${job} merge --no-ff --no-edit ${branch}--api`,
    `git -C ${repo} branch -D ${branch}`,
    `git -C ${repo} branch -D ${branch}--work`,
    `git -C ${repo} merge-base --is-ancestor ${branch} 5a1f0c9e2b7d4a6f8c3e1b0d9a7f5c3e1b2d4f6a && git -C ${repo} branch -D ${branch}`,
  ]
  expect(written.filter(command => isMergeCommand(command, 'main', branch))).toEqual([])
  expect(filled.filter(command => isMergeCommand(command, 'master', branch))).toEqual([])
  expect(isMergeCommand('git branch -m master main', 'master', branch)).toBe(true)
})

test('the merge tripwire sees through shells, wrappers, subshells, --repo and a push that names no branch, and watches Monitor too', async ($, on) => {
  files(on, {})
  engineAnswers(on)
  world(on, { seed: seededBook() })
  await $.session.start(START)

  const merges = [
    'bash -c "git push origin main"',
    "sh -c 'cd /r && git push origin main'",
    'bash -lc "git push origin HEAD:main"',
    'env X=1 git push origin main',
    'env -u HOME git push origin main',
    'command git push origin main',
    'timeout 60 git push origin main',
    '/usr/bin/git push origin main',
    '(git push origin main)',
    '{ git push origin main; }',
    'echo $(git push origin main)',
    'eval "git push origin main"',
    'git push --repo=origin main',
    'git push --repo origin main',
    'git push -o ci.skip origin main',
    'git push',
    'git push origin',
    'git push origin HEAD',
    'git push origin :',
    "git push origin '+refs/heads/*:refs/heads/*'",
    'git push origin $BRANCH',
    'git push origin ma\\in',
    'echo main | xargs git push origin',
    'curl -X PUT https://api.github.com/repos/o/r/pulls/1/merge',
    "gh api graphql -f query='mutation { mergePullRequest(input: {}) { clientMutationId } }'",
  ]
  const others = ['git push origin rex/0gaq820001', 'git push origin HEAD:refs/heads/rex/0gaq820001', 'git push --tags origin', 'bash -c "git status"', 'env git log main', 'sh ./build.sh main', 'git push -u origin feature/x']
  expect(merges.filter(command => !isMergeCommand(command, 'main', `rex/${JOB}`))).toEqual([])
  expect(others.filter(command => isMergeCommand(command, 'main', `rex/${JOB}`))).toEqual([])

  const monitored = await $.tool.call({ tool: 'Monitor', description: 'push', timeout_ms: 1000, command: 'sh -c "git push origin main"' })
  const watched = await $.tool.call({ tool: 'Monitor', description: 'log', timeout_ms: 1000, command: 'git log -1' })

  expect(monitored.deny).toBe('rex gate: merges reach main only through rex_merge, got sh -c "git push origin main"')
  expect(watched.deny).toBeUndefined()
})

function rex(args: string, origin: PromptOrigin = { kind: 'composer' }): CommandRunInput {
  return { command: 'rex', args, origin, presentation: { isFullscreen: false, columns: 120 } }
}

test('/rex limit shows and sets a limit from the person only, and refuses an amount out of range or a job it does not know', async ($, on) => {
  files(on, {})
  const session = world(on, { seed: seededBook() })
  await $.session.start(START)

  const shown = await $.command.run(rex(`limit ${JOB}`))
  const refused = await $.command.run(rex(`limit ${JOB} 25`, { kind: 'sdk' }))
  const bad = await $.command.run(rex(`limit ${JOB} 9000001`))
  const set = await $.command.run(rex(`limit ${JOB} 25`))
  const unnamed = await $.command.run(rex('limit'))
  const unknown = await $.command.run(rex('limit 0gaq829999 25'))

  expect(shown.text).toBe(`job=${JOB}\nspent_usd=0.00\nlimit_usd=10.00\nlimit_source=default\n`)
  expect(refused.text).toBe('rex limit: only your own typing may run this, got origin sdk')
  expect(bad.text).toBe('rex limit: 9000001 is not an amount from 0 to 9000000 dollars, got 9000001')
  expect(set.text).toBe(`job=${JOB}\nspent_usd=0.00\nlimit_usd=25.00\nlimit_source=set\n`)
  expect(session.book().jobs[0]?.limit).toEqual({ nano: 25_000_000_000, source: 'set' })
  expect(unnamed.text).toBe('rex limit: needs the id of a job, got none; name a job of /rex jobs')
  expect(unknown.text).toBe('rex limit: no job 0gaq829999 in this session, got 0gaq829999; name a job of /rex jobs')
})

test('a gate check another module adds stands in the table, and its closed hook refuses when it throws', async ($, on) => {
  const parts = createParts()
  registerProbeCheck(on, parts)
  world(on)
  await $.session.start(START)

  const refused = await $.tool.call({ tool: 'mcp__probe__check' })

  expect(parts.gateChecks.entries()).toEqual([{ name: 'probe', site: { kind: 'tool', tool: 'mcp__probe__check' }, failure: 'closed' }])
  expect(refused.deny).toMatch(/^rex gate: probe failed \(.+\); refused$/)
})

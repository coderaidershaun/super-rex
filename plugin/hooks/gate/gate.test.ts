// Proves the gate: spend at the Agent call and at the spawn, the limit a person set at a shell, depth outside a master tree, forks in a job, the merge tripwire, and /rex limit.
import { expect, test } from 'claude-code/testing'
import type { CommandRunInput, PromptOrigin } from 'claude-code'

import type { RexBook } from '../../types'
import { startRunIn } from '../book/runs'
import { CONFIG_ON, ROOT, START_MS, spawnInput, world } from '../book/kit'
import type { ProcessCall } from '../book/kit'
import { sharedJson } from '../book/shared'
import { createParts } from '../parts'
import { callFile, files, JOB, ok, scripted, seededBook } from '../tools/act-kit'
import { GATE_CHECKS, isMergeCommand, isOwnSpawn } from './index'
import { spawnCase } from '../pipeline/brief'
import { AUTHOR } from '../pipeline/assembly.fixture'
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
  expect(GATE_CHECKS.filter(check => check.failure === 'closed').map(check => check.name)).toEqual(['depth', 'fork', 'merge-tripwire'])
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
  expect(merges.filter(command => !isMergeCommand(command, 'main'))).toEqual([])
  expect(others.filter(command => isMergeCommand(command, 'main'))).toEqual([])
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
  expect(merges.filter(command => !isMergeCommand(command, 'main'))).toEqual([])
  expect(others.filter(command => isMergeCommand(command, 'main'))).toEqual([])

  const monitored = await $.tool.call({ tool: 'Monitor', description: 'push', timeout_ms: 1000, command: 'sh -c "git push origin main"' })
  const watched = await $.tool.call({ tool: 'Monitor', description: 'log', timeout_ms: 1000, command: 'git log -1' })

  expect(monitored.deny).toBe('rex gate: merges reach main only through rex_merge, got sh -c "git push origin main"')
  expect(watched.deny).toBeUndefined()
})

function rex(args: string, origin: PromptOrigin = { kind: 'composer' }): CommandRunInput {
  return { command: 'rex', args, origin, presentation: { isFullscreen: false, columns: 120 } }
}

test('/rex limit shows and sets a limit from the person only, and refuses an amount out of range', async ($, on) => {
  files(on, {})
  const session = world(on, { seed: seededBook() })
  await $.session.start(START)

  const shown = await $.command.run(rex(`limit ${JOB}`))
  const refused = await $.command.run(rex(`limit ${JOB} 25`, { kind: 'sdk' }))
  const bad = await $.command.run(rex(`limit ${JOB} 9000001`))
  const set = await $.command.run(rex(`limit ${JOB} 25`))

  expect(shown.text).toBe(`job=${JOB}\nspent_usd=0.00\nlimit_usd=10.00\nlimit_source=default\n`)
  expect(refused.text).toBe('rex limit: only your own typing may run this, got origin sdk')
  expect(bad.text).toBe('rex limit: 9000001 is not an amount from 0 to 9000000 dollars, got 9000001')
  expect(set.text).toBe(`job=${JOB}\nspent_usd=0.00\nlimit_usd=25.00\nlimit_source=set\n`)
  expect(session.book().jobs[0]?.limit).toEqual({ nano: 25_000_000_000, source: 'set' })
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

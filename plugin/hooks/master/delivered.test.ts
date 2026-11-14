// Proves what follows a delivered tree job: the done check from git answers, the merge commit and report in rex_status, the charter in every brief of the tree, and the final report once every job is closed.
import { expect, test } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { closeJobIn, mergeDoneIn, openJobIn } from '../book/jobs'
import { ROOT, spawnInput, START_MS } from '../book/kit'
import type { ProcessCall } from '../book/kit'
import { linkIn } from '../book/trees'
import { AUTHOR } from '../pipeline/assembly.fixture'
import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, lineOf, MASTER, masterWorld, POLICY_KEY, ran, rex, settleUntil, START, TREE, treeBook } from './master-kit'
import type { Rule } from './master-kit'

const JOB = '0gaq820002'
const OTHER = '0gaq820003'
const HAND_OFF = `${ROOT}/.rex/plans/${JOB}/builder.md`

function treeJob(book: RexBook, id: string, builder: string): RexBook {
  const input = {
    builder,
    tree: TREE,
    origin: 'master',
    request: `Fix part ${id}\nwith tests`,
    repo: ROOT,
    base: 'main',
    type: null,
    size: 'small',
    consequence: 'low',
    langs: [],
    prefs: { duration: null, budget: null, autonomy: 'high' },
    limit: { nano: 10_000_000_000, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  return linkIn(openJobIn(book, input, id, START_MS).book, TREE, id, builder, START_MS)
}

/** Tree t1 with job JOB of builder b1, merged as c1 by rex_merge and closed delivered. */
async function deliveredBook(policy: Policy): Promise<RexBook> {
  const opened = treeJob(await treeBook(policy), JOB, 'b1')
  const merged = mergeDoneIn(opened, JOB, { repo: ROOT, base: 'main', branch: `rex/${JOB}`, mergeCommit: 'c1' }, START_MS)
  const deliveries = merged.jobs.find(job => job.id === JOB)?.deliveries ?? []
  return closeJobIn(merged, JOB, { status: 'delivered', outcome: 'parser fixed', deliveries }, START_MS)
}

function doneRules(branches: string): readonly Rule[] {
  return [
    { starts: ['git', '-C', ROOT, 'fetch', 'origin'], answer: ran() },
    { starts: ['git', '-C', ROOT, 'branch', '-r', '--contains', 'c1'], answer: ran('  origin/main\n  origin/rex/x\n') },
    { starts: ['git', '-C', ROOT, 'rev-parse', 'main', 'origin/main'], answer: ran('abc\nabc\n') },
    { starts: ['git', '-C', ROOT, 'branch', '-a', '--list', `*${JOB}*`], answer: ran(branches) },
    { starts: ['git', '-C', ROOT, 'worktree', 'list', '--porcelain'], answer: ran(`worktree ${ROOT}\nHEAD abc\nbranch refs/heads/main\n`) },
    { starts: ['cargo', 'test'], answer: ran() },
  ]
}

const ENDED = { agentId: 'b1', reason: 'answer', answer: 'done', durationMs: 1, isAborted: false, turnId: 'turn-b1' } as const

test('done_check_lines_from_git_answers', async ($, on) => {
  const policy = await confirmedPolicy({ merge_checks: [['cargo', 'test']] })
  const calls: ProcessCall[] = []
  const session = masterWorld(on, { seed: await deliveredBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(doneRules(''), calls), files: { [HAND_OFF]: '# Report\n' } })
  await $.session.start(START)

  await $.turn.complete(ENDED)
  await settleUntil(session.clock, () => session.sent.some(sent => sent.to === MASTER))
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: MASTER, job: JOB })

  expect(calls.map(call => call.argv.slice(3, 5).join(' ') || call.argv.join(' '))).toEqual(['fetch origin', 'branch -r', 'rev-parse main', 'branch -a', 'worktree list', 'cargo test'])
  expect(['fetched', 'merged', 'base_current', 'branches_gone', 'worktrees_gone', 'output_exists', 'checks_on_base'].map(check => lineOf(status, `done.${check}`))).toEqual(['pass', 'pass', 'pass', 'pass', 'pass', 'pass', 'pass'])
  expect(session.book().jobs[0]?.doneChecks).toHaveLength(7)
  expect(session.sent.find(sent => sent.to === MASTER)?.text).toContain(`job ${JOB} was delivered and its done check ran`)
})

test('done check: a branch left behind fails its line, and no merge checks reads not-set', async ($, on) => {
  const policy = await confirmedPolicy({ merge_checks: [] })
  const session = masterWorld(on, { seed: await deliveredBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(doneRules(`  rex/${JOB}\n`)) })
  await $.session.start(START)

  await $.turn.complete(ENDED)
  await settleUntil(session.clock, () => session.sent.some(sent => sent.to === MASTER))
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: MASTER, job: JOB })

  expect([lineOf(status, 'done.branches_gone'), lineOf(status, 'done.output_exists'), lineOf(status, 'done.checks_on_base')]).toEqual(['fail', 'fail', 'not-set'])
  expect(session.book().jobs[0]?.status).toBe('delivered')
})

test('status_names_merge_commit_and_output', async ($, on) => {
  const policy = await confirmedPolicy()
  masterWorld(on, { seed: await deliveredBook(policy), store: { [POLICY_KEY]: policy } })
  await $.session.start(START)

  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: MASTER, job: JOB })

  expect([lineOf(status, 'merge_commit'), lineOf(status, 'output'), lineOf(status, 'tree')]).toEqual(['c1', HAND_OFF, TREE])
})

test('final_report_lists_each_job', async ($, on) => {
  const policy = await confirmedPolicy()
  const delivered = await deliveredBook(policy)
  const both = closeJobIn(treeJob(delivered, OTHER, 'b2'), OTHER, { status: 'blocked', outcome: 'needs a person', deliveries: [] }, START_MS)
  const session = masterWorld(on, { seed: both, store: { [POLICY_KEY]: policy } })
  await $.session.start(START)

  await $.turn.complete({ agentId: MASTER, reason: 'answer', answer: 'Result: the parser ships; the lexer waits.\nNot run: none.', durationMs: 1, isAborted: false, turnId: 'turn-m1' })

  const path = `${ROOT}/.rex/plans/${TREE}/final-report.md`
  const report = session.written.find(file => file.path === path)?.text ?? ''
  expect(report).toContain(`- ${JOB} delivered: Fix part ${JOB} (merge commit c1;`)
  expect(report).toContain(`- ${OTHER} blocked: Fix part ${OTHER} (merge commit none;`)
  expect(report).toContain('Result: the parser ships; the lexer waits.\nNot run: none.')
  const tree = session.book().trees[0]
  expect([tree?.state, tree?.reportFile]).toEqual(['done', path])
})

test('a master that ends with a job open writes no report', async ($, on) => {
  const policy = await confirmedPolicy()
  const session = masterWorld(on, { seed: treeJob(await treeBook(policy), OTHER, 'b2'), store: { [POLICY_KEY]: policy } })
  await $.session.start(START)

  await $.turn.complete({ agentId: MASTER, reason: 'answer', answer: 'waiting', durationMs: 1, isAborted: false, turnId: 'turn-m1' })

  expect(session.written).toEqual([])
  expect(session.book().trees[0]?.state).toBe('running')
})

test('charter_file_stored_unchanged_and_in_each_brief', async ($, on) => {
  const charter = '# Ship the parser\n\n| Step | Outcome |\n| --- | --- |\n| 1 | lexer |\n\n## Constraints\n\n- Keep infra/ as it is.\n- One squash commit per job.\n\n## Notes\n\nNot for briefs.\n'
  const policy = await confirmedPolicy()
  const session = masterWorld(on, { store: { [POLICY_KEY]: policy }, files: { [`${ROOT}/charter.md`]: charter }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)

  await $.command.run(rex('master @charter.md'))
  const master = session.book().trees[0]?.master ?? ''
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', agentId: master, request: 'Build the lexer', consequence: 'medium' })
  await $.agent.spawn(spawnInput({ description: 'engineer', parentAgentId: lineOf(built, 'builder') ?? '', prompt: AUTHOR }))

  expect(session.book().trees[0]?.charter).toBe(charter)
  const brief = session.spawned.find(spawn => spawn.description === 'engineer')?.prompt ?? ''
  expect(brief).toContain(`<charter-constraints>\n- Keep infra/ as it is.\n- One squash commit per job.\n</charter-constraints>`)
  expect(brief).not.toContain('Not for briefs.')
  expect(brief).toContain(`Merge answer of job ${lineOf(built, 'job') ?? ''}: none yet; rex_merge parks a question for the person`)
  expect(brief).toContain(`Consequence of job ${lineOf(built, 'job') ?? ''}: medium`)
})

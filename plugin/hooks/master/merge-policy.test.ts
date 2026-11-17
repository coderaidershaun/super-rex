// Proves the policy's rules over rex_merge of a tree job: the merge order, a changed policy, the paths that always ask, the checks a policy answer needs, and the person's answer to a merge question, by command or by a press in the pane.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import type { ProcessCall } from '../book/kit'
import { ROOT } from '../book/kit'
import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, masterWorld, POLICY_KEY, ran, resultText, rex, settleUntil, START, treeBook } from './master-kit'
import type { MasterOptions, Rule } from './master-kit'

const BUILD = 'mcp__super-rex__rex_build' as const
const MERGE = 'mcp__super-rex__rex_merge' as const

function mergeRules(diff: string): readonly Rule[] {
  return [
    { starts: ['git', '-C', ROOT, 'rev-list'], answer: ran('1\n') },
    { starts: ['git', '-C', ROOT, 'merge-base'], answer: ran() },
    { starts: ['git', '-C', ROOT, 'diff'], answer: ran(diff) },
    { starts: ['git', '-C', ROOT, 'push'], answer: ran() },
    { starts: ['git', '-C', ROOT, 'rev-parse'], answer: ran('c1\n') },
  ]
}

async function treeWith($: Engine, on: On, keys: Partial<Policy>, rules: readonly Rule[], calls: ProcessCall[] = [], options: MasterOptions = {}) {
  const policy = await confirmedPolicy(keys)
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule([...rules, ...buildRules('model-high-effort-high')], calls), ...options })
  await $.session.start(START)
  return session
}

/** The builder records a one-task plan, as it must before rex_merge. */
async function planAs($: Engine, built: unknown): Promise<void> {
  const planned = await $.tool.call({
    tool: 'mcp__super-rex__rex_job_plan',
    agentId: lineOf(built, 'builder') ?? '',
    orchestrator: 'single-owner-edit',
    breakdown: 'T1 engineer',
    tasks: [{ id: 'T1', role: 'engineer', skills: [], open_when: [] }],
    roles: [{ role: 'engineer', intelligence: 'high', reliability: 'high', consequence: 'high', verification: 'low' }],
  })
  expect(lineOf(planned, 'status')).toBe('planned')
}

test('merge_after_holds_a_merge', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))
  const first = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, first)
  const firstJob = lineOf(first, 'job') ?? ''
  const second = await $.tool.call({ tool: BUILD, ...KIND, request: 'two', consequence: 'low', merge_after: [firstJob] })
  await planAs($, second)
  const secondBuilder = lineOf(second, 'builder') ?? ''

  const held = await $.tool.call({ tool: MERGE, agentId: secondBuilder })
  const asked = await $.tool.call({ tool: MERGE, agentId: lineOf(first, 'builder') ?? '' })
  const firstQuestion = lineOf(asked, 'question') ?? ''
  await $.command.run(rex(`answer ${firstQuestion} Merge`))
  const merged = await $.tool.call({ tool: MERGE, agentId: lineOf(first, 'builder') ?? '' })
  const after = await $.tool.call({ tool: MERGE, agentId: secondBuilder })

  expect(held.deny).toBe(`rex_merge: job ${lineOf(second, 'job') ?? ''} merges after job ${firstJob}, got ${firstJob} planned and not merged; call again after it merges`)
  expect(lineOf(merged, 'merged')).toBe('yes')
  expect(lineOf(after, 'merged')).toBe('no')
  expect(lineOf(after, 'question')).toBe(session.book().questions[1]?.id)
  expect(session.book().questions.map(question => question.id)).not.toContain(undefined)
  expect(session.book().questions[0]?.id).toBe(firstQuestion)
  expect(session.book().jobs.map(job => job.deliveries.length)).toEqual([1, 0])
})

test('answered_merge_question_lets_tree_job_merge', async ($, on) => {
  const calls: ProcessCall[] = []
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'), calls)
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)
  const builder = lineOf(built, 'builder') ?? ''

  const asked = await $.tool.call({ tool: MERGE, agentId: builder })
  const id = lineOf(asked, 'question') ?? ''
  const again = await $.tool.call({ tool: MERGE, agentId: builder })
  const answered = await $.command.run(rex(`answer ${id} merge`))
  await settleUntil(session.clock, () => session.sent.length > 0 && session.woken().length > 0)
  const merged = await $.tool.call({ tool: MERGE, agentId: builder })

  expect(session.book().questions[0]?.id).toBe(id)
  expect(resultText(asked)).toBe(`merged=no\nquestion=${id}\n`)
  expect(resultText(again)).toBe(`merged=no\nquestion=${id}\n`)
  expect(answered.text).toBe(`question=${id}\nanswer=Merge\n`)
  const job = session.book().jobs[0]
  expect([job?.merge?.answer, job?.merge?.source, job?.merge?.question]).toEqual(['yes', 'question', id])
  expect(lineOf(merged, 'merged')).toBe('yes')
  expect(session.book().questions).toHaveLength(1)
  expect(calls.filter(call => call.argv.includes('push'))).toHaveLength(1)
  expect(session.sent.map(sent => sent.to)).toEqual([builder])
  expect(session.sent[0]?.text).toBe(`rex: the person answered question ${id}: Merge. Call rex_merge again.`)
  expect(session.woken()).toHaveLength(1)
  expect(session.woken()[0]).toContain(`rex: question ${id} is answered: Merge. The tree now:\n`)
  expect(session.woken()[0]).toContain(`answer.${id}=Merge`)
})

const PANE = {
  plugin: 'super-rex',
  component: 'Pane',
  requestId: 'rex',
  surface: 'terminal',
  props: { title: 'SuperRex', isFocused: true, bodyColumns: 120, placement: 'dock', scroll: { offset: 0, bodyRows: 40 }, view: {} },
} as const

test('a merge question answered by a press in the pane wakes the builder to merge and the master to read the tree', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)
  const builder = lineOf(built, 'builder') ?? ''
  const asked = await $.tool.call({ tool: MERGE, agentId: builder })
  const id = lineOf(asked, 'question') ?? ''

  const ui = await $.ui.mount(PANE)
  await ui.press({ key: 'view-questions' })
  await ui.press({ key: `answer-${id}-0` })
  await ui.unmount()
  await settleUntil(session.clock, () => session.sent.length > 0 && session.woken().length > 0)

  expect(session.book().questions[0]?.answer).toMatchObject({ option: 'Merge', source: 'pane' })
  expect(session.sent.map(sent => sent.to)).toEqual([builder])
  expect(session.sent[0]?.text).toBe(`rex: the person answered question ${id}: Merge. Call rex_merge again.`)
  expect(session.woken()[0]).toContain(`rex: question ${id} is answered: Merge. The tree now:\n`)
})

/** The status lines that show question `id` to the person. */
function shown(statuses: readonly string[], id: string): readonly string[] {
  return statuses.filter(line => line.startsWith(`rex: question ${id} waits: `))
}

test('a merge question rex_merge parks shows the status line with the command that answers it, once', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)
  const builder = lineOf(built, 'builder') ?? ''

  const asked = await $.tool.call({ tool: MERGE, agentId: builder })
  await $.tool.call({ tool: MERGE, agentId: builder })

  const id = lineOf(asked, 'question') ?? ''
  const job = lineOf(built, 'job') ?? ''
  expect(shown(session.statuses, id)).toEqual([`rex: question ${id} waits: Merge job ${job} (rex/${job}) to main of ${ROOT}? Answer with /rex answer ${id} <1-2>`])
})

test('a policy answer merges only after each merge check passes in the job worktree', async ($, on) => {
  const calls: ProcessCall[] = []
  const failing: Rule = { starts: ['cargo', 'test'], answer: { exitCode: 101, stdout: '', stderr: 'test parser ... FAILED\n' } }
  const session = await treeWith($, on, { merge: 'allowed_if_checks_pass', merge_checks: [['cargo', 'test']] }, [failing, ...mergeRules('src/a.rs\n')], calls)
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)
  const job = lineOf(built, 'job') ?? ''

  const refused = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  expect(refused.deny).toBe(`rex_merge: merge check cargo test failed in ${ROOT}.worktrees/${job}, got exit 101: test parser ... FAILED; fix it and call again`)
  expect(calls.find(call => call.argv[0] === 'cargo')?.cwd).toBe(`${ROOT}.worktrees/${job}`)
  expect(calls.some(call => call.argv.includes('push'))).toBe(false)
  expect(session.book().jobs[0]?.deliveries).toEqual([])
})

test('a policy answer with passing checks merges', async ($, on) => {
  const session = await treeWith($, on, { merge: 'allowed_if_checks_pass', merge_checks: [['cargo', 'test']] }, [{ starts: ['cargo', 'test'], answer: ran() }, ...mergeRules('src/a.rs\n')])
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)

  const merged = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  expect(lineOf(merged, 'merged')).toBe('yes')
  expect(session.book().jobs[0]?.deliveries.map(delivery => delivery.mergeCommit)).toEqual(['c1'])
})

test('a policy answer with no merge checks merges on the builder own checks, as a job a person starts does', async ($, on) => {
  const calls: ProcessCall[] = []
  await treeWith($, on, { merge: 'allowed_if_checks_pass', merge_checks: [] }, mergeRules('src/a.rs\n'), calls)
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)

  const merged = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  expect(lineOf(merged, 'merged')).toBe('yes')
  expect(calls.filter(call => call.argv.includes('push')).map(call => call.argv.slice(3))).toEqual([['push', 'origin', `rex/${lineOf(built, 'job') ?? ''}:refs/heads/main`]])
})

test('in mode none the paths that always ask are read against the local base, and the merge fast-forwards it', async ($, on) => {
  const calls: ProcessCall[] = []
  const local: readonly Rule[] = [{ starts: ['git', '-C', ROOT, 'worktree', 'list'], answer: ran(`worktree ${ROOT}\nHEAD c0\nbranch refs/heads/main\n`) }, { starts: ['git', '-C', ROOT, 'merge', '--ff-only'], answer: ran() }, ...mergeRules('src/a.rs\n')]
  const policy = await confirmedPolicy({ merge: 'allowed_if_checks_pass', merge_checks: [] })
  masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule([...local, ...buildRules('model-high-effort-high', 'none')], calls) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)

  const merged = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  const branch = `rex/${lineOf(built, 'job') ?? ''}`
  expect(lineOf(merged, 'merged')).toBe('yes')
  expect(calls.filter(call => call.argv.includes('diff')).map(call => call.argv.slice(3))).toEqual([['diff', '--name-only', `main...${branch}`], ['diff', '--name-only', `main...${branch}`]])
  expect(calls.filter(call => call.argv.includes('--ff-only')).map(call => call.argv.slice(3))).toEqual([['merge', '--ff-only', branch]])
  expect(calls.some(call => call.argv.includes('fetch') || call.argv.includes('push'))).toBe(false)
})

test('a diff that touches always_ask_paths turns a policy answer into a merge question', async ($, on) => {
  const calls: ProcessCall[] = []
  const session = await treeWith($, on, { merge: 'allowed_if_checks_pass', merge_checks: [['cargo', 'test']] }, [{ starts: ['cargo'], answer: ran() }, ...mergeRules('src/a.rs\ninfra/main.tf\n')], calls)
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)

  const asked = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  expect(resultText(asked)).toBe(`merged=no\nquestion=${session.book().questions[0]?.id ?? ''}\n`)
  expect(session.book().questions[0]?.question).toContain('infra/main.tf')
  expect(session.book().questions[0]?.mergeFor).toBe(lineOf(built, 'job'))
  expect(calls.some(call => call.argv.includes('push') || call.argv[0] === 'cargo')).toBe(false)
})

test('a merge question the policy check parks shows the status line with the command that answers it', async ($, on) => {
  const session = await treeWith($, on, { merge: 'allowed_if_checks_pass' }, mergeRules('infra/main.tf\n'))
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)

  const asked = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  const id = lineOf(asked, 'question') ?? ''
  const text = session.book().questions[0]?.question ?? ''
  expect(shown(session.statuses, id)).toEqual([`rex: question ${id} waits: ${text.slice(0, 80)}… Answer with /rex answer ${id} <1-2>`])
})

test('changed_policy_halts_tree', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)
  const builder = lineOf(built, 'builder') ?? ''

  const set = await $.command.run(rex('policy set merge never'))
  const build = await $.tool.call({ tool: BUILD, ...KIND, request: 'two', consequence: 'low' })
  const merge = await $.tool.call({ tool: MERGE, agentId: builder })
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: builder })

  const hash = /hash=(\S+)/.exec(set.text ?? '')?.[1] ?? ''
  expect(set.text).toContain('merge=never\n')
  expect(set.text).toContain('paused=t1\n')
  expect(build.deny).toBe(`rex: policy changed since this tree opened, got hash ${hash}; confirm it with /rex policy confirm`)
  expect(merge.deny).toBe(`rex: policy changed since this tree opened, got hash ${hash}; confirm it with /rex policy confirm`)
  expect(session.book().trees[0]?.state).toBe('paused')
  expect(session.statuses).toContain('rex: policy changed since master tree t1 started; confirm it with /rex policy confirm')
  expect(lineOf(status, 'status')).toBe('planned')
  expect(session.book().jobs).toHaveLength(1)
})

test('a paused tree runs again when the person confirms the policy it was pinned to', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))

  await $.command.run(rex('policy set merge never'))
  const back = await $.command.run(rex('policy set merge ask'))
  const stateBefore = session.book().trees[0]?.state
  const confirmed = await $.command.run(rex('policy confirm'))
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)

  expect(back.text).toContain('paused=none\n')
  expect(stateBefore).toBe('paused')
  expect(confirmed.text).toContain('resumed=t1\nstill_paused=none\n')
  expect(session.book().trees[0]?.state).toBe('running')
  expect(lineOf(built, 'status')).toBe('created')
  await settleUntil(session.clock, () => session.woken().length > 0)
  const woken = session.woken()[0] ?? ''
  expect(woken).toMatch(/^rex: the person confirmed the changed policy\. The tree now:\n/)
  expect(woken).toContain('\ntree=t1\ntree_state=running\n')
})

test('a confirmed new policy re-pins a paused tree of this session, which runs on with its builders and the main loop as its master', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))
  const first = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, first)

  await $.command.run(rex('policy set merge never'))
  const confirmed = await $.command.run(rex('policy confirm'))
  const second = await $.tool.call({ tool: BUILD, ...KIND, request: 'two', consequence: 'low' })
  await planAs($, second)

  const hash = /hash=(\S+)/.exec(confirmed.text ?? '')?.[1] ?? ''
  const tree = session.book().trees[0]
  expect(confirmed.text).toContain('resumed=t1\nstill_paused=none\n')
  expect([tree?.state, tree?.policyHash, tree?.master]).toEqual(['running', hash, 'main'])
  expect(tree?.builders.map(builder => builder.agentId)).toEqual([lineOf(first, 'builder'), lineOf(second, 'builder')])
  expect(session.book().jobs.map(job => job.merge?.answer ?? null)).toEqual([null, 'no'])
})

test('authority_guard_that_throws_refuses: policy-merge that throws refuses the merge', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))
  const built = await $.tool.call({ tool: BUILD, ...KIND, request: 'one', consequence: 'low' })
  await planAs($, built)
  session.breakStore()

  const merge = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  expect(merge.deny).toBe('rex gate: policy-merge failed (HooksError); refused')
})

test('/rex policy and /rex answer run only on the person\'s own typing', async ($, on) => {
  await treeWith($, on, {}, [])

  const policy = await $.command.run(rex('policy set merge never', { kind: 'sdk' }))
  const answer = await $.command.run(rex('answer q1 Merge', { kind: 'peer' }))

  expect(policy.text).toBe('rex policy: only your own typing may run this, got origin sdk')
  expect(answer.text).toBe('rex answer: only your own typing may run this, got origin peer')
})

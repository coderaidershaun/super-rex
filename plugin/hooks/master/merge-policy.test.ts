// Proves the policy's rules over rex_merge of a tree job: the merge order, a changed policy, the paths that always ask, the checks a policy answer needs, and the person's answer to a merge question.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import type { ProcessCall } from '../book/kit'
import { ROOT } from '../book/kit'
import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, lineOf, MASTER, masterWorld, POLICY_KEY, ran, resultText, rex, settleUntil, START, treeBook } from './master-kit'
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

test('merge_after_holds_a_merge', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))
  const first = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'one', consequence: 'low' })
  const firstJob = lineOf(first, 'job') ?? ''
  const second = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'two', consequence: 'low', merge_after: [firstJob] })
  const secondBuilder = lineOf(second, 'builder') ?? ''

  const held = await $.tool.call({ tool: MERGE, agentId: secondBuilder })
  const asked = await $.tool.call({ tool: MERGE, agentId: lineOf(first, 'builder') ?? '' })
  const firstQuestion = lineOf(asked, 'question') ?? ''
  await $.command.run(rex(`answer ${firstQuestion} Merge`))
  const merged = await $.tool.call({ tool: MERGE, agentId: lineOf(first, 'builder') ?? '' })
  const after = await $.tool.call({ tool: MERGE, agentId: secondBuilder })

  expect(held.deny).toBe(`rex_merge: job ${lineOf(second, 'job') ?? ''} merges after job ${firstJob}, got ${firstJob} created and not merged; call again after it merges`)
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
  const built = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'one', consequence: 'low' })
  const builder = lineOf(built, 'builder') ?? ''

  const asked = await $.tool.call({ tool: MERGE, agentId: builder })
  const id = lineOf(asked, 'question') ?? ''
  const again = await $.tool.call({ tool: MERGE, agentId: builder })
  const answered = await $.command.run(rex(`answer ${id} merge`))
  await settleUntil(session.clock, () => session.sent.length > 0)
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
  expect(session.sent.map(sent => [sent.to, sent.text])).toEqual([[MASTER, `rex: question ${id} is answered: Merge. Read it with rex_status.`]])
})

test('a policy answer merges only after each merge check passes in the job worktree', async ($, on) => {
  const calls: ProcessCall[] = []
  const failing: Rule = { starts: ['cargo', 'test'], answer: { exitCode: 101, stdout: '', stderr: 'test parser ... FAILED\n' } }
  const session = await treeWith($, on, { merge: 'allowed_if_checks_pass', merge_checks: [['cargo', 'test']] }, [failing, ...mergeRules('src/a.rs\n')], calls)
  const built = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'one', consequence: 'low' })
  const job = lineOf(built, 'job') ?? ''

  const refused = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  expect(refused.deny).toBe(`rex_merge: merge check cargo test failed in ${ROOT}.worktrees/${job}, got exit 101: test parser ... FAILED; fix it and call again`)
  expect(calls.find(call => call.argv[0] === 'cargo')?.cwd).toBe(`${ROOT}.worktrees/${job}`)
  expect(calls.some(call => call.argv.includes('push'))).toBe(false)
  expect(session.book().jobs[0]?.deliveries).toEqual([])
})

test('a policy answer with passing checks merges', async ($, on) => {
  const session = await treeWith($, on, { merge: 'allowed_if_checks_pass', merge_checks: [['cargo', 'test']] }, [{ starts: ['cargo', 'test'], answer: ran() }, ...mergeRules('src/a.rs\n')])
  const built = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'one', consequence: 'low' })

  const merged = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  expect(lineOf(merged, 'merged')).toBe('yes')
  expect(session.book().jobs[0]?.deliveries.map(delivery => delivery.mergeCommit)).toEqual(['c1'])
})

test('a policy answer with no merge checks refuses', async ($, on) => {
  await treeWith($, on, { merge: 'allowed_if_checks_pass', merge_checks: [] }, mergeRules('src/a.rs\n'))
  const built = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'one', consequence: 'low' })

  const refused = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  expect(refused.deny).toBe('rex_merge: policy allowed_if_checks_pass needs merge_checks, got none')
})

test('a diff that touches always_ask_paths turns a policy answer into a merge question', async ($, on) => {
  const calls: ProcessCall[] = []
  const session = await treeWith($, on, { merge: 'allowed_if_checks_pass', merge_checks: [['cargo', 'test']] }, [{ starts: ['cargo'], answer: ran() }, ...mergeRules('src/a.rs\ninfra/main.tf\n')], calls)
  const built = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'one', consequence: 'low' })

  const asked = await $.tool.call({ tool: MERGE, agentId: lineOf(built, 'builder') ?? '' })

  expect(resultText(asked)).toBe(`merged=no\nquestion=${session.book().questions[0]?.id ?? ''}\n`)
  expect(session.book().questions[0]?.question).toContain('infra/main.tf')
  expect(session.book().questions[0]?.mergeFor).toBe(lineOf(built, 'job'))
  expect(calls.some(call => call.argv.includes('push') || call.argv[0] === 'cargo')).toBe(false)
})

test('changed_policy_halts_tree', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))
  const built = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'one', consequence: 'low' })
  const builder = lineOf(built, 'builder') ?? ''

  const set = await $.command.run(rex('policy set merge never'))
  const build = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'two', consequence: 'low' })
  const merge = await $.tool.call({ tool: MERGE, agentId: builder })
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: builder })

  const hash = /hash=(\S+)/.exec(set.text ?? '')?.[1] ?? ''
  expect(set.text).toContain('merge=never\n')
  expect(set.text).toContain('paused=t1\n')
  expect(build.deny).toBe(`rex: policy changed since this master started, got hash ${hash}; confirm it with /rex policy confirm`)
  expect(merge.deny).toBe(`rex: policy changed since this master started, got hash ${hash}; confirm it with /rex policy confirm`)
  expect(session.book().trees[0]?.state).toBe('paused')
  expect(session.statuses).toContain('rex: policy changed since master t1 started; /rex policy confirm, then /rex master resume')
  expect(lineOf(status, 'status')).toBe('created')
  expect(session.book().jobs).toHaveLength(1)
})

test('a paused tree runs again when the person confirms the policy it was pinned to', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))

  await $.command.run(rex('policy set merge never'))
  const back = await $.command.run(rex('policy set merge ask'))
  const stateBefore = session.book().trees[0]?.state
  const confirmed = await $.command.run(rex('policy confirm'))
  const built = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'one', consequence: 'low' })

  expect(back.text).toContain('paused=none\n')
  expect(stateBefore).toBe('paused')
  expect(confirmed.text).toContain('resumed=t1\nstill_paused=none\n')
  expect(session.book().trees[0]?.state).toBe('running')
  expect(lineOf(built, 'status')).toBe('created')
})

test('a confirmed new policy re-pins a paused tree of this session, which runs on with its live master and builders', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))
  const first = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'one', consequence: 'low' })

  await $.command.run(rex('policy set merge never'))
  const confirmed = await $.command.run(rex('policy confirm'))
  const second = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'two', consequence: 'low' })

  const hash = /hash=(\S+)/.exec(confirmed.text ?? '')?.[1] ?? ''
  const tree = session.book().trees[0]
  expect(confirmed.text).toContain('resumed=t1\nstill_paused=none\n')
  expect([tree?.state, tree?.policyHash, tree?.master]).toEqual(['running', hash, MASTER])
  expect(tree?.builders.map(builder => builder.agentId)).toEqual([lineOf(first, 'builder'), lineOf(second, 'builder')])
  expect(session.book().jobs.map(job => job.merge?.answer ?? null)).toEqual([null, 'no'])
})

test('authority_guard_that_throws_refuses: policy-merge that throws refuses the merge', async ($, on) => {
  const session = await treeWith($, on, { merge: 'ask' }, mergeRules('src/a.rs\n'))
  const built = await $.tool.call({ tool: BUILD, agentId: MASTER, request: 'one', consequence: 'low' })
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

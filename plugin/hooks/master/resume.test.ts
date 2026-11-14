// Proves /rex master resume: the tree, its jobs and questions come back from rex tree show, an open job's builder starts again or waits on a question by policy, a new master starts with a digest, and no second tree.open reaches the store.
import { expect, test } from 'claude-code/testing'

import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { ROOT, SESSION } from '../book/kit'
import { sharedJson } from '../book/shared'
import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, masterWorld, POLICY_KEY, ran, rex, settleUntil, START } from './master-kit'
import type { Rule } from './master-kit'

const OLD_SESSION = '11111111-2222-4333-8444-555555555555'
const JOB = '0gaq7z0001'
const QUESTION = 'q1-555555555555'
const CHARTER = 'Ship the parser.\n\n## Constraints\n\n- Keep infra/ as it is.\n'

function field(value: unknown, key: string): unknown {
  return typeof value === 'object' && value !== null ? Reflect.get(value, key) : undefined
}

/** The job of the shared job-show fixture, which the binary prints, moved into the stored tree. */
function storedJob(): Record<string, unknown> {
  const job = field(field(field(sharedJson('fixtures/shared/calls/job-show.json'), 'expect'), 'stdout_json_contains'), 'job')
  return {
    ...Object.fromEntries(typeof job === 'object' && job !== null ? Object.entries(job) : []),
    id: JOB,
    status: 'planned',
    stored_status: 'planned',
    origin: 'master',
    session_id: OLD_SESSION,
    builder_agent_id: 'b7',
    tree: 't1',
    request: 'Fix the parser',
    restatement: null,
    merge: { answer: null, source: null, question: null },
    branch: `rex/${JOB}`,
    worktree: `${ROOT}.worktrees/${JOB}`,
    size: 'small',
    consequence: 'low',
    created_at: '2026-11-13T19:00:00.000Z',
    updated_at: '2026-11-13T19:30:00.000Z',
  }
}

function treeShow(policyHash: string): ProcessAnswer {
  const shown = {
    tree: { id: 't1', session_id: OLD_SESSION, master_agent_id: 'm9', charter: CHARTER, charter_hash: 'sha256:c', policy_hash: policyHash, state: 'running', cap_usd: 150, report_file: null, started_at: '2026-11-13T19:00:00.000Z', closed_at: null },
    jobs: [storedJob()],
    questions: [
      { id: QUESTION, tree: 't1', job: JOB, merge_for: null, question: 'Which parser?', options: ['Old', 'New'], recommendation: 'New', category: 'scope', asked_at: '2026-11-13T19:10:00.000Z', answer: null, answer_source: null, answered_at: null },
    ],
  }
  return ran(`${JSON.stringify(shown)}\n`)
}

/** The kinds of every event the mod sent to `rex ingest`, beside the answers of the rules. */
function recording(rules: readonly Rule[], calls: ProcessCall[], kinds: string[]): (call: ProcessCall) => ProcessAnswer {
  const answer = byRule(rules, calls)
  return call => {
    if (call.argv[0] === 'rex' && call.argv[1] === 'ingest') {
      const batch: unknown = JSON.parse(call.stdin ?? '{}')
      const events: unknown = typeof batch === 'object' && batch !== null ? Reflect.get(batch, 'events') : undefined
      for (const event of Array.isArray(events) ? events : []) {
        kinds.push(String(typeof event === 'object' && event !== null ? Reflect.get(event, 'kind') : ''))
      }
    }
    return answer(call)
  }
}

async function resumed(on: Parameters<typeof masterWorld>[0], keys: Partial<Policy>) {
  const policy = await confirmedPolicy(keys)
  const calls: ProcessCall[] = []
  const kinds: string[] = []
  const rules: readonly Rule[] = [
    { starts: ['rex', 'tree', 'show', '--json'], answer: treeShow(policy.confirmed_hash ?? '') },
    { starts: ['git', '-C', ROOT, 'worktree', 'list', '--porcelain'], answer: ran(`worktree ${ROOT}\nHEAD abc\n\nworktree ${ROOT}.worktrees/${JOB}\nHEAD abc\nbranch refs/heads/rex/${JOB}\n`) },
    ...buildRules('model-high-effort-high'),
  ]
  const session = masterWorld(on, { store: { [POLICY_KEY]: policy }, processAnswer: recording(rules, calls, kinds) })
  return { session, policy, calls, kinds }
}

test('resume_reattaches_questions_and_continues_or_asks: on_dead_builder continue starts the job a new builder', async ($, on) => {
  const { session, policy, calls, kinds } = await resumed(on, { on_dead_builder: 'continue' })
  await $.session.start(START)

  const answered = await $.command.run(rex('master resume'))
  await session.clock.advance(10_000)
  await settleUntil(session.clock, () => kinds.includes('job.rebind'))

  const book = session.book()
  const tree = book.trees[0]
  const builder = session.spawned.find(spawn => spawn.description === `rex builder ${JOB}`)
  const master = session.spawned.find(spawn => spawn.description === 'rex master t1')
  expect(answered.text).toBe(`tree=t1\nmaster=m1\ncontinued=${JOB}\nasked=none\n`)
  expect([tree?.session, tree?.state, tree?.policyHash, tree?.master, tree?.charter]).toEqual([SESSION, 'running', policy.confirmed_hash, 'm1', CHARTER])
  expect(tree?.builders).toEqual([{ job: JOB, agentId: 'b2' }])
  expect(book.jobs.map(job => [job.id, job.status, job.owner, job.worktree])).toEqual([[JOB, 'planned', { session: SESSION, builder: 'b2' }, `${ROOT}.worktrees/${JOB}`]])
  expect(book.questions.map(question => [question.id, question.job, question.answer])).toEqual([[QUESTION, JOB, null]])
  expect(master?.prompt).toContain(`<charter>\n${CHARTER}\n</charter>`)
  expect(master?.prompt).toContain(`job ${JOB} planned: its builder is gone, so a new builder continues it`)
  expect(master?.prompt).toContain(`question ${QUESTION} parked: Which parser?`)
  expect([builder?.type, builder?.cwd]).toEqual(['super-rex:model-high-effort-high', `${ROOT}.worktrees/${JOB}`])
  expect(builder?.prompt).toContain(`This job started before a restart. Continue job ${JOB} from the first step not done.`)
  expect(calls.some(call => call.argv[0] === 'rex' && call.argv[1] === 'worktree')).toBe(false)
  expect(kinds).toContain('tree.link')
  expect(kinds).not.toContain('tree.open')
})

test('resume_reattaches_questions_and_continues_or_asks: on_dead_builder ask parks a question and starts no builder', async ($, on) => {
  const { session, kinds } = await resumed(on, { on_dead_builder: 'ask' })
  await $.session.start(START)

  const answered = await $.command.run(rex('master resume'))
  await session.clock.advance(10_000)
  await settleUntil(session.clock, () => kinds.includes('question.ask'))

  const book = session.book()
  const asked = book.questions.find(question => question.id !== QUESTION)
  expect(answered.text).toBe(`tree=t1\nmaster=m1\ncontinued=none\nasked=${asked?.id ?? ''}\n`)
  expect([asked?.tree, asked?.job, asked?.options, asked?.category]).toEqual(['t1', JOB, ['Start a new builder', 'Leave it blocked'], 'other'])
  expect(session.spawned.map(spawn => spawn.description)).toEqual(['rex master t1'])
  expect(book.jobs[0]?.owner).toEqual({ session: OLD_SESSION, builder: 'b7' })
  expect(book.trees[0]?.builders).toEqual([])
  expect(kinds).not.toContain('tree.open')
})

test('/rex master resume refuses while a tree of this session is live, and with no open tree in the store', async ($, on) => {
  const policy = await confirmedPolicy()
  const empty = ran('{"tree":null,"jobs":[],"questions":[]}\n')
  const session = masterWorld(on, { store: { [POLICY_KEY]: policy }, processAnswer: byRule([{ starts: ['rex', 'tree', 'show'], answer: empty }]) })
  await $.session.start(START)

  const none = await $.command.run(rex('master resume'))
  await $.command.run(rex('master Ship the parser'))
  const live = await $.command.run(rex('master resume'))

  expect(none.text).toBe('rex master resume: no open tree in the store of this repo, got none; start one with /rex master <goal>')
  expect(live.text).toBe(`rex master resume: tree ${session.book().trees[0]?.id ?? ''} is live in this session, got master m1; use /rex policy confirm`)
})

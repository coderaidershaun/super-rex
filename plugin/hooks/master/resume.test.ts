// Proves the resume of a tree through rex_tree_open after the person's /super-rex:orchestration-master resume: the tree, its jobs and questions come back from rex tree show with the main loop as master, the confirmed tree_usd as its cap and each open job's stored spend, an open job's builder starts again or waits on a question by policy, the answer's brief carries a digest, and no second tree.open reaches the store; a resume of the live tree revives only builders the engine no longer lists, and a new process ends the runs of the agents it lost.
import { expect, test } from 'claude-code/testing'

import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { ROOT, SESSION } from '../book/kit'
import { sharedJson } from '../book/shared'
import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, masterWorld, openedTree, placesRules, POLICY_KEY, ran, resultText, rex, settleUntil, START } from './master-kit'
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

/** The answer of a resume up to the blank line before its brief. */
function headOf(result: unknown): string {
  return resultText(result).split('\n\n')[0] ?? ''
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

const JOB_WORKTREE_LISTED = `worktree ${ROOT}\nHEAD abc\n\nworktree ${ROOT}.worktrees/${JOB}\nHEAD abc\nbranch refs/heads/rex/${JOB}\n`

async function resumed(on: Parameters<typeof masterWorld>[0], keys: Partial<Policy>, before: readonly Rule[] = [], listed = JOB_WORKTREE_LISTED) {
  const policy = await confirmedPolicy(keys)
  const calls: ProcessCall[] = []
  const kinds: string[] = []
  const rules: readonly Rule[] = [
    ...before,
    { starts: ['rex', 'tree', 'show', '--json'], answer: treeShow(policy.confirmed_hash ?? '') },
    { starts: ['git', '-C', ROOT, 'worktree', 'list', '--porcelain'], answer: ran(listed) },
    { starts: ['rex', 'limit'], answer: ran(`${JSON.stringify({ job: JOB, spent_usd: 1.5, limit_usd: 100_000, source: 'default' })}\n`) },
    ...buildRules('model-high-effort-high'),
  ]
  const session = masterWorld(on, { store: { [POLICY_KEY]: policy }, processAnswer: recording(rules, calls, kinds) })
  return { session, policy, calls, kinds }
}

test('resume_reattaches_questions_and_continues_or_asks: on_dead_builder continue starts the job a new builder', async ($, on) => {
  const { session, policy, calls, kinds } = await resumed(on, { on_dead_builder: 'continue', tree_usd: 400 })
  session.limitAnswers.budget = '$1,500'
  await $.session.start(START)

  const answered = await openedTree($, 'resume')
  await session.clock.advance(10_000)
  await settleUntil(session.clock, () => kinds.includes('job.rebind'))

  const book = session.book()
  const tree = book.trees[0]
  const builder = session.spawned.find(spawn => spawn.description === `rex builder ${JOB}`)
  const brief = resultText(answered)
  expect(headOf(answered)).toBe(`tree=t1\nstate=running\nexisting=no\nresumed=yes\ncontinued=${JOB}\nasked=none\nmerge=ask\nbudget_usd=100000.00\ntime=unlimited\nends_at=none\nperson=yes`.replace('budget_usd=100000.00', 'budget_usd=1500.00'))
  expect([tree?.session, tree?.state, tree?.policyHash, tree?.master, tree?.charter]).toEqual([SESSION, 'running', policy.confirmed_hash, 'main', CHARTER])
  // The budget is the person's answer to the dialog of the resume, not the stored cap_usd of 150 nor tree_usd of 400; the spend is what rex limit read from the store.
  expect(tree?.capNano).toBe(1_500_000_000_000)
  expect(book.jobs.map(job => job.spentNano)).toEqual([1_500_000_000])
  expect(tree?.builders).toEqual([{ job: JOB, agentId: 'b1' }])
  expect(book.jobs.map(job => [job.id, job.status, job.owner, job.worktree])).toEqual([[JOB, 'planned', { session: SESSION, builder: 'b1' }, `${ROOT}.worktrees/${JOB}`]])
  expect(book.questions.map(question => [question.id, question.job, question.answer])).toEqual([[QUESTION, JOB, null]])
  expect(brief).toContain(`<charter>\n${CHARTER}\n</charter>`)
  expect(brief).toContain(`job ${JOB} planned: its builder is gone, so a new builder continues it`)
  expect(brief).toContain(`question ${QUESTION} parked: Which parser?`)
  expect(book.treeStartPrompt).toBeNull()
  expect([builder?.type, builder?.cwd]).toEqual(['super-rex:model-high-effort-high', `${ROOT}.worktrees/${JOB}`])
  expect(builder?.prompt).toContain(`This job started before a restart. Continue job ${JOB} from the first step not done.`)
  expect(calls.some(call => call.argv[0] === 'rex' && call.argv[1] === 'worktree')).toBe(false)
  expect(kinds).toContain('tree.link')
  expect(kinds).not.toContain('tree.open')
})

test('a builder restarted on resume gets its worktree back on the job branch that outlived it, at the tip of that branch', async ($, on) => {
  const tip = 'c'.repeat(40)
  const branchTip: Rule = { starts: ['git', '-C', ROOT, 'rev-parse', '--verify', '--quiet', `refs/heads/rex/${JOB}`], answer: ran(`${tip}\n`) }
  const { session, calls, kinds } = await resumed(on, { on_dead_builder: 'continue' }, [branchTip], `worktree ${ROOT}\nHEAD abc\n`)
  await $.session.start(START)

  await openedTree($, 'resume')
  await session.clock.advance(10_000)
  await settleUntil(session.clock, () => kinds.includes('job.rebind'))

  const added = calls.filter(call => call.argv[0] === 'rex' && call.argv[1] === 'worktree').map(call => call.argv.slice(1))
  expect(added).toEqual([['worktree', 'add', ROOT, `${ROOT}.worktrees/${JOB}`, tip, '--branch', `rex/${JOB}`]])
  expect(session.spawned.find(spawn => spawn.description === `rex builder ${JOB}`)?.cwd).toBe(`${ROOT}.worktrees/${JOB}`)
})

test('a git call that throws while a builder restarts on resume leaves that job for the person, and the tree still resumes with its watchdog', async ($, on) => {
  const hung: Rule = { starts: ['git', '-C', ROOT, 'worktree', 'list', '--porcelain'], answer: { reject: 'the process ran past its timeout of 120000 ms' } }
  const { session } = await resumed(on, { on_dead_builder: 'continue' }, [hung])
  await $.session.start(START)

  const answered = await openedTree($, 'resume')

  expect(headOf(answered)).toBe('tree=t1\nstate=running\nexisting=no\nresumed=yes\ncontinued=none\nasked=none\nmerge=ask\nbudget_usd=100000.00\ntime=unlimited\nends_at=none\nperson=yes')
  expect(session.statuses.some(line => line.startsWith(`rex: job ${JOB} of tree t1 got no new builder:`) && line.includes('timeout'))).toBe(true)
  expect(session.spawned.some(spawn => spawn.description === `rex builder ${JOB}`)).toBe(false)
})

test('resume_reattaches_questions_and_continues_or_asks: on_dead_builder ask parks a question and starts no builder', async ($, on) => {
  const { session, kinds } = await resumed(on, { on_dead_builder: 'ask' })
  await $.session.start(START)

  const answered = await openedTree($, 'resume')
  await session.clock.advance(10_000)
  await settleUntil(session.clock, () => kinds.includes('question.ask'))

  const book = session.book()
  const asked = book.questions.find(question => question.id !== QUESTION)
  expect([lineOf(answered, 'continued'), lineOf(answered, 'asked')]).toEqual(['none', asked?.id])
  expect([asked?.tree, asked?.job, asked?.options, asked?.category]).toEqual(['t1', JOB, ['Start a new builder', 'Leave it blocked'], 'other'])
  expect(session.spawned).toEqual([])
  expect(book.jobs[0]?.owner).toEqual({ session: OLD_SESSION, builder: 'b7' })
  expect(book.trees[0]?.builders).toEqual([])
  expect(kinds).not.toContain('tree.open')
})

test('resume refuses with no open tree in the store, and while a tree of this session is live it resumes that tree in place and reads no store', async ($, on) => {
  const policy = await confirmedPolicy()
  const empty = ran('{"tree":null,"jobs":[],"questions":[]}\n')
  const calls: ProcessCall[] = []
  const session = masterWorld(on, { store: { [POLICY_KEY]: policy }, processAnswer: byRule([{ starts: ['rex', 'tree', 'show'], answer: empty }, ...placesRules('origin')], calls) })
  await $.session.start(START)

  const none = await openedTree($, 'resume')
  const opened = await openedTree($, 'Ship the parser')
  const shows = calls.filter(call => call.argv[1] === 'tree').length
  const live = await openedTree($, 'resume')

  expect(none.deny).toBe('rex_tree_open resume: no open tree in the store of this repo, got none; the person starts one with /super-rex:orchestration-master <goal>')
  expect(headOf(live)).toBe(`tree=${lineOf(opened, 'tree') ?? ''}\nstate=running\nexisting=yes\nresumed=yes\ncontinued=none\nasked=none\nmerge=ask\nbudget_usd=100000.00\ntime=unlimited\nends_at=none\nperson=yes`)
  expect(calls.filter(call => call.argv[1] === 'tree').length).toBe(shows)
  expect(session.book().trees.map(tree => tree.charter)).toEqual(['Ship the parser'])
})

test('on_dead_builder ask on resume: the answer Start a new builder starts one for the job', async ($, on) => {
  const { session, kinds } = await resumed(on, { on_dead_builder: 'ask' })
  await $.session.start(START)

  const answered = await openedTree($, 'resume')
  await settleUntil(session.clock, () => kinds.includes('question.ask'))
  const asked = lineOf(answered, 'asked') ?? ''
  const reply = await $.command.run(rex(`answer ${asked} Start a new builder`))
  await settleUntil(session.clock, () => session.spawned.length > 0)

  const book = session.book()
  expect(reply.text).toBe(`question=${asked}\nanswer=Start a new builder\n`)
  expect(session.spawned.map(spawn => [spawn.description, spawn.cwd])).toEqual([[`rex builder ${JOB}`, `${ROOT}.worktrees/${JOB}`]])
  expect(book.trees[0]?.builders).toEqual([{ job: JOB, agentId: 'b1' }])
  expect(book.jobs[0]?.owner).toEqual({ session: SESSION, builder: 'b1' })
  expect(book.trees[0]?.notes.map(note => note.text)).toEqual([`rex: started a new builder b1 for this job (the person's answer Start a new builder to question ${asked})`])
})

test('resume typed while the tree is live leaves a listed builder alone, and asks for one the engine no longer lists', async ($, on) => {
  const { session } = await resumed(on, { on_dead_builder: 'ask' })
  await $.session.start(START)
  await openedTree($, 'Ship the parser')
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })
  const job = lineOf(built, 'job') ?? ''

  const listed = await openedTree($, 'resume')
  session.agents.splice(0)
  const gone = await openedTree($, 'resume')

  const asked = session.book().questions
  expect([lineOf(listed, 'resumed'), lineOf(listed, 'continued'), lineOf(listed, 'asked')]).toEqual(['yes', 'none', 'none'])
  expect([lineOf(gone, 'existing'), lineOf(gone, 'resumed'), lineOf(gone, 'continued'), lineOf(gone, 'asked')]).toEqual(['yes', 'yes', 'none', asked[0]?.id])
  expect(asked.map(question => [question.job, question.question, question.options])).toEqual([[job, `The builder of job ${job} ended with the last session. Start a new builder for it?`, ['Start a new builder', 'Leave it blocked']]])
  expect(resultText(gone)).toContain(`job ${job} created: its builder is gone, and a question asks the person`)
  expect(session.spawned.filter(spawn => spawn.description.startsWith('rex builder '))).toHaveLength(1)
})

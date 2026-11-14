// Proves the seam to rex ingest: the batch and the call equal the shared fixture files, and flushes come at a run's end, a job's change and every ten seconds.
import { expect, test } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { acked, emptyBook, nextBatch, withTranscript } from '../book/events'
import { openJobIn, questionIn } from '../book/jobs'
import { CONFIG_ON, ingestAnswer, ROOT, SESSION, spawnInput, START_MS, world } from '../book/kit'
import type { ProcessAnswer } from '../book/kit'
import { promptIn, sessionIn } from '../book/session'
import { sharedJson } from '../book/shared'
import { MERGE_NO, MERGE_QUESTION, MERGE_YES } from '../observe/index'
import { batchText } from './batch'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const STREAM = 'aaaaaaaaaaaaaaaa'
const SECRET = '1'.repeat(64)
const REQUEST = 'Fix the parser. Merge to main: yes'

/** The book the shared example batch comes from: a session, the job-start prompt, and the job it opened. */
function exampleBook(): RexBook {
  const setup = { session: SESSION, storage: 'on', jobsFromSdk: true, inert: null, claudeCode: '2.1.323', plugin: '0.8.0' } as const
  const started = withTranscript(
    sessionIn(emptyBook(), setup, START_MS, { stream: STREAM, secret: SECRET }),
    `/home/person/.claude/projects/-repo/${SESSION}.jsonl`,
  )
  const prompted = promptIn(
    started,
    {
      text: `/super-rex:orchestration-builder ${REQUEST}`,
      origin: 'composer',
      command: { name: 'super-rex:orchestration-builder', args: REQUEST },
      restatement: 'Goal: fix the parser. Missing: budget.',
      isRecorded: true,
      jobStart: { origin: 'composer', text: REQUEST, restatement: 'Goal: fix the parser. Missing: budget.', restatementSkipped: null },
    },
    START_MS + 1000,
  )
  const input = {
    builder: 'main',
    tree: null,
    origin: 'composer',
    request: REQUEST,
    repo: ROOT,
    base: 'main',
    type: 'fix',
    size: 'small',
    consequence: 'low',
    langs: ['rust'],
    prefs: { duration: null, budget: null, autonomy: null },
    limit: { nano: 10_000_000_000, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  return openJobIn(prompted, input, '0gaq820001', START_MS + 2000).book
}

test('emitted_batch_equals_shared_fixture', async ($, on) => {
  const book = exampleBook()
  expect(JSON.parse(batchText({ session: SESSION, stream: STREAM, secret: SECRET, events: nextBatch(book, 500) }))).toEqual(
    sharedJson('fixtures/shared/ingest/batch-example.json'),
  )

  const call = sharedJson('fixtures/shared/calls/ingest.json')
  expect(call).toMatchObject({ argv: ['rex', 'ingest'], stdin_file: 'fixtures/shared/ingest/batch-example.json', expect: { exit: 0, stdout: 'last_sequence=4\n' } })
  const files = ['fixtures/shared/ingest/batch-open-stream.json', 'fixtures/shared/ingest/batch-example.json']
  const session = world(on, {
    config: CONFIG_ON,
    seed: book,
    processAnswer: one => {
      const file = files.shift()
      if (file === undefined) {
        return { reject: `a call no test expected: ${one.argv.join(' ')}` }
      }
      expect(one.argv).toEqual(['rex', 'ingest'])
      expect(one.cwd).toBe(ROOT)
      expect(JSON.parse(one.stdin ?? '')).toEqual(sharedJson(file))
      return ingestAnswer(files.length === 1 ? 0 : 4)
    },
  })
  on('tool.call', () => ({ result: {} }))

  await $.session.start(START)
  await $.tool.call({ tool: 'Bash', command: 'ls' })
  await session.clock.settle()

  expect(session.calls).toHaveLength(2)
  expect(session.book().ingest?.nextSequence).toBe(5)
  expect(session.book().ingest?.queue).toEqual([])
})

test('a parked question goes to the store under an id of its session, as the shared question batch holds it', () => {
  const asked = questionIn(
    acked(acked(exampleBook(), 0), 4),
    { tree: null, job: '0gaq820001', question: 'Merge job 0gaq820001 (rex/0gaq820001) to main of /home/person/repo?', options: ['Merge', 'Keep the branch'], recommendation: 'Merge', category: 'merge', mergeFor: '0gaq820001' },
    START_MS + 3000,
  ).book

  expect(JSON.parse(batchText({ session: SESSION, stream: STREAM, secret: SECRET, events: nextBatch(asked, 500) }))).toEqual(sharedJson('fixtures/shared/ingest/batch-question.json'))
})

/** Answers every batch as stored: the highest sequence it holds, or the last one stored before. */
function storeAll(): (call: { readonly stdin: string | undefined }) => ProcessAnswer {
  let last = 4
  return call => {
    const sent: unknown = JSON.parse(call.stdin ?? '{}')
    const events: unknown = typeof sent === 'object' && sent !== null ? Reflect.get(sent, 'events') : undefined
    const seqs = Array.isArray(events) ? events.map((event: unknown) => (typeof event === 'object' && event !== null ? Number(Reflect.get(event, 'seq')) : 0)) : []
    last = Math.max(last, ...seqs)
    return ingestAnswer(last)
  }
}

function sentKinds(stdin: string | undefined): string[] {
  const sent: unknown = JSON.parse(stdin ?? '{}')
  const events: unknown = typeof sent === 'object' && sent !== null ? Reflect.get(sent, 'events') : undefined
  return Array.isArray(events) ? events.map((event: unknown) => (typeof event === 'object' && event !== null ? String(Reflect.get(event, 'kind')) : '')) : []
}

test('flush_on_agent_stop_state_change_and_ten_seconds', async ($, on) => {
  const session = world(on, { config: CONFIG_ON, seed: acked(acked(exampleBook(), 0), 4), processAnswer: storeAll() })
  on('agent.spawn', (_$, e) => ({ model: 'claude-haiku-5-5', agentId: e.description === 'first' ? 'a1' : 'a2' }))
  on('turn.complete', (_$, e) => ({ text: e.answer }))
  on('tool.call', () => ({ result: { questions: [], answers: { [MERGE_QUESTION]: MERGE_YES } } }))

  await $.session.start(START)
  expect(session.calls.map(call => sentKinds(call.stdin))).toEqual([[]])

  await $.agent.spawn(spawnInput({ description: 'first', prompt: 'Task: T1\nOutcome: o\nScope: s\nContext: c\nAcceptance: `cargo test` passes.\nLimits: l' }))
  await session.clock.settle()
  expect(session.calls).toHaveLength(1)

  await $.turn.complete({ agentId: 'a1', reason: 'answer', answer: 'done', durationMs: 5, isAborted: false, turnId: 't1' })
  await session.clock.settle()
  expect(session.calls.map(call => sentKinds(call.stdin))).toEqual([[], ['run.start', 'run.end']])

  await session.clock.advance(9_000)
  expect(session.calls).toHaveLength(2)

  await $.agent.spawn(spawnInput({ description: 'second', prompt: 'Task: T1\nOutcome: o\nScope: s\nContext: c\nAcceptance: `cargo test` passes.\nLimits: l' }))
  await session.clock.advance(1_000)
  expect(session.calls.map(call => sentKinds(call.stdin))).toEqual([[], ['run.start', 'run.end'], ['run.start']])

  await $.tool.call({
    tool: 'AskUserQuestion',
    questions: [
      {
        question: MERGE_QUESTION,
        header: 'Merge',
        multiSelect: false,
        options: [
          { label: MERGE_YES, description: 'Merge when the checks pass' },
          { label: MERGE_NO, description: 'Keep the branch' },
        ],
      },
    ],
  })
  await session.clock.settle()
  expect(session.calls.map(call => sentKinds(call.stdin))).toEqual([[], ['run.start', 'run.end'], ['run.start'], ['job.merge_answer']])
  expect(session.book().jobs[0]?.merge).toMatchObject({ answer: 'yes', source: 'dialog' })
  expect(session.book().ingest?.queue).toEqual([])
})

// Proves how the mod learns where the session's main transcript is: from whichever classic hook event names it first, sent to the binary once in an event of its own, with no queued or sent event changed.
import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'
import type { On } from 'claude-code'

import type { RexBook, RexEvent } from '../../types'
import { acked, emptyBook, queued } from '../book/events'
import { CONFIG_ON, ingestAnswer, ROOT, SESSION, START_MS, world } from '../book/kit'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { transcriptIn } from '../book/session'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const PATH = `/home/person/.claude/projects/-home-person-repo/${SESSION}.jsonl`
const OTHER_PATH = '/home/person/.claude/projects/-elsewhere/other.jsonl'

type Raise = ($: Engine, path: string) => Promise<unknown>

/** The classic hook events a session raises, each with the fields its hook input needs, naming the transcript at `path`. */
const SOURCES: readonly (readonly [string, Raise])[] = [
  ['SessionStart', ($, path) => $.classic.SessionStart({ source: 'startup', session_id: SESSION, transcript_path: path })],
  ['UserPromptSubmit', ($, path) => $.classic.UserPromptSubmit({ prompt: 'Fix the parser', session_id: SESSION, transcript_path: path })],
  ['Stop', ($, path) => $.classic.Stop({ stop_hook_active: false, session_id: SESSION, transcript_path: path })],
  ['SubagentStart', ($, path) => $.classic.SubagentStart({ agent_id: 'a1', agent_type: 'general-purpose', session_id: SESSION, transcript_path: path })],
  ['Notification', ($, path) => $.classic.Notification({ message: 'waiting', notification_type: 'idle_prompt', session_id: SESSION, transcript_path: path })],
]

/** The settings hooks beneath the plugins: none, so each classic event ends with nothing to do. */
function noSettingsHooks(on: On): void {
  on('classic.SessionStart', () => ({}))
  on('classic.UserPromptSubmit', () => ({}))
  on('classic.Stop', () => ({}))
  on('classic.SubagentStart', () => ({}))
  on('classic.Notification', () => ({}))
}

/** Answers every batch as stored: the highest sequence it holds, or the last one stored before. */
function storeAll(): (call: ProcessCall) => ProcessAnswer {
  let last = 0
  return call => {
    for (const event of sentIn(call)) {
      last = Math.max(last, event.seq)
    }
    return ingestAnswer(last)
  }
}

function sentIn(call: ProcessCall): readonly RexEvent[] {
  const sent: unknown = JSON.parse(call.stdin ?? '{}')
  const events: unknown = typeof sent === 'object' && sent !== null ? Reflect.get(sent, 'events') : undefined
  return Array.isArray(events) ? events.filter((event): event is RexEvent => typeof event === 'object' && event !== null && 'seq' in event && 'kind' in event) : []
}

/** Every event sent, once each by sequence; a sequence sent again with other content fails the test. */
function sentEvents(calls: readonly ProcessCall[]): readonly RexEvent[] {
  const bySeq = new Map<number, RexEvent>()
  for (const event of calls.flatMap(sentIn)) {
    const first = bySeq.get(event.seq)
    if (first === undefined) {
      bySeq.set(event.seq, event)
    } else {
      expect(event).toEqual(first)
    }
  }
  return [...bySeq.values()]
}

function transcriptEvents(calls: readonly ProcessCall[]): readonly RexEvent[] {
  return sentEvents(calls).filter(event => event.kind === 'session.transcript')
}

for (const [name, raise] of SOURCES) {
  test(`classic ${name} names the transcript, and the mod sends its path`, async ($, on) => {
    const session = world(on, { config: CONFIG_ON, processAnswer: storeAll() })
    noSettingsHooks(on)
    await $.session.start(START)

    await raise($, PATH)
    await session.clock.advance(10_000)

    expect(session.book().transcript).toBe(PATH)
    expect(transcriptEvents(session.calls).map(event => event.data)).toEqual([{ transcript_path: PATH }])
  })
}

// In the live run of 15 November the classic SessionStart came before the book
// of its session existed, so the path it named was lost.
test('a path named before the session has its book waits for the next event, and only the first path is sent', async ($, on) => {
  const session = world(on, { config: CONFIG_ON, processAnswer: storeAll() })
  noSettingsHooks(on)
  const [early, later, last] = [SOURCES[0], SOURCES[1], SOURCES[2]]
  if (early === undefined || later === undefined || last === undefined) {
    throw new Error('three sources are listed')
  }

  await early[1]($, OTHER_PATH)
  await $.session.start(START)
  await session.clock.advance(10_000)
  expect(transcriptEvents(session.calls)).toEqual([])

  await later[1]($, PATH)
  await session.clock.advance(10_000)
  await last[1]($, OTHER_PATH)
  await session.clock.advance(10_000)

  expect(session.book().transcript).toBe(PATH)
  expect(transcriptEvents(session.calls).map(event => event.data)).toEqual([{ transcript_path: PATH }])
})

function startedBook(): RexBook {
  const base: RexBook = { ...emptyBook(), session: SESSION, storage: 'on', ingest: { stream: 'a'.repeat(16), secret: '1'.repeat(64), nextSequence: 1, acked: -1, failures: 0, resets: 0, halted: null, dropped: 0, queue: [] } }
  const started = queued(base, START_MS, 'session.start', 'main', null, { claude_code: '2.1.323', plugin: '0.8.0', storage: 'on' })
  return queued(started, START_MS + 1, 'prompt', 'main', null, { text: 'Fix the parser', origin: 'composer' })
}

test('noting the transcript changes no queued or sent event: it adds one event after them', () => {
  const waiting = startedBook()
  const sent = acked(acked(waiting, 0), 2)

  for (const before of [waiting, sent]) {
    const after = transcriptIn(before, { session: SESSION, path: PATH }, START_MS + 2)
    const queue = before.ingest?.queue ?? []
    const next = before.ingest?.nextSequence ?? 0

    expect(after.ingest?.queue.slice(0, queue.length)).toEqual(queue)
    expect(after.ingest?.queue.slice(queue.length)).toEqual([
      { seq: next, at: '2026-11-13T20:00:00.002Z', kind: 'session.transcript', agent: 'main', job: null, data: { transcript_path: PATH } },
    ])
    expect(transcriptIn(after, { session: SESSION, path: OTHER_PATH }, START_MS + 3)).toBe(after)
  }
})

// Pure changes at the start of a session, when its transcript is first named, and at each prompt the person sends, which the book records before any job exists.
import type { RexBook, RexJobStartPrompt, RexOriginKind, RexTreeStartPrompt } from '../../types'
import { emptyBook, newIngest, queued } from './events'
import type { StreamKeys } from './ids'

export type SessionSetup = {
  readonly session: string
  readonly storage: RexBook['storage']
  readonly jobsFromSdk: boolean
  readonly inert: string | null
  readonly claudeCode: string
  readonly plugin: string
}

/** A new session gets a new book and stream; a reload of the same session keeps both and records no second start. */
export function sessionIn(old: RexBook, setup: SessionSetup, at: number, keys: StreamKeys): RexBook {
  const isReload = old.session === setup.session
  const base: RexBook = isReload ? old : { ...emptyBook(), session: setup.session }
  const ingest = setup.storage === 'on' && setup.inert === null ? (base.ingest ?? newIngest(keys.stream, keys.secret)) : null
  const versions = { ...base.versions, mod: setup.plugin, claudeCode: setup.claudeCode }
  const book: RexBook = { ...base, versions, storage: setup.storage, jobsFromSdk: setup.jobsFromSdk, inert: setup.inert, ingest }
  if (isReload) {
    return book
  }
  return queued(book, at, 'session.start', 'main', null, { claude_code: setup.claudeCode, plugin: setup.plugin, storage: setup.storage })
}

/** Where a hook event of the engine says the session's main transcript is. */
export type TranscriptSeen = { readonly session: string; readonly path: string }

/**
 * Keeps the session's main transcript path the first time an event names it,
 * with an event of its own for the binary. A path of another session, such as
 * one seen before this session's book exists, an empty path, or any path after
 * the first changes nothing.
 */
export function transcriptIn(book: RexBook, seen: TranscriptSeen, at: number): RexBook {
  if (book.transcript !== null || seen.path === '' || seen.session !== book.session) {
    return book
  }
  return queued({ ...book, transcript: seen.path }, at, 'session.transcript', 'main', null, { transcript_path: seen.path })
}

export type PromptRecord = {
  readonly text: string
  readonly origin: RexOriginKind
  readonly command: { readonly name: string; readonly args: string } | null
  readonly restatement: string | null
  /** False in prompt mode `off`: nothing is kept but the job-start mark. */
  readonly isRecorded: boolean
  readonly jobStart: Omit<RexJobStartPrompt, 'at'> | null
  /** Set on the prompt that runs the master skill: the tree that opens next takes its charter from here. */
  readonly treeStart: Omit<RexTreeStartPrompt, 'at'> | null
}

export function promptIn(book: RexBook, prompt: PromptRecord, at: number): RexBook {
  const withJob = prompt.jobStart === null ? book : { ...book, jobStartPrompt: { ...prompt.jobStart, at } }
  const started = prompt.treeStart === null ? withJob : { ...withJob, treeStartPrompt: { ...prompt.treeStart, at } }
  if (!prompt.isRecorded) {
    return started
  }
  const base = { text: prompt.text, origin: prompt.origin }
  const withCommand = prompt.command === null ? base : { ...base, command: { name: prompt.command.name, args: prompt.command.args } }
  const data = prompt.restatement === null ? withCommand : { ...withCommand, restatement: prompt.restatement }
  return queued(started, at, 'prompt', 'main', null, data)
}

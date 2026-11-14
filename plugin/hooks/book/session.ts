// Pure changes at the start of a session and at each prompt the person sends, which the book records before any job exists.
import type { RexBook, RexJobStartPrompt, RexOriginKind } from '../../types'
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

export type PromptRecord = {
  readonly text: string
  readonly origin: RexOriginKind
  readonly command: { readonly name: string; readonly args: string } | null
  readonly restatement: string | null
  /** False in prompt mode `off`: nothing is kept but the job-start mark. */
  readonly isRecorded: boolean
  readonly jobStart: Omit<RexJobStartPrompt, 'at'> | null
}

export function promptIn(book: RexBook, prompt: PromptRecord, at: number): RexBook {
  const started = prompt.jobStart === null ? book : { ...book, jobStartPrompt: { ...prompt.jobStart, at } }
  if (!prompt.isRecorded) {
    return started
  }
  const base = { text: prompt.text, origin: prompt.origin }
  const withCommand = prompt.command === null ? base : { ...base, command: { name: prompt.command.name, args: prompt.command.args } }
  const data = prompt.restatement === null ? withCommand : { ...withCommand, restatement: prompt.restatement }
  return queued(started, at, 'prompt', 'main', null, data)
}

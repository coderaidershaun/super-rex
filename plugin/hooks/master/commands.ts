// /rex policy and /rex answer: the person shows, confirms or changes the policy, and answers a parked question by its option's number or text, each from their own typing only, and each wakes the agents that wait on it.
import type { CommandRunInput, CommandRunResult, EngineInterface } from 'claude-code'

import type { RexBook, RexQuestion } from '../../types'
import { answerQuestion, closeTree, fail, isOpen, readBook, repinTree } from '../book/index'
import { originRefusal, rexLine } from '../commands/index'
import type { StatusSection, Table } from '../parts'
import { afterAnswer } from './answers'
import { ANSWER_SUBCOMMAND, POLICY_SUBCOMMAND } from './entries'
import { DEFAULT_POLICY, policyLines, policyWith, PolicyShapeError } from './policy'
import type { Policy } from './policy'
import { readPolicy, writePolicy } from './store'
import type { StoredPolicy } from './store'
import { CONTINUE_TEXT, wakeLater, wakeMaster } from './wake'

function lines(rows: readonly string[]): CommandRunResult {
  return { text: rows.map(row => `${row}\n`).join('') }
}

function storedLines(stored: StoredPolicy, origin: 'stored' | 'default'): readonly string[] {
  return [`policy=${origin}`, ...policyLines(stored.policy), `hash=${stored.hash}`, `confirmed=${stored.isConfirmed ? 'yes' : 'no'}`]
}

async function originGate($: EngineInterface, event: CommandRunInput, name: 'policy' | 'answer'): Promise<CommandRunResult | null> {
  const entry = name === 'policy' ? POLICY_SUBCOMMAND : ANSWER_SUBCOMMAND
  const refusal = originRefusal(event, entry, false)
  if (refusal === null) {
    return null
  }
  await fail($, { kind: 'origin-not-person', action: `rex ${name}`, origin: event.origin.kind })
  return { text: refusal }
}

/** Live trees of this session pinned to another hash than `hash`. */
function treesOff(book: RexBook, hash: string): readonly string[] {
  return book.trees.filter(tree => tree.session === book.session && (tree.state === 'running' || tree.state === 'paused') && tree.policyHash !== hash).map(tree => tree.id)
}

async function pauseChanged($: EngineInterface, hash: string): Promise<readonly string[]> {
  const book = await readBook($)
  const changed = treesOff(book, hash)
  for (const tree of book.trees.filter(each => changed.includes(each.id) && each.state === 'running')) {
    await fail($, { kind: 'policy-changed', tree: tree.id, pinned: tree.policyHash, current: hash })
  }
  return changed
}

/**
 * Each paused tree of this session runs on under the policy the person just
 * confirmed: pinned to its hash, with its builders, its budget and its time,
 * its master woken to go on, and each builder that had stopped woken too.
 */
async function resumeConfirmed($: EngineInterface, hash: string, sections: Table<StatusSection>): Promise<readonly string[]> {
  const book = await readBook($)
  const resumed = book.trees.filter(tree => tree.session === book.session && tree.state === 'paused')
  for (const tree of resumed) {
    await repinTree($, tree.id, hash)
    await closeTree($, tree.id, 'running', null)
  }
  const now = await readBook($)
  for (const tree of now.trees.filter(each => resumed.some(paused => paused.id === each.id))) {
    wakeMaster($, now, tree, 'the person confirmed the changed policy', sections)
    // A builder that a paused tree refused, at rex_merge say, ended its turn: it goes on now, and a builder at work is left alone.
    for (const builder of tree.builders) {
      const job = now.jobs.find(each => each.id === builder.job)
      const isWorking = now.runs.some(run => run.job === builder.job && run.state === 'open')
      if (job !== undefined && isOpen(job) && !isWorking) {
        wakeLater($, builder.agentId, `rex: the person confirmed the changed policy. ${CONTINUE_TEXT(job.id)}`)
      }
    }
  }
  return resumed.map(tree => tree.id)
}

async function policyNow($: EngineInterface): Promise<{ readonly policy: Policy; readonly origin: 'stored' | 'default' }> {
  const stored = await readPolicy($)
  return stored === null ? { policy: DEFAULT_POLICY, origin: 'default' } : { policy: stored.policy, origin: 'stored' }
}

async function setKey($: EngineInterface, rest: string): Promise<CommandRunResult> {
  const space = rest.indexOf(' ')
  const key = space === -1 ? rest : rest.slice(0, space)
  const value = space === -1 ? '' : rest.slice(space + 1).trim()
  if (key === '' || value === '') {
    return { text: `rex policy: set needs a key and a value, got ${rest === '' ? 'nothing' : rest}; run /rex policy set <key> <value>` }
  }
  const now = await policyNow($)
  const changed = policyWith(now.policy, key, value)
  const written = await writePolicy($, changed, 'keep')
  const paused = await pauseChanged($, written.hash)
  return lines([...storedLines(written, 'stored'), `paused=${paused.length === 0 ? 'none' : paused.join(',')}`])
}

async function confirm($: EngineInterface, sections: Table<StatusSection>): Promise<CommandRunResult> {
  const now = await policyNow($)
  const written = await writePolicy($, now.policy, 'confirm')
  const resumed = await resumeConfirmed($, written.hash, sections)
  const waiting = treesOff(await readBook($), written.hash)
  return lines([...storedLines(written, 'stored'), `resumed=${resumed.length === 0 ? 'none' : resumed.join(',')}`, `still_paused=${waiting.length === 0 ? 'none' : waiting.join(',')}`])
}

export async function servePolicy($: EngineInterface, event: CommandRunInput, sections: Table<StatusSection>): Promise<CommandRunResult> {
  const refused = await originGate($, event, 'policy')
  if (refused !== null) {
    return refused
  }
  const args = rexLine(event).args
  const verb = args.split(' ')[0] ?? ''
  try {
    switch (verb) {
      case '':
      case 'show': {
        const stored = await readPolicy($)
        return lines(stored === null ? storedLines({ policy: DEFAULT_POLICY, hash: 'none', isConfirmed: false }, 'default') : storedLines(stored, 'stored'))
      }
      case 'confirm':
        return await confirm($, sections)
      case 'set':
        return await setKey($, args.slice(verb.length).trim())
      default:
        return { text: `rex policy: ${verb} is not show, confirm or set, got ${args}; run /rex policy show` }
    }
  } catch (error) {
    if (error instanceof PolicyShapeError) {
      return { text: `rex policy: ${error.message}; nothing was changed` }
    }
    throw error
  }
}

/** The option the person typed: its number from 1, or its text in any letter case. */
function optionOf(question: RexQuestion, typed: string): string | undefined {
  const number = /^\d+$/.test(typed) ? Number(typed) : null
  if (number !== null) {
    return question.options[number - 1]
  }
  return question.options.find(option => option.toLowerCase() === typed.toLowerCase())
}

function numbered(question: RexQuestion): string {
  return question.options.map((option, index) => `${String(index + 1)} (${option})`).join(', ')
}

/** Each question that waits, with its numbered options, so a refusal shows what can be typed. */
function waitingList(waiting: readonly RexQuestion[]): string {
  return waiting.length === 0 ? 'no question waits' : waiting.map(question => `${question.id}: ${numbered(question)}`).join('; ')
}

type Picked = { readonly question: RexQuestion; readonly typed: string } | { readonly refused: string }

/** `<id> <option>`, or `<option>` alone when exactly one question waits. */
function pickQuestion(book: RexBook, args: string): Picked {
  const space = args.indexOf(' ')
  const first = space === -1 ? args : args.slice(0, space)
  const named = book.questions.find(each => each.id === first)
  if (named !== undefined) {
    return { question: named, typed: space === -1 ? '' : args.slice(space + 1).trim() }
  }
  const waiting = book.questions.filter(each => each.answer === null)
  const only = waiting.length === 1 ? waiting[0] : undefined
  if (only !== undefined && args !== '') {
    return { question: only, typed: args }
  }
  const seen = args === '' ? 'nothing' : args
  return { refused: `rex answer: name a question and one of its options, got ${seen}; run /rex answer <id> <option number>, with ${waitingList(waiting)}` }
}

export async function serveAnswer($: EngineInterface, event: CommandRunInput): Promise<CommandRunResult> {
  const refused = await originGate($, event, 'answer')
  if (refused !== null) {
    return refused
  }
  const picked = pickQuestion(await readBook($), rexLine(event).args)
  if ('refused' in picked) {
    return { text: picked.refused }
  }
  const { question, typed } = picked
  if (question.answer !== null) {
    return { text: `rex answer: question ${question.id} is answered already, got ${question.answer.option} (${question.answer.source}); a new question needs a new id` }
  }
  const option = optionOf(question, typed)
  if (option === undefined) {
    return { text: `rex answer: ${typed === '' ? 'nothing' : typed} is not an option of ${question.id}, got ${typed === '' ? 'nothing' : typed}; one of ${numbered(question)}` }
  }
  await answerQuestion($, question.id, { option, source: 'command' })
  await afterAnswer($, question, option)
  return lines([`question=${question.id}`, `answer=${option}`])
}

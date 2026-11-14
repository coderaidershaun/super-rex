// /rex policy and /rex answer: the person shows, confirms or changes the policy, and answers a parked question, each from their own typing only.
import type { CommandRunInput, CommandRunResult, EngineInterface } from 'claude-code'

import type { RexBook, RexQuestion } from '../../types'
import { answerQuestion, closeTree, fail, readBook, repinTree } from '../book/index'
import { originRefusal, rexLine } from '../commands/index'
import { ANSWER_SUBCOMMAND, POLICY_SUBCOMMAND } from './entries'
import { DEFAULT_POLICY, policyLines, policyWith, PolicyShapeError } from './policy'
import type { Policy } from './policy'
import { readPolicy, writePolicy } from './store'
import type { StoredPolicy } from './store'
import { answeredSpend } from './limits'
import { wakeLater } from './wake'

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
 * confirmed: pinned to its hash, with its live master and builders.
 */
async function resumeConfirmed($: EngineInterface, hash: string): Promise<readonly string[]> {
  const book = await readBook($)
  const resumed = book.trees.filter(tree => tree.session === book.session && tree.state === 'paused')
  for (const tree of resumed) {
    await repinTree($, tree.id, hash)
    await closeTree($, tree.id, 'running', null)
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

async function confirm($: EngineInterface): Promise<CommandRunResult> {
  const now = await policyNow($)
  const written = await writePolicy($, now.policy, 'confirm')
  const resumed = await resumeConfirmed($, written.hash)
  const waiting = treesOff(await readBook($), written.hash)
  return lines([...storedLines(written, 'stored'), `resumed=${resumed.length === 0 ? 'none' : resumed.join(',')}`, `still_paused=${waiting.length === 0 ? 'none' : waiting.join(',')}`])
}

export async function servePolicy($: EngineInterface, event: CommandRunInput): Promise<CommandRunResult> {
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
        return await confirm($)
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

function optionOf(question: RexQuestion, typed: string): string | undefined {
  return question.options.find(option => option === typed) ?? question.options.find(option => option.toLowerCase() === typed.toLowerCase())
}

export async function serveAnswer($: EngineInterface, event: CommandRunInput): Promise<CommandRunResult> {
  const refused = await originGate($, event, 'answer')
  if (refused !== null) {
    return refused
  }
  const args = rexLine(event).args
  const space = args.indexOf(' ')
  const id = space === -1 ? args : args.slice(0, space)
  const typed = space === -1 ? '' : args.slice(space + 1).trim()
  if (id === '' || typed === '') {
    return { text: `rex answer: name a question and one of its options, got ${args === '' ? 'nothing' : args}; run /rex answer <id> <option>` }
  }
  const book = await readBook($)
  const question = book.questions.find(each => each.id === id)
  if (question === undefined) {
    return { text: `rex answer: no question ${id} in this session, got ${id}; see the pane or /rex status` }
  }
  if (question.answer !== null) {
    return { text: `rex answer: question ${id} is answered already, got ${question.answer.option} (${question.answer.source}); a new question needs a new id` }
  }
  const option = optionOf(question, typed)
  if (option === undefined) {
    return { text: `rex answer: ${typed} is not an option of ${id}, got ${typed}; one of ${question.options.join(', ')}` }
  }
  await answerQuestion($, id, { option, source: 'command' })
  await answeredSpend($, question, option)
  const master = book.trees.find(tree => tree.id === question.tree && (tree.state === 'running' || tree.state === 'paused'))?.master ?? null
  if (master !== null) {
    wakeLater($, master, `rex: question ${id} is answered: ${option}. Read it with rex_status.`)
  }
  return lines([`question=${id}`, `answer=${option}`])
}

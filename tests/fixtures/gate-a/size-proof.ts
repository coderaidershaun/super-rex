// Proves that the copied token estimate of size-lib.ts makes the binary's own decisions: on each answer, shortening the rebuilt full page by the copy gives the binary's page byte for byte.
// Answers: the two latency forms, with and without the pool, and probe forms that this script builds from each vocabulary term. Usage: node tests/fixtures/gate-a/size-proof.ts [--pool <folder>]
import { join } from 'node:path'

import { BASE_COMMIT, DEFAULT_POOL, flag, HERE, isArray, isObject, type Json, jsonFiles, LIBRARY, libraryName, main, readFlags, readJson, say, ScriptError, text, VOCABULARY } from './lib.ts'
import { budget, fullPage, pageTokens, simulateFit } from './size-lib.ts'

type Case = { readonly id: string; readonly form: Json; readonly libraries: readonly string[] }

const LIMIT = 10
/** Words outside the vocabulary, so that lookup C searches the pool for them. They are probes, not the words of any task. */
const POOL_WORDS = ['kafka', 'terraform', 'postgres']

function termsOf(group: string): string[] {
  const vocabulary = readJson(VOCABULARY)
  const entry = isObject(vocabulary) ? vocabulary[group] : undefined
  if (!isObject(entry) || !isArray(entry['terms'])) throw new ScriptError(`${VOCABULARY} has no group ${group}`)
  return entry['terms'].map((term) => (isObject(term) ? (text(term['term']) ?? '') : ''))
}

/** Probe forms: each term alone, for a task that writes Rust and for one that writes no code, then all terms at once. */
function probes(pool: string): Case[] {
  const fields: readonly (readonly [string, string])[] = [
    ['activity', 'activities'],
    ['touches', 'touches'],
    ['must_have', 'qualities'],
    ['artefacts', 'artefacts'],
  ]
  const factSets: readonly (readonly [string, Json])[] = [
    ['rust', { writes_code: true, languages: ['rust'] }],
    ['nocode', { writes_code: false }],
  ]
  const cases: Case[] = []
  for (const [factName, facts] of factSets) {
    const all: Record<string, Json> = { id: `probe-${factName}-all`, facts, activity: 'build' }
    for (const [field, group] of fields) {
      for (const term of termsOf(group)) {
        const value: Json = field === 'activity' ? term : [term]
        cases.push({ id: `probe-${factName}-${field}-${term}`, form: { id: `probe-${factName}-${field}-${term}`, facts, [field]: value }, libraries: [LIBRARY] })
      }
      if (field !== 'activity') all[field] = termsOf(group)
    }
    cases.push({ id: `probe-${factName}-all`, form: all, libraries: [LIBRARY] })
  }
  for (const word of POOL_WORDS) {
    const id = `probe-rust-touches-${word}`
    cases.push({ id, form: { id, facts: { writes_code: true, languages: ['rust'] }, touches: [word] }, libraries: [LIBRARY, pool] })
  }
  return cases
}

function latencyCases(pool: string): Case[] {
  return jsonFiles(join(HERE, 'latency')).flatMap((file) => {
    const form = readJson(file)
    const id = isObject(form) ? (text(form['id']) ?? file) : file
    return [
      { id, form, libraries: [LIBRARY] },
      { id, form, libraries: [LIBRARY, pool] },
    ]
  })
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['pool'])
  const pool = flag(flags, 'pool', DEFAULT_POOL)
  say('h0', `sizes measured on base ${BASE_COMMIT}, before a parallel job adds open_when items to up to 12 library headers; budget ${budget(LIMIT)} tokens for a page of up to ${LIMIT} items`)
  const cases = [...latencyCases(pool), ...probes(pool)]
  let shortened = 0
  let kept = 0
  let claimHolds = 0
  let sameAsBinary = 0
  let inexact = 0
  for (const entry of cases) {
    const where = `${entry.id} [${entry.libraries.map(libraryName).join('+')}]`
    const page = fullPage(entry.libraries, entry.form, LIMIT)
    const fullTokens = pageTokens(page.full)
    const limitTokens = budget(page.full.items.length)
    const binaryText = JSON.stringify(page.answer.raw)
    const isShortened = JSON.stringify(page.full) !== binaryText
    const simulated = simulateFit(page.full, page.binding)
    const isSame = JSON.stringify(simulated.page) === binaryText
    const holds = isShortened ? fullTokens > limitTokens : fullTokens <= limitTokens
    if (isShortened) shortened += 1
    else kept += 1
    if (holds) claimHolds += 1
    if (isSame) sameAsBinary += 1
    if (page.inexact.length > 0) inexact += 1
    const shownTokens = pageTokens(page.answer.raw)
    say(
      'h1',
      `${where}: full page ${page.full.items.length} items, ${fullTokens} tokens, budget ${limitTokens}; binary ${isShortened ? 'shortened' : 'kept'} it (${page.answer.items.length} items, ${shownTokens} tokens); estimate ${holds ? 'agrees' : 'DISAGREES'}; copy of fit: ${simulated.steps} steps, ${simulated.popped} items moved on, ${isSame ? 'same page as binary' : 'DIFFERENT page from binary'}${page.inexact.length > 0 ? `; inexact: ${page.inexact.join('; ')}` : ''}`,
    )
  }
  say('h2', `${cases.length} answers: ${shortened} shortened by the binary, ${kept} kept whole; the full estimate agrees with the binary's decision on ${claimHolds} of ${cases.length}; the copied fit gives the binary's page byte for byte on ${sameAsBinary} of ${cases.length}; ${inexact} rebuilt pages marked inexact`)
  if (claimHolds !== cases.length || sameAsBinary !== cases.length) process.exitCode = 1
})

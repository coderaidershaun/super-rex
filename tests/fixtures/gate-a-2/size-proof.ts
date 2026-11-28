// Proves that the copied token estimate and fit of size-lib.ts make the binary's own decisions: on each answer, shortening the rebuilt full page by the copy gives the binary's page byte for byte.
// It also prints each answer's size and items, with and without keep_hashes, and counts how many answers reached each step. Usage: see README.md in this folder.
import { readFileSync } from 'node:fs'

import { BASE_COMMIT, DEFAULT_POOL, flag, formId, isArray, isObject, type Json, jsonFiles, LATENCY_FORMS, LIBRARY, libraryName, main, readAnswer, readFlags, readJson, say, ScriptError, search, shown, text, VOCABULARY } from './lib.ts'
import { budget, fullPage, OUTCOMES, pageTokens, requestFields, simulateFit, type SizeSettings, withPageConfirms } from './size-lib.ts'

type Case = { readonly group: string; readonly id: string; readonly form: Json; readonly libraries: readonly string[] }
type Setting = { readonly name: string; readonly isMeasured: boolean; readonly size: SizeSettings }

const LIMIT = 10
/** The measured settings, then two smaller budgets that only add answers to the proof, so that more steps are reached. */
const SETTINGS: readonly Setting[] = [
  { name: 'default budget', isMeasured: true, size: { limit: LIMIT, tokenBudget: null, keepHashes: false } },
  { name: 'default budget, keep_hashes', isMeasured: true, size: { limit: LIMIT, tokenBudget: null, keepHashes: true } },
  { name: 'budget 1200', isMeasured: false, size: { limit: LIMIT, tokenBudget: 1200, keepHashes: false } },
  { name: 'budget 1200, keep_hashes', isMeasured: false, size: { limit: LIMIT, tokenBudget: 1200, keepHashes: true } },
  { name: 'budget 600', isMeasured: false, size: { limit: LIMIT, tokenBudget: 600, keepHashes: false } },
  { name: 'budget 600, keep_hashes', isMeasured: false, size: { limit: LIMIT, tokenBudget: 600, keepHashes: true } },
]
/** Words outside the vocabulary, so that lookup C searches the pool for them. They are probes, not the words of any task. */
const POOL_WORDS = ['kafka', 'terraform', 'postgres']

function termsOf(group: string): string[] {
  const vocabulary = readJson(VOCABULARY)
  const entry = isObject(vocabulary) ? vocabulary[group] : undefined
  if (!isObject(entry) || !isArray(entry['terms'])) throw new ScriptError(`${VOCABULARY} has no group ${group}`)
  return entry['terms'].map((term) => (isObject(term) ? (text(term['term']) ?? '') : ''))
}

/** Probe forms: each term alone, for a task that writes Rust and for one that writes no code, then all terms at once. They are not real answers. */
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
        const id = `probe-${factName}-${field}-${term}`
        cases.push({ group: 'probes', id, form: { id, facts, [field]: field === 'activity' ? term : [term] }, libraries: [LIBRARY] })
      }
      if (field !== 'activity') all[field] = termsOf(group)
    }
    cases.push({ group: 'probes', id: `probe-${factName}-all`, form: all, libraries: [LIBRARY] })
  }
  for (const word of POOL_WORDS) {
    const id = `probe-rust-touches-${word}`
    cases.push({ group: 'probes', id, form: { id, facts: { writes_code: true, languages: ['rust'] }, touches: [word] }, libraries: [LIBRARY, pool] })
  }
  return cases
}

function realCases(sets: readonly string[], pool: string): Case[] {
  const latency = jsonFiles(LATENCY_FORMS).flatMap((file) => {
    const form = readJson(file)
    const id = formId(form, file)
    return [
      { group: 'latency', id, form, libraries: [LIBRARY] },
      { group: 'latency', id, form, libraries: [LIBRARY, pool] },
    ]
  })
  const forms = sets.flatMap((entry) => {
    const [name, folder] = entry.split('=')
    if (name === undefined || folder === undefined || name === '') throw new ScriptError(`--set ${entry} is not of the form <name>=<filled folder>`)
    return jsonFiles(folder).map((file) => {
      const form = JSON.parse(readFileSync(file, 'utf8')) as Json
      return { group: name, id: formId(form, file), form, libraries: [LIBRARY, pool] }
    })
  })
  return [...forms, ...latency]
}

function spread(values: readonly number[]): string {
  if (values.length === 0) return 'none'
  const sorted = [...values].sort((left, right) => left - right)
  const middle = Math.floor(sorted.length / 2)
  const median = sorted.length % 2 === 1 ? (sorted[middle] ?? 0) : ((sorted[middle - 1] ?? 0) + (sorted[middle] ?? 0)) / 2
  return `${sorted[0] ?? 0} / ${median} / ${sorted[sorted.length - 1] ?? 0}`
}

type Tally = { answers: number; same: number; estimateAgrees: number; inexact: number; reached: Map<string, number>; items: number[]; tokens: number[]; fullTokens: number[]; overBudget: string[] }

function newTally(): Tally {
  return { answers: 0, same: 0, estimateAgrees: 0, inexact: 0, reached: new Map(), items: [], tokens: [], fullTokens: [], overBudget: [] }
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['pool', 'set'])
  const pool = flag(flags, 'pool', DEFAULT_POOL)
  const cases = [...realCases(flags.values.get('set') ?? [], pool), ...probes(pool)]
  say('h0', `copy of the page module of base ${BASE_COMMIT}; limit ${LIMIT}; default budget ${budget({ limit: LIMIT, tokenBudget: null, keepHashes: false })} tokens; settings: ${SETTINGS.map((setting) => setting.name).join('; ')}; full pages rebuilt from one-item pages with no size limit`)
  const tallies = new Map<string, Tally>()
  const tally = (key: string): Tally => {
    const known = tallies.get(key) ?? newTally()
    tallies.set(key, known)
    return known
  }
  for (const entry of cases) {
    const where = `${entry.group} ${entry.id} [${entry.libraries.map(libraryName).join('+')}]`
    const rebuilt = fullPage(entry.libraries, entry.form, LIMIT)
    for (const setting of SETTINGS) {
      const line = search(entry.libraries, { ...requestFields(setting.size), requests: [entry.form] })
      if (JSON.stringify(JSON.parse(line)) !== line) throw new ScriptError(`${where}: the answer line does not equal its own compact JSON, so sizes of rebuilt pages would not match`)
      const answer = readAnswer(line).results[0]
      if (answer === undefined) throw new ScriptError(`${where}: the answer holds no result`)
      const page = withPageConfirms(rebuilt, answer)
      const fit = simulateFit(page.full, page.binding, setting.size)
      const binaryText = JSON.stringify(answer.raw)
      const isSame = JSON.stringify(fit.page) === binaryText
      const fullTokens = pageTokens(page.full)
      const isShortened = JSON.stringify(page.full) !== binaryText
      const agrees = isShortened === fullTokens > budget(setting.size)
      const outcomes = [...(fit.applied.length === 0 ? ['fits whole'] : fit.applied), ...(fit.popped > 0 ? ['drop items'] : []), ...(fit.overBudget ? ['over_budget'] : [])]
      const group = entry.group === 'probes' ? 'probes' : 'real answers'
      for (const key of [`${group} | ${setting.name}`, ...(setting.isMeasured && group === 'real answers' ? [`${entry.group} | ${setting.name}`] : [])]) {
        const counts = tally(key)
        counts.answers += 1
        if (isSame) counts.same += 1
        if (agrees) counts.estimateAgrees += 1
        if (page.inexact.length > 0) counts.inexact += 1
        outcomes.forEach((outcome) => counts.reached.set(outcome, (counts.reached.get(outcome) ?? 0) + 1))
        counts.items.push(answer.items.length)
        counts.tokens.push(pageTokens(answer.raw))
        counts.fullTokens.push(fullTokens)
        if (answer.overBudget) counts.overBudget.push(entry.id)
      }
      say(
        'h1',
        `${where} ${setting.name}: full page ${page.full.items.length} of ${rebuilt.total} items, ${fullTokens} tokens, budget ${budget(setting.size)}; binary: ${answer.items.length} items, ${pageTokens(answer.raw)} tokens, more ${answer.more}${answer.overBudget ? ', over_budget' : ''}; copy: ${outcomes.join(', ')}${fit.popped > 0 ? ` (${fit.popped} dropped)` : ''}; ${isSame ? 'same page as binary' : 'DIFFERENT page from binary'}; estimate ${agrees ? 'agrees' : 'DISAGREES'}${page.inexact.length > 0 ? `; inexact: ${page.inexact.join('; ')}` : ''}`,
      )
    }
  }
  let failed = 0
  for (const [key, counts] of [...tallies.entries()].sort(([left], [right]) => left.localeCompare(right))) {
    const steps = OUTCOMES.filter((outcome) => !(key.includes('keep_hashes') && outcome === 'leave out hashes'))
    const reached = steps.map((outcome) => `${outcome} ${counts.reached.get(outcome) ?? 0}`).join(', ')
    const unproven = steps.filter((outcome) => (counts.reached.get(outcome) ?? 0) === 0)
    if (counts.same !== counts.answers || counts.estimateAgrees !== counts.answers) failed += 1
    say('h2', `${key}: ${counts.answers} answers; copy gives the binary's page on ${counts.same}, estimate agrees on ${counts.estimateAgrees}, ${counts.inexact} rebuilt pages inexact; answers that reached each step: ${reached}; not proven here (no answer reached it): ${unproven.join(', ') || 'none'}`)
    say('h3', `${key}: items on the first page (least / median / most) ${spread(counts.items)}; tokens of the binary's page ${spread(counts.tokens)}; tokens of the full page ${spread(counts.fullTokens)}; over_budget ${counts.overBudget.length}${counts.overBudget.length > 0 ? ` (${counts.overBudget.join(', ')})` : ''}`)
  }
  say('h4', `${cases.length} cases at ${SETTINGS.length} settings; ${failed === 0 ? 'the copy makes the binary\'s decisions on every answer' : `the copy FAILS on ${failed} groups, so its token numbers are not run`}; results from ${shown(LIBRARY)} and ${shown(pool)}`)
  if (failed > 0) process.exitCode = 1
})

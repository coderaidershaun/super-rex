// Checks request forms against the search request of spec section 8.2 and the vocabulary in plugin/vocab/terms.json, before any search runs.
// Usage: node tests/fixtures/gate-a/validate-form.ts <form file or folder>... — exits 1 when a form breaks the format or names a word outside the vocabulary.
import { existsSync } from 'node:fs'

import { isArray, isObject, type Json, type JsonObject, jsonFiles, main, readJson, say, ScriptError, TASKS_FILE, text, VOCABULARY } from './lib.ts'

type Severity = 'format' | 'vocabulary' | 'evidence' | 'note'
type Finding = { readonly severity: Severity; readonly message: string }

type Fact = { readonly term: string; readonly type: string; readonly values: string; readonly allowed: readonly Json[]; readonly pattern: string | null; readonly suppliedBy: string }

type Vocabulary = {
  readonly facts: ReadonlyMap<string, Fact>
  readonly groups: ReadonlyMap<string, ReadonlySet<string>>
}

const FORM_KEYS = ['id', 'facts', 'activity', 'touches', 'must_have', 'artefacts', 'needs', 'evidence']
const REQUEST_KEYS = ['requests', 'limit', 'cursor', 'kinds', 'categories']
/** The request field of each vocabulary group whose terms a form ticks. */
const TERM_FIELDS: ReadonlyMap<string, string> = new Map([
  ['activity', 'activities'],
  ['touches', 'touches'],
  ['must_have', 'qualities'],
  ['artefacts', 'artefacts'],
  ['kinds', 'kinds'],
  ['categories', 'categories'],
])
const MAX_NEEDS = 3

function readVocabulary(): Vocabulary {
  const terms = readJson(VOCABULARY)
  if (!isObject(terms)) throw new ScriptError(`${VOCABULARY} is not an object`)
  const groups = new Map<string, Set<string>>()
  for (const group of new Set(TERM_FIELDS.values())) {
    const entry = terms[group]
    if (!isObject(entry) || !isArray(entry['terms'])) throw new ScriptError(`${VOCABULARY} has no group ${group}`)
    groups.set(group, new Set(entry['terms'].map((term) => (isObject(term) ? (text(term['term']) ?? '') : ''))))
  }
  const facts = new Map<string, Fact>()
  const factGroup = terms['facts']
  if (!isObject(factGroup) || !isArray(factGroup['terms'])) throw new ScriptError(`${VOCABULARY} has no group facts`)
  for (const fact of factGroup['terms']) {
    if (!isObject(fact)) continue
    const term = text(fact['term']) ?? ''
    const allowed = fact['allowed']
    facts.set(term, {
      term,
      type: text(fact['type']) ?? '',
      values: text(fact['values']) ?? '',
      allowed: isArray(allowed) ? allowed : [],
      pattern: text(fact['pattern']) ?? null,
      suppliedBy: text(fact['supplied_by']) ?? '',
    })
  }
  return { facts, groups }
}

/** The task text of each item of tasks.json, by its ID, in lower case with runs of white space made one space. */
function readTaskTexts(): ReadonlyMap<string, string> {
  const texts = new Map<string, string>()
  if (!existsSync(TASKS_FILE)) return texts
  const tasks = readJson(TASKS_FILE)
  if (!isObject(tasks) || !isArray(tasks['items'])) return texts
  for (const item of tasks['items']) {
    if (!isObject(item)) continue
    const words = ['outcome', 'evidence', 'owns'].map((field) => text(item[field]) ?? '').join(' ')
    texts.set(text(item['id']) ?? '', normal(words))
  }
  return texts
}

function normal(words: string): string {
  return words.toLowerCase().replace(/\s+/g, ' ').trim()
}

function checkFactValue(fact: Fact, value: Json, out: Finding[]): void {
  const name = `facts.${fact.term}`
  const closed = fact.values === 'closed'
  const isAllowed = (entry: Json): boolean => fact.allowed.includes(entry)
  if (fact.type === 'boolean') {
    if (typeof value !== 'boolean') out.push({ severity: 'format', message: `${name} must be true or false` })
    return
  }
  if (fact.type === 'list') {
    if (!isArray(value) || !value.every((entry) => typeof entry === 'string')) {
      out.push({ severity: 'format', message: `${name} must be a list of strings` })
      return
    }
    if (closed) value.filter((entry) => !isAllowed(entry)).forEach((entry) => out.push({ severity: 'vocabulary', message: `${name} holds ${JSON.stringify(entry)}, which the vocabulary does not allow` }))
    if (new Set(value).size !== value.length) out.push({ severity: 'format', message: `${name} repeats a value` })
    return
  }
  if (typeof value !== 'string') {
    out.push({ severity: 'format', message: `${name} must be a string` })
    return
  }
  if (closed && !isAllowed(value)) out.push({ severity: 'vocabulary', message: `${name} is ${JSON.stringify(value)}, which the vocabulary does not allow` })
  if (fact.pattern !== null && !new RegExp(fact.pattern).test(value)) out.push({ severity: 'vocabulary', message: `${name} ${JSON.stringify(value)} does not match ${fact.pattern}` })
}

function checkTerms(form: JsonObject, field: string, vocabulary: Vocabulary, out: Finding[]): string[] {
  const value = form[field]
  if (value === undefined || value === null) return []
  const isSingle = field === 'activity'
  const words = isSingle ? [value] : isArray(value) ? value : null
  if (words === null || !words.every((word) => typeof word === 'string')) {
    out.push({ severity: 'format', message: isSingle ? `${field} must be one term` : `${field} must be a list of terms` })
    return []
  }
  const known = vocabulary.groups.get(TERM_FIELDS.get(field) ?? '') ?? new Set<string>()
  const terms = words.filter((word): word is string => typeof word === 'string')
  terms.filter((term) => !known.has(term)).forEach((term) => out.push({ severity: 'vocabulary', message: `${field} holds ${JSON.stringify(term)}, which is not a term of the vocabulary; search lists it as unrecognised` }))
  if (new Set(terms).size !== terms.length) out.push({ severity: 'format', message: `${field} repeats a term` })
  return terms
}

/** Every term of the four term fields needs a quote. A fact may carry one, by its name or value, but need not: the agent states facts as it does for assignment today. */
function checkEvidence(form: JsonObject, ticked: readonly string[], required: readonly string[], taskText: string | undefined, out: Finding[]): void {
  const evidence = form['evidence'] ?? []
  if (!isArray(evidence)) {
    out.push({ severity: 'format', message: 'evidence must be a list of { term, quote }' })
    return
  }
  const quoted = new Set<string>()
  evidence.forEach((entry, at) => {
    const where = `evidence ${at + 1}`
    if (!isObject(entry) || Object.keys(entry).some((key) => key !== 'term' && key !== 'quote')) {
      out.push({ severity: 'format', message: `${where} must hold exactly term and quote` })
      return
    }
    const term = text(entry['term'])
    const quote = text(entry['quote'])
    if (term === undefined || quote === undefined || quote.trim() === '') {
      out.push({ severity: 'format', message: `${where} needs a term and a quote that is not empty` })
      return
    }
    quoted.add(term)
    if (!ticked.includes(term)) out.push({ severity: 'evidence', message: `${where} names ${JSON.stringify(term)}, which the form does not tick` })
    if (taskText !== undefined && !taskText.includes(normal(quote))) out.push({ severity: 'evidence', message: `${where}: the quote for ${JSON.stringify(term)} is not in the task's own text` })
  })
  required.filter((term) => !quoted.has(term)).forEach((term) => out.push({ severity: 'evidence', message: `${JSON.stringify(term)} is ticked with no evidence quote` }))
}

function checkForm(form: Json, vocabulary: Vocabulary, tasks: ReadonlyMap<string, string>): Finding[] {
  const out: Finding[] = []
  if (!isObject(form)) return [{ severity: 'format', message: 'a form must be a JSON object' }]
  Object.keys(form)
    .filter((key) => !FORM_KEYS.includes(key))
    .forEach((key) => out.push({ severity: 'format', message: `unknown field ${key}; search refuses the request` }))
  const id = text(form['id'])
  if (id === undefined || id === '') out.push({ severity: 'format', message: 'id must be a string that is not empty' })
  else if (tasks.size > 0 && !tasks.has(id)) out.push({ severity: 'note', message: `id ${id} is not an item of tasks.json` })
  const ticked: string[] = []
  const facts = form['facts'] ?? {}
  if (!isObject(facts)) out.push({ severity: 'format', message: 'facts must be an object' })
  else {
    for (const [name, value] of Object.entries(facts)) {
      const fact = vocabulary.facts.get(name)
      if (fact === undefined) {
        out.push({ severity: 'vocabulary', message: `facts.${name} is not a fact of the vocabulary; search lists it as unrecognised` })
        continue
      }
      if (fact.suppliedBy === 'mod') out.push({ severity: 'note', message: `facts.${name} is filled by the mod from the repo; the writer's value is replaced` })
      checkFactValue(fact, value, out)
      ticked.push(name)
      if (isArray(value)) ticked.push(...value.filter((entry): entry is string => typeof entry === 'string'))
      else if (typeof value === 'string') ticked.push(value)
    }
  }
  const terms = ['activity', 'touches', 'must_have', 'artefacts'].flatMap((field) => checkTerms(form, field, vocabulary, out))
  ticked.push(...terms)
  const needs = form['needs'] ?? []
  if (!isArray(needs) || !needs.every((need) => typeof need === 'string' && need.trim() !== '')) out.push({ severity: 'format', message: 'needs must be a list of text that is not empty' })
  else if (needs.length > MAX_NEEDS) out.push({ severity: 'format', message: `needs holds ${needs.length} items; at most ${MAX_NEEDS} are allowed` })
  checkEvidence(form, ticked, terms, id === undefined ? undefined : tasks.get(id), out)
  if (terms.length === 0 && (!isArray(needs) || needs.length === 0) && Object.keys(isObject(facts) ? facts : {}).length === 0) {
    out.push({ severity: 'note', message: 'the form asks for nothing: no facts, terms or needs' })
  }
  return out
}

/** A file holds one form, or a whole search request whose `requests` are checked one by one. */
function formsOf(file: string, vocabulary: Vocabulary, top: Finding[]): Json[] {
  const value = readJson(file)
  if (!isObject(value) || !('requests' in value)) return [value]
  Object.keys(value)
    .filter((key) => !REQUEST_KEYS.includes(key))
    .forEach((key) => top.push({ severity: 'format', message: `unknown request field ${key}` }))
  const limit = value['limit']
  if (limit !== undefined && limit !== null && !(typeof limit === 'number' && Number.isInteger(limit) && limit >= 1)) top.push({ severity: 'format', message: 'limit must be a whole number of 1 or more' })
  checkTerms(value, 'kinds', vocabulary, top)
  checkTerms(value, 'categories', vocabulary, top)
  const requests = value['requests']
  return isArray(requests) ? [...requests] : []
}

function filesOf(path: string): string[] {
  if (!existsSync(path)) throw new ScriptError(`${path} does not exist`)
  return path.endsWith('.json') ? [path] : jsonFiles(path)
}

main(() => {
  const paths = process.argv.slice(2)
  if (paths.length === 0) throw new ScriptError('give at least one form file or folder of form files')
  const vocabulary = readVocabulary()
  const tasks = readTaskTexts()
  say('v0', `vocabulary ${VOCABULARY}: ${vocabulary.facts.size} facts, ${[...vocabulary.groups.values()].reduce((sum, group) => sum + group.size, 0)} terms; ${tasks.size} task texts`)
  const counts = new Map<Severity, number>([['format', 0], ['vocabulary', 0], ['evidence', 0], ['note', 0]])
  let forms = 0
  let clean = 0
  for (const file of paths.flatMap(filesOf)) {
    const top: Finding[] = []
    const inFile = formsOf(file, vocabulary, top)
    const all = [top, ...inFile.map((form) => checkForm(form, vocabulary, tasks))]
    all.forEach((findings, at) => {
      if (at > 0) forms += 1
      if (at > 0 && findings.every((finding) => finding.severity === 'note')) clean += 1
      for (const finding of findings) {
        counts.set(finding.severity, (counts.get(finding.severity) ?? 0) + 1)
        say('v1', `${file}${at > 0 ? ` form ${at}` : ''}: ${finding.severity}: ${finding.message}`)
      }
    })
  }
  const summary = [...counts.entries()].map(([severity, count]) => `${severity} ${count}`).join(', ')
  say('v2', `${forms} forms checked, ${clean} with no finding beyond notes; findings: ${summary}`)
  if ((counts.get('format') ?? 0) + (counts.get('vocabulary') ?? 0) > 0) process.exitCode = 1
})

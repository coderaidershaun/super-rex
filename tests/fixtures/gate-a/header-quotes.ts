// Writes header-quotes.json: each item of tasks.json whose own text quotes an intent phrase, claim term, kind or category of a library skill header.
// Such items are easy for search by construction, so the hit-rate scorer also reports without them. Usage: node tests/fixtures/gate-a/header-quotes.ts
import { readFileSync } from 'node:fs'

import {
  HEADER_QUOTES_FILE,
  isArray,
  isObject,
  type Json,
  type JsonObject,
  LIBRARY,
  main,
  readJson,
  REX,
  run,
  say,
  ScriptError,
  sha256,
  TASKS_FILE,
  text,
  textList,
  writeJson,
} from './lib.ts'

type Source = 'intent' | 'claim' | 'kind' | 'category'
type Phrase = { readonly words: string; readonly source: Source; readonly skills: string[] }
type Quote = { readonly words: string; readonly source: Source; readonly skills: readonly string[]; readonly fields: readonly string[] }

const TEXT_FIELDS = ['outcome', 'evidence', 'owns'] as const

function normal(words: string): string {
  return words.toLowerCase().replace(/\s+/g, ' ').trim()
}

/** Every phrase that a library header declares, with the skills that declare it, read from the binary's static index. */
function headerPhrases(): Phrase[] {
  const result = run(REX, ['skills', 'index', '--library', LIBRARY])
  if (result.status !== 0) throw new ScriptError(`rex skills index exited ${result.status}: ${result.stderr.trim()}`)
  const index = JSON.parse(result.stdout) as Json
  if (!isObject(index) || !isArray(index['skills'])) throw new ScriptError('the static index has no `skills` list')
  const phrases = new Map<string, Phrase>()
  const add = (words: string, source: Source, skill: string): void => {
    const key = `${source}:${normal(words)}`
    const phrase = phrases.get(key) ?? { words: normal(words), source, skills: [] }
    if (!phrase.skills.includes(skill)) phrase.skills.push(skill)
    phrases.set(key, phrase)
  }
  for (const skill of index['skills']) {
    if (!isObject(skill) || !isObject(skill['metadata'])) continue
    const name = text(skill['name']) ?? ''
    const metadata: JsonObject = skill['metadata']
    textList(metadata['intents']).forEach((words) => add(words, 'intent', name))
    const kind = text(metadata['kind'])
    if (kind !== undefined) add(kind, 'kind', name)
    const category = text(metadata['category'])
    if (category !== undefined) add(category, 'category', name)
    const claims = metadata['claims']
    if (!isObject(claims)) continue
    for (const field of ['quality', 'artefact', 'activity', 'touches']) textList(claims[field]).forEach((term) => add(term, 'claim', name))
    const facts = claims['facts']
    if (!isObject(facts)) continue
    const role = text(facts['role'])
    if (role !== undefined) add(role, 'claim', name)
    textList(facts['languages']).forEach((language) => add(language, 'claim', name))
  }
  return [...phrases.values()].sort((left, right) => `${left.source}:${left.words}`.localeCompare(`${right.source}:${right.words}`))
}

function escape(words: string): string {
  return words.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/** A phrase counts as a quote when the item's text holds it whole: no letter, digit or hyphen touches either end. */
function fieldsQuoting(item: JsonObject, phrase: Phrase): string[] {
  const pattern = new RegExp(`(?<![a-z0-9-])${escape(phrase.words)}(?![a-z0-9-])`)
  return TEXT_FIELDS.filter((field) => pattern.test(normal(text(item[field]) ?? '')))
}

/** An intent phrase, or a term of two or more words, is a quote. A one-word term such as `build` or `rule` is an everyday word, kept apart. */
function isStrong(phrase: Phrase): boolean {
  return phrase.source === 'intent' || /[- ]/.test(phrase.words)
}

main(() => {
  const tasks = readJson(TASKS_FILE)
  if (!isObject(tasks) || !isArray(tasks['items'])) throw new ScriptError(`${TASKS_FILE} has no \`items\` list`)
  say('c3', `tasks.json sha256 ${sha256(readFileSync(TASKS_FILE, 'utf8'))}`)
  const phrases = headerPhrases()
  const strongCount = phrases.filter(isStrong).length
  say('c3', `${phrases.length} header phrases of the library: ${strongCount} intent phrases or terms of two or more words, ${phrases.length - strongCount} one-word terms`)
  const quoting: Json[] = []
  const singleOnly: Json[] = []
  for (const item of tasks['items']) {
    if (!isObject(item)) continue
    const found: Quote[] = phrases
      .map((phrase) => ({ words: phrase.words, source: phrase.source, skills: phrase.skills, fields: fieldsQuoting(item, phrase) }))
      .filter((quote) => quote.fields.length > 0)
    const strong = found.filter((quote) => isStrong({ words: quote.words, source: quote.source, skills: [] }))
    const single = found.filter((quote) => !strong.includes(quote))
    const entry = { id: text(item['id']) ?? '', quotes: strong, single_word: single }
    if (strong.length > 0) quoting.push(entry)
    else if (single.length > 0) singleOnly.push(entry)
  }
  writeJson(HEADER_QUOTES_FILE, {
    about:
      'Items whose Outcome, Evidence or Owns text holds, whole and in any case, an intent phrase of a library skill or a claim term, kind or category of two or more words. One-word terms are listed under single_word and do not put an item in `items`.',
    items: quoting,
    single_word_only: singleOnly,
  })
  say('c3', `wrote ${HEADER_QUOTES_FILE}: ${quoting.length} of ${tasks['items'].length} items quote a header; ${singleOnly.length} more hold only one-word terms; sha256 ${sha256(readFileSync(HEADER_QUOTES_FILE, 'utf8'))}`)
})

// Compares request forms field by field: each run against each other run, and each run against a reference, with counts of agreement per field.
// Usage: node tests/fixtures/gate-a/compare-forms.ts --run <folder> --run <folder> [--reference <folder>] — the repo facts are left out, because the mod fills them.
import { isArray, isObject, type Json, type JsonObject, jsonFiles, main, readFlags, readJson, say, ScriptError, text } from './lib.ts'

type Field = { readonly name: string; readonly isSet: boolean; readonly read: (form: JsonObject) => readonly string[] }

function strings(value: Json | undefined): string[] {
  if (value === undefined || value === null) return []
  if (isArray(value)) return value.map((entry) => (typeof entry === 'string' ? entry : JSON.stringify(entry)))
  return [typeof value === 'string' ? value : JSON.stringify(value)]
}

function fact(name: string): (form: JsonObject) => string[] {
  return (form) => (isObject(form['facts']) ? strings(form['facts'][name]) : [])
}

const FIELDS: readonly Field[] = [
  { name: 'facts.writes_code', isSet: false, read: fact('writes_code') },
  { name: 'facts.languages', isSet: true, read: fact('languages') },
  { name: 'facts.role', isSet: false, read: fact('role') },
  { name: 'facts.latency_scale', isSet: false, read: fact('latency_scale') },
  { name: 'activity', isSet: false, read: (form) => strings(form['activity']) },
  { name: 'touches', isSet: true, read: (form) => strings(form['touches']) },
  { name: 'must_have', isSet: true, read: (form) => strings(form['must_have']) },
  { name: 'artefacts', isSet: true, read: (form) => strings(form['artefacts']) },
  { name: 'needs.count', isSet: false, read: (form) => [String(strings(form['needs']).length)] },
  { name: 'needs.text', isSet: true, read: (form) => strings(form['needs']).map((need) => need.toLowerCase().trim()) },
  { name: 'evidence.terms', isSet: true, read: (form) => (isArray(form['evidence']) ? form['evidence'].map((entry) => (isObject(entry) ? (text(entry['term']) ?? '') : '')) : []) },
]

function readForms(folder: string): Map<string, JsonObject> {
  const forms = new Map<string, JsonObject>()
  for (const file of jsonFiles(folder)) {
    const form = readJson(file)
    if (!isObject(form)) throw new ScriptError(`${file} is not a form object`)
    forms.set(text(form['id']) ?? file, form)
  }
  return forms
}

function key(values: readonly string[], isSet: boolean): string {
  return JSON.stringify(isSet ? [...new Set(values)].sort() : values)
}

function compare(number: string, leftName: string, left: ReadonlyMap<string, JsonObject>, rightName: string, right: ReadonlyMap<string, JsonObject>): void {
  const shared = [...left.keys()].filter((id) => right.has(id)).sort()
  const onlyLeft = [...left.keys()].filter((id) => !right.has(id)).length
  const onlyRight = [...right.keys()].filter((id) => !left.has(id)).length
  say(number, `${leftName} against ${rightName}: ${shared.length} forms in both, ${onlyLeft} only in ${leftName}, ${onlyRight} only in ${rightName}`)
  let wholeSame = 0
  const differences: string[] = []
  const sameCount = new Map<string, number>()
  const termCount = new Map<string, { both: number; leftOnly: number; rightOnly: number }>()
  for (const id of shared) {
    const leftForm = left.get(id)
    const rightForm = right.get(id)
    if (leftForm === undefined || rightForm === undefined) continue
    let isSame = true
    for (const field of FIELDS) {
      const leftValues = field.read(leftForm)
      const rightValues = field.read(rightForm)
      const equal = key(leftValues, field.isSet) === key(rightValues, field.isSet)
      if (equal) sameCount.set(field.name, (sameCount.get(field.name) ?? 0) + 1)
      else if (field.name !== 'needs.text' && field.name !== 'evidence.terms') {
        isSame = false
        differences.push(`${id} ${field.name}: ${leftName} ${key(leftValues, field.isSet)} ${rightName} ${key(rightValues, field.isSet)}`)
      }
      if (field.isSet) {
        const counts = termCount.get(field.name) ?? { both: 0, leftOnly: 0, rightOnly: 0 }
        const leftSet = new Set(leftValues)
        const rightSet = new Set(rightValues)
        counts.both += [...leftSet].filter((value) => rightSet.has(value)).length
        counts.leftOnly += [...leftSet].filter((value) => !rightSet.has(value)).length
        counts.rightOnly += [...rightSet].filter((value) => !leftSet.has(value)).length
        termCount.set(field.name, counts)
      }
    }
    if (isSame) wholeSame += 1
  }
  for (const field of FIELDS) {
    const terms = termCount.get(field.name)
    const termLine = terms === undefined ? '' : `; values in both ${terms.both}, only ${leftName} ${terms.leftOnly}, only ${rightName} ${terms.rightOnly}`
    say(number, `${field.name}: equal on ${sameCount.get(field.name) ?? 0} of ${shared.length}${termLine}`)
  }
  differences.forEach((line) => say(number, `differs: ${line}`))
  say(number, `${wholeSame} of ${shared.length} forms equal on every field but needs.text and evidence.terms; ${differences.length} field differences`)
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['run', 'reference'])
  const runs = flags.values.get('run') ?? []
  const reference = flags.values.get('reference')?.[0]
  if (runs.length === 0) throw new ScriptError('give at least one --run folder')
  const forms = runs.map((folder) => ({ folder, forms: readForms(folder) }))
  say('m0', `${runs.length} runs${reference === undefined ? '' : ' and a reference'}; fields: ${FIELDS.map((field) => field.name).join(', ')}`)
  let pair = 0
  for (let first = 0; first < forms.length; first += 1) {
    for (let second = first + 1; second < forms.length; second += 1) {
      const left = forms[first]
      const right = forms[second]
      if (left === undefined || right === undefined) continue
      pair += 1
      compare(`m1.${pair}`, `run${first + 1}`, left.forms, `run${second + 1}`, right.forms)
    }
  }
  if (reference !== undefined) {
    const referenceForms = readForms(reference)
    forms.forEach((entry, at) => compare(`m2.${at + 1}`, `run${at + 1}`, entry.forms, 'reference', referenceForms))
  }
})

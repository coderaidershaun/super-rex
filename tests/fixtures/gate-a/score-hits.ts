// Scores search results against a labels file: the top-five hit rate of the right skills, by expected lookup and by the tier that found them, and each label with no hit.
// It scores all items, then without the items of header-quotes.json; with --judgments it counts the judge's verdicts on tier A and B items. Usage: see README.md.
import { existsSync } from 'node:fs'

import { flag, HEADER_QUOTES_FILE, isArray, isObject, type Json, jsonFiles, LIBRARY, libraryName, main, readFlags, readJson, requiredFlag, say, ScriptError, text } from './lib.ts'

type Found = { readonly rank: number; readonly skill: string; readonly library: string; readonly tier: string }
type Target = { readonly skill: string; readonly library: string }
type Label = { readonly id: string; readonly kind: 'skill' | 'need'; readonly name: string; readonly expected: string; readonly targets: readonly Target[] }
type Score = { readonly label: Label; readonly hit: Found | undefined; readonly seen: Found | undefined; readonly hasResult: boolean }

const LOOKUPS = ['A', 'B', 'C']
const HOUSE = libraryName(LIBRARY)

function readLabels(path: string): Label[] {
  const labels = readJson(path)
  if (!isObject(labels) || !isArray(labels['items'])) throw new ScriptError(`${path} has no \`items\` list`)
  return labels['items'].flatMap((item, at): Label[] => {
    const where = `${path} item ${at + 1}`
    if (!isObject(item)) throw new ScriptError(`${where} is not an object`)
    const id = text(item['id'])
    if (id === undefined) throw new ScriptError(`${where} has no id`)
    const skills = (isArray(item['skills']) ? item['skills'] : []).map((entry, index): Label => {
      const skill = isObject(entry) ? text(entry['skill']) : undefined
      const expected = isObject(entry) ? text(entry['lookup']) : undefined
      if (skill === undefined || expected === undefined || !LOOKUPS.includes(expected)) throw new ScriptError(`${where} skill ${index + 1} needs a skill name and a lookup of A, B or C`)
      const library = (isObject(entry) ? text(entry['library']) : undefined) ?? HOUSE
      return { id, kind: 'skill', name: skill, expected, targets: [{ skill, library }] }
    })
    const needs = (isArray(item['needs']) ? item['needs'] : []).map((entry, index): Label => {
      const need = isObject(entry) ? text(entry['need']) : undefined
      const targets = isObject(entry) && isArray(entry['skills']) ? entry['skills'] : null
      if (need === undefined || targets === null) throw new ScriptError(`${where} need ${index + 1} needs a need text and a skills list`)
      const parsed = targets.map((target): Target => {
        const skill = isObject(target) ? text(target['skill']) : undefined
        if (skill === undefined) throw new ScriptError(`${where} need ${index + 1} has a skill with no name`)
        return { skill, library: (isObject(target) ? text(target['library']) : undefined) ?? HOUSE }
      })
      return { id, kind: 'need', name: need, expected: 'C', targets: parsed }
    })
    return [...skills, ...needs]
  })
}

function readResults(folder: string): Map<string, Found[]> {
  const results = new Map<string, Found[]>()
  for (const file of jsonFiles(folder)) {
    const result = readJson(file)
    if (!isObject(result) || !isArray(result['items'])) throw new ScriptError(`${file} is not a result of run-search.ts`)
    const later = isArray(result['later_items']) ? result['later_items'] : []
    const items = [...result['items'], ...later].filter(isObject).map((item) => ({
      rank: typeof item['rank'] === 'number' ? item['rank'] : 0,
      skill: text(item['skill']) ?? '',
      library: text(item['library']) ?? '',
      tier: text(item['tier']) ?? '',
    }))
    results.set(text(result['id']) ?? file, items)
  }
  return results
}

function quotingIds(): Set<string> {
  if (!existsSync(HEADER_QUOTES_FILE)) throw new ScriptError(`${HEADER_QUOTES_FILE} does not exist`)
  const quotes = readJson(HEADER_QUOTES_FILE)
  const items: readonly Json[] = isObject(quotes) && isArray(quotes['items']) ? quotes['items'] : []
  return new Set(items.map((item) => (isObject(item) ? (text(item['id']) ?? '') : '')))
}

function score(labels: readonly Label[], results: ReadonlyMap<string, readonly Found[]>, top: number): Score[] {
  return labels.map((label) => {
    const items = results.get(label.id)
    const matches = (items ?? []).filter((item) => label.targets.some((target) => target.skill === item.skill && target.library === item.library))
    const seen = matches[0]
    return { label, hit: seen !== undefined && seen.rank <= top ? seen : undefined, seen, hasResult: items !== undefined }
  })
}

function rate(hits: number, total: number): string {
  return total === 0 ? `${hits} of 0` : `${hits} of ${total} (${((100 * hits) / total).toFixed(1)}%)`
}

function report(part: string, scores: readonly Score[], results: ReadonlyMap<string, readonly Found[]>, labels: readonly Label[], top: number): void {
  const checkable = scores.filter((entry) => entry.label.targets.length > 0)
  const hits = checkable.filter((entry) => entry.hit !== undefined)
  const ids = new Set(scores.map((entry) => entry.label.id))
  say(`${part}.1`, `${ids.size} items, ${checkable.length} labels with a right skill (${scores.length - checkable.length} needs labelled with no right skill), ${scores.filter((entry) => !entry.hasResult).length} labels with no result file`)
  say(`${part}.2`, `top-${top} hit rate, all labels: ${rate(hits.length, checkable.length)}`)
  for (const kind of ['skill', 'need'] as const) {
    const ofKind = checkable.filter((entry) => entry.label.kind === kind)
    say(`${part}.2`, `top-${top} hit rate, ${kind === 'skill' ? 'library skill labels' : 'free-text need labels'}: ${rate(ofKind.filter((entry) => entry.hit !== undefined).length, ofKind.length)}`)
  }
  for (const lookup of LOOKUPS) {
    const expected = checkable.filter((entry) => entry.label.expected === lookup)
    say(`${part}.3`, `expected lookup ${lookup}: ${rate(expected.filter((entry) => entry.hit !== undefined).length, expected.length)}`)
  }
  for (const tier of LOOKUPS) {
    const byTier = hits.filter((entry) => entry.hit?.tier === tier)
    const asExpected = byTier.filter((entry) => entry.label.expected === tier).length
    say(`${part}.4`, `found by tier ${tier}: ${rate(byTier.length, checkable.length)} of all labels; ${asExpected} of them expected at ${tier}, ${byTier.length - asExpected} expected at another lookup`)
  }
  for (const lookup of LOOKUPS) {
    const row = checkable.filter((entry) => entry.label.expected === lookup)
    const cells = [...LOOKUPS.map((tier) => `${tier} ${row.filter((entry) => entry.hit?.tier === tier).length}`), `below top ${row.filter((entry) => entry.hit === undefined && entry.seen !== undefined).length}`, `absent ${row.filter((entry) => entry.seen === undefined).length}`]
    say(`${part}.5`, `expected ${lookup} -> found ${cells.join(', ')}`)
  }
  const misses = checkable.filter((entry) => entry.hit === undefined)
  for (const miss of misses) {
    const where = miss.seen === undefined ? (miss.hasResult ? 'not on the first page' : 'no result file') : `rank ${miss.seen.rank}, tier ${miss.seen.tier}`
    say(`${part}.6`, `no hit: ${miss.label.id} ${miss.label.kind} ${JSON.stringify(miss.label.name)} (expected ${miss.label.expected}): ${where}`)
  }
  say(`${part}.6`, `${misses.length} labels with no hit in the top ${top}`)
  const labelled = new Set(labels.filter((label) => ids.has(label.id)).flatMap((label) => label.targets.map((target) => `${label.id} ${target.library}/${target.skill}`)))
  let unlabelled = 0
  for (const id of [...ids].sort()) {
    for (const item of results.get(id) ?? []) {
      if ((item.tier === 'A' || item.tier === 'B') && !labelled.has(`${id} ${item.library}/${item.skill}`)) {
        unlabelled += 1
        say(`${part}.7`, `tier ${item.tier} item not in the labels (for the judge): ${id} ${item.library}/${item.skill} rank ${item.rank}`)
      }
    }
  }
  say(`${part}.7`, `${unlabelled} tier A or B items that no label names`)
}

const VERDICTS = ['right', 'wrong', 'doubtful']

/** Counts the judge's verdict on each tier A or B item of the results, and lists each one judged wrong and each one not judged. */
function reportJudgments(path: string, results: ReadonlyMap<string, readonly Found[]>): void {
  const judgments = readJson(path)
  if (!isObject(judgments) || !isArray(judgments['items'])) throw new ScriptError(`${path} has no \`items\` list`)
  const verdicts = new Map<string, string>()
  for (const item of judgments['items']) {
    if (!isObject(item) || !isArray(item['assignments'])) throw new ScriptError(`${path}: each item needs an id and an assignments list`)
    for (const entry of item['assignments']) {
      const verdict = isObject(entry) ? text(entry['verdict']) : undefined
      if (!isObject(entry) || verdict === undefined || !VERDICTS.includes(verdict)) throw new ScriptError(`${path}: each assignment needs a verdict of ${VERDICTS.join(', ')}`)
      verdicts.set(`${text(item['id']) ?? ''} ${text(entry['library']) ?? HOUSE}/${text(entry['skill']) ?? ''}`, verdict)
    }
  }
  const counts = new Map<string, number>()
  for (const [id, items] of [...results.entries()].sort()) {
    for (const item of items.filter((found) => found.tier === 'A' || found.tier === 'B')) {
      const verdict = verdicts.get(`${id} ${item.library}/${item.skill}`) ?? 'not judged'
      counts.set(`${item.tier} ${verdict}`, (counts.get(`${item.tier} ${verdict}`) ?? 0) + 1)
      if (verdict !== 'right') say('k3.1', `tier ${item.tier} ${verdict}: ${id} ${item.library}/${item.skill} rank ${item.rank}`)
    }
  }
  for (const tier of ['A', 'B']) say('k3.2', `tier ${tier}: ${[...VERDICTS, 'not judged'].map((verdict) => `${verdict} ${counts.get(`${tier} ${verdict}`) ?? 0}`).join(', ')}`)
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['labels', 'results', 'top', 'judgments'])
  const labelsPath = requiredFlag(flags, 'labels')
  const top = Number(flag(flags, 'top', '5'))
  if (!Number.isInteger(top) || top < 1) throw new ScriptError('--top must be a whole number of 1 or more')
  const labels = readLabels(labelsPath)
  const results = readResults(requiredFlag(flags, 'results'))
  say('k0', `${labels.length} labels from ${labelsPath}; ${results.size} result files; top ${top}`)
  report('k1', score(labels, results, top), results, labels, top)
  const quoting = quotingIds()
  const kept = labels.filter((label) => !quoting.has(label.id))
  say('k2', `without the ${quoting.size} items of header-quotes.json: ${kept.length} labels left`)
  report('k2', score(kept, results, top), results, kept, top)
  const judgments = flags.values.get('judgments')?.[0]
  if (judgments !== undefined) reportJudgments(judgments, results)
})

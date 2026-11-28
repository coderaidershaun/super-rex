// Lists every tier A or B pair of item and skill in result folders, joins the judge's verdicts on item, skill and library, and lists each pair with no verdict.
// It also tells, for each tier B pair that the first measurement judged wrong or doubtful, whether it is still returned. Usage: see README.md in this folder.
import { basename } from 'node:path'

import { pairKey, type Verdict, VERDICTS, readVerdicts } from './judgments-lib.ts'
import { isArray, isObject, type Json, jsonFiles, main, OLD_JUDGMENTS, readFlags, readJson, say, ScriptError, shown, text } from './lib.ts'

type Found = { readonly id: string; readonly skill: string; readonly library: string; readonly tier: string; readonly rank: number }

/** The place of each item of every result file, with any tier, so a pair that fell to tier C is still found. A skill is at most once in one answer. */
function readAll(folder: string): Map<string, Found> {
  const all = new Map<string, Found>()
  for (const file of jsonFiles(folder)) {
    const result = readJson(file)
    if (!isObject(result) || !isArray(result['items'])) throw new ScriptError(`${file} is not a result of run-search.ts`)
    const id = text(result['id']) ?? file
    const later: readonly Json[] = isArray(result['later_items']) ? result['later_items'] : []
    for (const item of [...result['items'], ...later].filter(isObject)) {
      const found = { id, skill: text(item['skill']) ?? '', library: text(item['library']) ?? '', tier: text(item['tier']) ?? '', rank: typeof item['rank'] === 'number' ? item['rank'] : 0 }
      const key = pairKey(id, found.library, found.skill)
      if (!all.has(key)) all.set(key, found)
    }
  }
  return all
}

function declaredPairs(all: ReadonlyMap<string, Found>): Found[] {
  return [...all.values()].filter((found) => found.tier === 'A' || found.tier === 'B')
}

function reportSet(set: string, pairs: readonly Found[], verdicts: ReadonlyMap<string, Verdict>): readonly string[] {
  const counts = new Map<string, number>()
  const unjudged: string[] = []
  let tierChanges = 0
  for (const pair of pairs) {
    const key = pairKey(pair.id, pair.library, pair.skill)
    const verdict = verdicts.get(key)
    const name = verdict?.verdict ?? 'no verdict'
    counts.set(`${pair.tier} ${name}`, (counts.get(`${pair.tier} ${name}`) ?? 0) + 1)
    const changed = verdict !== undefined && verdict.tier !== pair.tier
    if (changed) tierChanges += 1
    if (verdict === undefined) unjudged.push(key)
    say('q1', `${set}: ${key} tier ${pair.tier} rank ${pair.rank}: ${name}${changed ? `; TIER CHANGED from ${verdict.tier}, verdict kept` : ''}`)
  }
  for (const tier of ['A', 'B']) say('q2', `${set}: tier ${tier}: ${[...VERDICTS, 'no verdict'].map((verdict) => `${verdict} ${counts.get(`${tier} ${verdict}`) ?? 0}`).join(', ')}`)
  say('q2', `${set}: ${pairs.length} distinct tier A or B pairs, ${unjudged.length} with no verdict, ${tierChanges} with a verdict given at another tier`)
  return unjudged
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['results', 'judgments'])
  const folders = flags.values.get('results') ?? []
  if (folders.length === 0) throw new ScriptError('give at least one --results folder')
  const judgmentFiles = flags.values.get('judgments') ?? [OLD_JUDGMENTS]
  const verdicts = readVerdicts(judgmentFiles)
  say('q0', `${verdicts.size} verdicts from ${judgmentFiles.map(shown).join(', ')}; results ${folders.map(shown).join(', ')}; a pair is item, skill and library, and keeps its verdict when its tier changed`)
  const everything = folders.map((folder) => ({ set: basename(folder), all: readAll(folder) }))
  const unjudged = new Set<string>()
  for (const { set, all } of everything) reportSet(set, declaredPairs(all), verdicts).forEach((key) => unjudged.add(key))
  for (const key of [...unjudged].sort()) say('q3', `no verdict in any judgments file: ${key}`)
  say('q3', `${unjudged.size} distinct pairs over all ${folders.length} result folders have no verdict`)
  const oldVerdicts = readVerdicts([OLD_JUDGMENTS])
  const watched = [...oldVerdicts.entries()].filter(([, verdict]) => verdict.tier === 'B' && verdict.verdict !== 'right').sort(([left], [right]) => left.localeCompare(right))
  let stillReturned = 0
  for (const [key, verdict] of watched) {
    const places = everything.map(({ set, all }) => {
      const found = all.get(key)
      return found === undefined ? `${set} not returned` : `${set} tier ${found.tier} rank ${found.rank}`
    })
    if (everything.some(({ all }) => all.get(key)?.tier === 'B')) stillReturned += 1
    say('q4', `first measurement tier B ${verdict.verdict}: ${key}: ${places.join('; ')}`)
  }
  const wrong = watched.filter(([, verdict]) => verdict.verdict === 'wrong').length
  say('q5', `${watched.length} tier B pairs judged not right in the first measurement (${wrong} wrong, ${watched.length - wrong} doubtful); ${stillReturned} of them are still at tier B in at least one result folder`)
})

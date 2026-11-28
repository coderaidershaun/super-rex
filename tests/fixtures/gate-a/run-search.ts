// Runs `rex skills search` over the library and the filler pool for each form of a folder; full answers, which hold pool text, go to the raw folder outside the repo.
// The --out folder gets results that name each pool item by skill, library, tier and rank only. Usage: see README.md in this folder.
import { writeFileSync } from 'node:fs'
import { basename, join } from 'node:path'

import {
  DEFAULT_POOL,
  DEFAULT_RAW,
  ensureFolder,
  flag,
  isArray,
  isObject,
  type Json,
  jsonFiles,
  LIBRARY,
  libraryName,
  main,
  readAnswer,
  readFlags,
  readJson,
  requiredFlag,
  say,
  ScriptError,
  search,
  text,
  writeJson,
} from './lib.ts'

/** One result with every pool item cut down to its skill, library, tier and rank. */
function namedItems(id: string, items: readonly Json[], firstRank: number, pool: string): Json[] {
  return items.map((item, at): Json => {
    if (!isObject(item)) throw new ScriptError(`item ${at + 1} of ${id} is not an object`)
    const base = { rank: firstRank + at, skill: text(item['skill']) ?? '', library: text(item['library']) ?? '', tier: text(item['tier']) ?? '' }
    return text(item['library']) === pool ? base : { ...base, slot: text(item['slot']) ?? '' }
  })
}

/** One result with every pool item cut down to its skill, library, tier and rank. `later` holds the items of the pages after the first. */
function nameOnly(id: string, indexVersion: string, raw: Json, later: readonly Json[], pool: string): Json {
  if (!isObject(raw)) throw new ScriptError(`the result of ${id} is not an object`)
  const items = isArray(raw['items']) ? raw['items'] : []
  const isPool = (value: Json | undefined): boolean => text(value) === pool
  const named = namedItems(id, items, 1, pool)
  const gaps = (isArray(raw['gaps']) ? raw['gaps'] : []).filter(isObject).map((gap): Json => {
    const kept: Record<string, Json> = { need: text(gap['need']) ?? '' }
    if (!isPool(gap['library'])) kept['reason'] = text(gap['reason']) ?? ''
    if (gap['skill'] !== undefined) kept['skill'] = text(gap['skill']) ?? ''
    if (gap['library'] !== undefined) kept['library'] = text(gap['library']) ?? ''
    return kept
  })
  const dropped = (isArray(raw['dropped']) ? raw['dropped'] : []).filter(isObject).map((entry): Json => {
    const kept: Record<string, Json> = { skill: text(entry['skill']) ?? '', library: text(entry['library']) ?? '' }
    if (!isPool(entry['library'])) kept['reason'] = text(entry['reason']) ?? ''
    return kept
  })
  const laterItems = namedItems(id, later, items.length + 1, pool)
  return { id, index_version: indexVersion, items: named, more: raw['more'] ?? 0, later_items: laterItems, gaps, unrecognised: raw['unrecognised'] ?? [], dropped }
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['forms', 'out', 'label', 'pool', 'raw', 'limit', 'library-only', 'all-pages'])
  const forms = requiredFlag(flags, 'forms')
  const out = requiredFlag(flags, 'out')
  const label = requiredFlag(flags, 'label')
  const pool = flag(flags, 'pool', DEFAULT_POOL)
  const limit = Number(flag(flags, 'limit', '10'))
  if (!Number.isInteger(limit) || limit < 1) throw new ScriptError('--limit must be a whole number of 1 or more')
  const libraryOnly = flag(flags, 'library-only', 'no') === 'yes'
  const libraries = libraryOnly ? [LIBRARY] : [LIBRARY, pool]
  const allPages = flag(flags, 'all-pages', 'no') === 'yes'
  const rawFolder = join(flag(flags, 'raw', DEFAULT_RAW), label)
  ensureFolder(out)
  ensureFolder(rawFolder)
  say('s0', `libraries: ${libraries.join(' then ')}; limit ${limit}; ${allPages ? 'every page' : 'first page only'}; raw answers to ${rawFolder}; results to ${out}`)
  const tiers = new Map<string, number>()
  let count = 0
  let indexVersion = ''
  for (const file of jsonFiles(forms)) {
    const form = readJson(file)
    const id = isObject(form) ? (text(form['id']) ?? basename(file)) : basename(file)
    const line = search(libraries, { requests: [form], limit })
    const answer = readAnswer(line)
    const result = answer.results[0]
    if (result === undefined) throw new ScriptError(`the answer for ${id} holds no result`)
    const lines = [line]
    const later: Json[] = []
    let cursor = result.cursor
    while (allPages && cursor !== null) {
      const next = search(libraries, { requests: [form], limit, cursor })
      lines.push(next)
      const page = readAnswer(next).results[0]
      later.push(...(page?.items ?? []).map((item) => item.raw))
      cursor = page?.cursor ?? null
    }
    writeFileSync(join(rawFolder, basename(file)), `${lines.join('\n')}\n`)
    indexVersion = answer.indexVersion
    result.items.forEach((item) => tiers.set(item.tier, (tiers.get(item.tier) ?? 0) + 1))
    writeJson(join(out, basename(file)), nameOnly(id, answer.indexVersion, result.raw, later, libraryName(pool)))
    count += 1
  }
  writeJson(join(out, '_run.json'), { libraries: libraries.map(libraryName), limit, all_pages: allPages, forms: count, index_version: indexVersion })
  const tierLine = ['A', 'B', 'C'].map((tier) => `tier ${tier} ${tiers.get(tier) ?? 0}`).join(', ')
  say('s1', `${count} forms searched; items on first pages: ${tierLine}; index ${indexVersion}`)
})

// The search settings that the flags of the search scripts ask for, and the name-only copy of a result that may be committed.
// A pool item keeps only its skill, library, tier and rank, because its card and reasons are outside text that nobody reviewed.
import { type Flags, isArray, isObject, type Json, type JsonObject, type KeywordSettings, ScriptError, text, wholeNumber, yesNo } from './lib.ts'

export type SearchSettings = { readonly request: JsonObject; readonly keyword: KeywordSettings }

/** The flags that `searchSettings` reads. */
export const SETTING_FLAGS = ['limit', 'keep-hashes', 'token-budget', 'keyword-ratio', 'keyword-scores']

/** The request fields and command options that the flags ask for. Only a field that is asked for is sent, so a plain run sends what the first measurement sent. */
export function searchSettings(flags: Flags): SearchSettings {
  const request: Record<string, Json> = { limit: wholeNumber(flags, 'limit', '10') }
  if (yesNo(flags, 'keep-hashes')) request['keep_hashes'] = true
  if (flags.values.has('token-budget')) request['token_budget'] = wholeNumber(flags, 'token-budget', '1')
  const ratio = flags.values.get('keyword-ratio')?.at(-1)
  const scores = yesNo(flags, 'keyword-scores')
  const keyword: KeywordSettings = ratio === undefined ? { scores } : { ratio, scores }
  return { request, keyword }
}

/** Each item cut down to its rank, skill, library and tier, and for a library item also its slot. */
function namedItems(id: string, items: readonly Json[], firstRank: number, pool: string): Json[] {
  return items.map((item, at): Json => {
    if (!isObject(item)) throw new ScriptError(`item ${at + 1} of ${id} is not an object`)
    const base = { rank: firstRank + at, skill: text(item['skill']) ?? '', library: text(item['library']) ?? '', tier: text(item['tier']) ?? '' }
    return text(item['library']) === pool ? base : { ...base, slot: text(item['slot']) ?? '' }
  })
}

/** One result with every pool item cut down to its skill, library, tier and rank. `later` holds the items of the pages after the first. */
export function nameOnly(id: string, indexVersion: string, raw: JsonObject, later: readonly Json[], pool: string): Json {
  const items = isArray(raw['items']) ? raw['items'] : []
  const isPool = (value: Json | undefined): boolean => text(value) === pool
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
  const result: Record<string, Json> = { id, index_version: indexVersion, items: namedItems(id, items, 1, pool), more: raw['more'] ?? 0 }
  if (raw['over_budget'] === true) result['over_budget'] = true
  return { ...result, later_items: namedItems(id, later, items.length + 1, pool), gaps, unrecognised: raw['unrecognised'] ?? [], dropped }
}

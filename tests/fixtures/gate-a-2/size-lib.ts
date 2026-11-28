// A copy of the binary's token estimate and page shortening at the base commit, and the rebuild of a full page from one-item pages, for the size proof.
// The copy is trusted only as far as size-proof.ts shows that it makes the binary's own decisions on real answers.
import { isObject, type Item, type Json, type JsonObject, readAnswer, type Result, ScriptError, search } from './lib.ts'

/** The estimate's constants, as the binary's page module and request defaults define them at the base commit. */
export const TOKENS_PER_ITEM = 200
const TEXT_TOKENS_PER_FIVE_BYTES = 2
const HASH_TOKENS_PER_EIGHT_BYTES = 5
const HASH_DIGITS = 64
/** A budget no page reaches, so a one-item page is never shortened and shows every field of its item. */
const UNLIMITED_BUDGET = 1_000_000_000

/** The tokens that the compact JSON `text` costs a caller, rounded up: runs of exactly 64 lower-case hex digits count as hashes. */
export function estimatedTokens(json: string): number {
  let hashBytes = 0
  let runLength = 0
  for (const char of `${json} `) {
    if (/^[0-9a-f]$/.test(char)) {
      runLength += 1
      continue
    }
    if (runLength === HASH_DIGITS) hashBytes += runLength
    runLength = 0
  }
  const textBytes = Buffer.byteLength(json) - hashBytes
  return Math.ceil((textBytes * TEXT_TOKENS_PER_FIVE_BYTES) / 5) + Math.ceil((hashBytes * HASH_TOKENS_PER_EIGHT_BYTES) / 8)
}

/** The size settings of one request: `limit`, and `token_budget` and `keep_hashes` when given. */
export type SizeSettings = { readonly limit: number; readonly tokenBudget: number | null; readonly keepHashes: boolean }

export function budget(settings: SizeSettings): number {
  return settings.tokenBudget ?? TOKENS_PER_ITEM * settings.limit
}

export function requestFields(settings: SizeSettings): JsonObject {
  const fields: Record<string, Json> = { limit: settings.limit }
  if (settings.tokenBudget !== null) fields['token_budget'] = settings.tokenBudget
  if (settings.keepHashes) fields['keep_hashes'] = true
  return fields
}

type MutableItem = Record<string, Json>
type MutableGap = Record<string, Json>
export type MutablePage = { request: Json; items: MutableItem[]; more: number; cursor: string | null; over_budget?: true; gaps: MutableGap[]; unrecognised: Json; dropped: Json }

export function pageTokens(page: MutablePage | JsonObject): number {
  return estimatedTokens(JSON.stringify(page))
}

function cutAtSemicolon(holder: Record<string, Json>, field: string): void {
  const value = holder[field]
  if (typeof value === 'string' && value.includes('; ')) holder[field] = value.slice(0, value.indexOf('; '))
}

/** The shortening steps, in the binary's order. Each applies to the whole page. */
export const STEPS: readonly { readonly name: string; readonly skippedWhenKeepingHashes: boolean; readonly apply: (page: MutablePage) => void }[] = [
  {
    name: 'cut reasons',
    skippedWhenKeepingHashes: false,
    apply: (page) => {
      page.items.forEach((item) => cutAtSemicolon(item, 'why'))
      page.gaps.forEach((gap) => cutAtSemicolon(gap, 'reason'))
    },
  },
  {
    name: 'leave out hashes',
    skippedWhenKeepingHashes: true,
    apply: (page) =>
      page.items.forEach((item) => {
        delete item['sha256']
        const files = item['files']
        if (Array.isArray(files)) item['files'] = files.map((file: Json) => (isObject(file) ? Object.fromEntries(Object.entries(file).filter(([key]) => key !== 'sha256')) : file))
      }),
  },
  { name: 'leave out words', skippedWhenKeepingHashes: false, apply: (page) => page.items.forEach((item) => void delete item['words']) },
  { name: 'leave out placement', skippedWhenKeepingHashes: false, apply: (page) => page.items.forEach((item) => void delete item['placement']) },
]

/** The names of what the fit can do, in order, for the counts of the proof. */
export const OUTCOMES = ['fits whole', ...STEPS.map((step) => step.name), 'drop items', 'over_budget']

export function clonePage(page: MutablePage): MutablePage {
  return JSON.parse(JSON.stringify(page)) as MutablePage
}

export type Binding = { readonly version: string; readonly request: string }

function cursorText(binding: Binding, offset: number): string {
  return `c1.${offset}.${binding.version}.${binding.request}`
}

export type Fit = { readonly page: MutablePage; readonly applied: readonly string[]; readonly popped: number; readonly overBudget: boolean }

/** What the binary's fit does to a full page at offset 0: each step while the page is over budget, then items to the next page, then `over_budget`. */
export function simulateFit(full: MutablePage, binding: Binding | null, settings: SizeSettings): Fit {
  const page = clonePage(full)
  const limit = budget(settings)
  const fits = (): boolean => pageTokens(page) <= limit
  const setCursor = (): void => {
    if (page.more > 0 && binding === null) throw new ScriptError('a page has more items but the answer gave no cursor binding')
    page.cursor = page.more > 0 && binding !== null ? cursorText(binding, page.items.length) : null
  }
  setCursor()
  const applied: string[] = []
  for (const step of STEPS.filter((entry) => !(settings.keepHashes && entry.skippedWhenKeepingHashes))) {
    if (fits()) return { page, applied, popped: 0, overBudget: false }
    step.apply(page)
    applied.push(step.name)
  }
  let popped = 0
  while (!fits() && page.items.length > 1) {
    page.items.pop()
    page.more += 1
    popped += 1
    setCursor()
  }
  const overBudget = !fits()
  if (overBudget) return { page: withOverBudget(page), applied, popped, overBudget }
  return { page, applied, popped, overBudget }
}

/** The page with `over_budget` written where the binary writes it: after `cursor`. */
function withOverBudget(page: MutablePage): MutablePage {
  return { request: page.request, items: page.items, more: page.more, cursor: page.cursor, over_budget: true, gaps: page.gaps, unrecognised: page.unrecognised, dropped: page.dropped }
}

export type FullPage = {
  readonly full: MutablePage
  readonly binding: Binding | null
  readonly total: number
  /** Why the rebuilt page may differ from the page the binary built before it shortened it; empty when exact. */
  readonly inexact: readonly string[]
}

/** The order in which the binary writes the fields of an item. */
const ITEM_FIELDS = ['skill', 'library', 'kind', 'category', 'slot', 'tier', 'why', 'score', 'best_score', 'card', 'not_when', 'confirm', 'placement', 'sha256', 'words', 'files', 'open_when']

function inFieldOrder(item: MutableItem): MutableItem {
  const unknown = Object.keys(item).filter((key) => !ITEM_FIELDS.includes(key))
  if (unknown.length > 0) throw new ScriptError(`an item has fields this copy does not know: ${unknown.join(', ')}`)
  const ordered: MutableItem = {}
  for (const key of ITEM_FIELDS) {
    const value = item[key]
    if (value !== undefined) ordered[key] = value
  }
  return ordered
}

function readBinding(cursor: string | null): Binding | null {
  if (cursor === null) return null
  const parts = cursor.split('.')
  const [tag, , version, request] = parts
  if (parts.length !== 4 || tag !== 'c1' || version === undefined || request === undefined) throw new ScriptError(`cursor ${cursor} is not of the form c1.<offset>.<version>.<request>`)
  return { version, request }
}

/** One item at `offset` on a page of one item with no size limit, so nothing of it is left out. */
function onePage(libraries: readonly string[], form: Json, offset: number, binding: Binding | null): Result {
  const cursor = offset === 0 || binding === null ? null : cursorText(binding, offset)
  const line = search(libraries, { requests: [form], limit: 1, cursor, token_budget: UNLIMITED_BUDGET, keep_hashes: true })
  const result = readAnswer(line).results[0]
  if (result === undefined) throw new ScriptError(`the one-item page at offset ${offset} holds no result`)
  if (result.overBudget) throw new ScriptError(`the one-item page at offset ${offset} is over a budget of ${UNLIMITED_BUDGET} tokens`)
  return result
}

/** The full first page that the binary builds before it shortens it, for a page of `limit` items, rebuilt from one-item pages. */
export function fullPage(libraries: readonly string[], form: Json, limit: number): FullPage {
  const first = onePage(libraries, form, 0, null)
  const total = first.items.length + first.more
  const binding = readBinding(first.cursor)
  const shown = Math.min(limit, total)
  const items: MutableItem[] = []
  const inexact: string[] = []
  for (let offset = 0; offset < shown; offset += 1) {
    const single = offset === 0 ? first : onePage(libraries, form, offset, binding)
    const item: Item | undefined = single.items[0]
    if (item === undefined) throw new ScriptError(`the one-item page at offset ${offset} holds no item`)
    items.push(inFieldOrder({ ...item.raw }))
  }
  const more = total - shown
  const raw = first.raw
  const gaps = Array.isArray(raw['gaps']) ? raw['gaps'].filter(isObject).map((gap) => ({ ...gap })) : []
  const full: MutablePage = { request: raw['request'] ?? '', items, more, cursor: null, gaps, unrecognised: raw['unrecognised'] ?? [], dropped: raw['dropped'] ?? [] }
  return { full, binding, total, inexact }
}

/** Puts on the full page the confirm questions of the binary's page. A question is asked once per page, so a one-item page may ask one that the full page asks only on an earlier item. */
export function withPageConfirms(rebuilt: FullPage, answer: Result): FullPage {
  const full = clonePage(rebuilt.full)
  const inexact = [...rebuilt.inexact]
  full.items.forEach((item, at) => {
    const onPage = answer.items[at]
    if (onPage !== undefined) {
      if (onPage.skill !== item['skill']) throw new ScriptError(`item ${at + 1} is ${onPage.skill} on the page but ${String(item['skill'])} on its own page`)
      const confirm = onPage.raw['confirm']
      if (confirm === undefined) delete item['confirm']
      else item['confirm'] = confirm
    } else if (typeof item['confirm'] === 'string' && item['confirm'].includes('Does the task have')) {
      inexact.push(`${String(item['skill'])} left the page and asks a scope question, which an earlier item may have asked`)
    }
  })
  return { ...rebuilt, full, inexact }
}

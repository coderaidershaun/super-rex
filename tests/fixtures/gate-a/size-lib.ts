// A copy of the binary's token estimate and page shortening, and the rebuild of a full page from one-item pages, for the answer-size script and its proof.
// The copy is trusted only as far as size-proof.ts shows that it makes the binary's own decisions on real answers.
import { isArray, isObject, type Item, type Json, type JsonObject, readAnswer, type Result, ScriptError, search } from './lib.ts'

/** The estimate's constants, as the binary defines them in its page module at the base commit. */
export const MAX_TOKENS_PER_ITEM = 120
export const DEFAULT_LIMIT = 10
const TEXT_TOKENS_PER_FIVE_BYTES = 2
const HASH_TOKENS_PER_EIGHT_BYTES = 5
const HASH_DIGITS = 64

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

export function budget(itemCount: number): number {
  return MAX_TOKENS_PER_ITEM * Math.max(itemCount, DEFAULT_LIMIT)
}

type MutableItem = Record<string, Json>
type MutablePage = { request: Json; items: MutableItem[]; more: number; cursor: string | null; gaps: Json; unrecognised: Json; dropped: Json }

export function pageTokens(page: MutablePage | JsonObject): number {
  return estimatedTokens(JSON.stringify(page))
}

function itemsOf(page: MutablePage): MutableItem[] {
  return page.items
}

/** The shortening steps, in the binary's order, each applied to every item. */
export const STEPS: readonly { readonly name: string; readonly apply: (item: MutableItem) => void }[] = [
  {
    name: 'cut why',
    apply: (item) => {
      const why = item['why']
      if (typeof why === 'string' && why.includes('; ')) item['why'] = why.slice(0, why.indexOf('; '))
    },
  },
  {
    name: 'leave out hashes',
    apply: (item) => {
      delete item['sha256']
      const files = item['files']
      if (isArray(files)) item['files'] = files.map((file) => (isObject(file) ? Object.fromEntries(Object.entries(file).filter(([key]) => key !== 'sha256')) : file))
    },
  },
  { name: 'leave out words', apply: (item) => void delete item['words'] },
  { name: 'leave out placement', apply: (item) => void delete item['placement'] },
  { name: 'leave out open_when', apply: (item) => void delete item['open_when'] },
  { name: 'leave out files', apply: (item) => void delete item['files'] },
]

export function clonePage(page: MutablePage): MutablePage {
  return JSON.parse(JSON.stringify(page)) as MutablePage
}

export type Binding = { readonly version: string; readonly request: string }

function cursorText(binding: Binding, offset: number): string {
  return `c1.${offset}.${binding.version}.${binding.request}`
}

/** What the binary's fit does to a full page at offset 0: each step while the page is over budget, then items to the next page. */
export function simulateFit(full: MutablePage, binding: Binding | null): { page: MutablePage; steps: number; popped: number } {
  const page = clonePage(full)
  const fits = (): boolean => pageTokens(page) <= budget(page.items.length)
  if (page.items.length === 0) return { page, steps: 0, popped: 0 }
  let steps = 0
  for (const step of STEPS) {
    if (fits()) return { page, steps, popped: 0 }
    itemsOf(page).forEach(step.apply)
    steps += 1
  }
  let popped = 0
  while (!fits() && page.items.length > 1) {
    page.items.pop()
    page.more += 1
    popped += 1
    if (binding === null) throw new ScriptError('a page lost an item but the answer gave no cursor binding')
    page.cursor = cursorText(binding, page.items.length)
  }
  return { page, steps, popped }
}

export type FullPage = {
  readonly answer: Result
  readonly line: string
  readonly full: MutablePage
  readonly binding: Binding | null
  /** Why the rebuilt page may differ from the page the binary built before it shortened it; empty when exact. */
  readonly inexact: readonly string[]
}

/** The order in which the binary writes the fields of an item. */
const ITEM_FIELDS = ['skill', 'library', 'kind', 'category', 'slot', 'tier', 'why', 'card', 'not_when', 'confirm', 'placement', 'sha256', 'words', 'files', 'open_when']

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

function onePage(libraries: readonly string[], form: Json, offset: number, binding: Binding | null): Item {
  const cursor = offset === 0 || binding === null ? null : cursorText(binding, offset)
  const result = readAnswer(search(libraries, { requests: [form], limit: 1, cursor })).results[0]
  const item = result?.items[0]
  if (item === undefined) throw new ScriptError(`the one-item page at offset ${offset} holds no item`)
  return item
}

/** The answer at `limit`, and the full page the binary built before shortening it, rebuilt from one-item pages, which fit unshortened. */
export function fullPage(libraries: readonly string[], form: Json, limit: number): FullPage {
  const line = search(libraries, { requests: [form], limit })
  if (JSON.stringify(JSON.parse(line)) !== line) throw new ScriptError('the answer line does not equal its own compact JSON, so sizes of rebuilt pages would not match')
  const answer = readAnswer(line).results[0]
  if (answer === undefined) throw new ScriptError('the answer holds no result')
  const total = answer.items.length + answer.more
  const shown = Math.min(limit, total)
  const needsOwnCursor = answer.cursor === null && shown > 1
  const firstOne = needsOwnCursor ? readAnswer(search(libraries, { requests: [form], limit: 1 })).results[0] : undefined
  const binding = readBinding(answer.cursor ?? firstOne?.cursor ?? null)
  const inexact: string[] = []
  const items: MutableItem[] = []
  for (let offset = 0; offset < shown; offset += 1) {
    const single = onePage(libraries, form, offset, binding)
    const merged: MutableItem = { ...single.raw }
    if (!('sha256' in merged) || !('words' in merged)) inexact.push(`the one-item page of ${single.skill} was itself shortened`)
    const onPage = answer.items[offset]
    if (onPage !== undefined) {
      if (onPage.skill !== single.skill) throw new ScriptError(`item ${offset + 1} is ${onPage.skill} on the page but ${single.skill} on its own page`)
      const confirm = onPage.raw['confirm']
      if (confirm === undefined) delete merged['confirm']
      else merged['confirm'] = confirm
    } else if (typeof merged['confirm'] === 'string' && merged['confirm'].includes('Does the task have')) {
      inexact.push(`${single.skill} left the page and asks a scope question, which an earlier item may have asked`)
    }
    items.push(inFieldOrder(merged))
  }
  const raw = answer.raw
  const more = total - shown
  const full: MutablePage = {
    request: raw['request'] ?? '',
    items,
    more,
    cursor: more > 0 && binding !== null ? cursorText(binding, shown) : null,
    gaps: raw['gaps'] ?? [],
    unrecognised: raw['unrecognised'] ?? [],
    dropped: raw['dropped'] ?? [],
  }
  return { answer, line, full, binding, inexact }
}

/** The full page with every item's files and open_when left out. */
export function withoutFilesAndOpenWhen(page: MutablePage): MutablePage {
  const copy = clonePage(page)
  copy.items.forEach((item) => {
    delete item['files']
    delete item['open_when']
  })
  return copy
}

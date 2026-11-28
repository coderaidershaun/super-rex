// Costs the three options of finding G5 on real answers with the proven size copy of size-lib.ts: a larger budget, shorter cards, or files without hashes and items to the next page sooner.
// It changes no setting of the binary; it rebuilds each full page and shortens it in another order. Usage: node tests/fixtures/gate-a/results/g5-options.ts --forms <filled folder>
import { DEFAULT_POOL, flag, isObject, jsonFiles, LIBRARY, main, readFlags, readJson, requiredFlag, say, text } from '../lib.ts'
import { budget, clonePage, fullPage, pageTokens, STEPS } from '../size-lib.ts'

type Page = ReturnType<typeof fullPage>['full']

const LIMIT = 10
const CARD_WORDS = 30
/** Steps 1 to 4 of the binary: cut why, leave out hashes, words and placement. Files and open_when stay. */
const KEEP_FILES_STEPS = STEPS.slice(0, 4)

/** A token difference from the budget, with its sign. */
function overBy(tokens: number, limit: number): string {
  const over = tokens - limit
  return over > 0 ? `${over} over` : `${-over} under`
}

function shortened(page: Page, steps: typeof STEPS): Page {
  const copy = clonePage(page)
  for (const step of steps) copy.items.forEach(step.apply)
  return copy
}

/** The page with each card cut to its first `words` words. */
function withCards(page: Page, words: number): Page {
  const copy = clonePage(page)
  copy.items.forEach((item) => {
    const card = item['card']
    if (typeof card === 'string') item['card'] = card.split(/\s+/).filter((word) => word !== '').slice(0, words).join(' ')
  })
  return copy
}

/** The most card words, 30 down to 0, at which the page fits its budget; -1 when even empty cards do not fit. */
function cardWordsThatFit(page: Page): number {
  for (let words = CARD_WORDS; words >= 0; words -= 1) if (pageTokens(withCards(page, words)) <= budget(page.items.length)) return words
  return -1
}

/** How many items stay on the first page when items move to the next page before files and open_when are left out. */
function itemsKept(page: Page): number {
  const copy = clonePage(page)
  while (pageTokens(copy) > budget(copy.items.length) && copy.items.length > 1) copy.items.pop()
  return copy.items.length
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['forms', 'pool'])
  const forms = requiredFlag(flags, 'forms')
  const libraries = [LIBRARY, flag(flags, 'pool', DEFAULT_POOL)]
  say('o0', `options of G5 on each answer at limit ${LIMIT}, library then pool; budget = 120 tokens per item, at least 10 items`)
  let tenItems = 0
  for (const file of jsonFiles(forms)) {
    const form = readJson(file)
    const id = isObject(form) ? (text(form['id']) ?? file) : file
    const page = fullPage(libraries, form, LIMIT)
    const count = page.full.items.length
    if (count === LIMIT) tenItems += 1
    const limit = budget(count)
    const full = pageTokens(page.full)
    const keepFiles = shortened(page.full, KEEP_FILES_STEPS)
    const allSteps = shortened(page.full, STEPS)
    const fileCount = page.full.items.reduce((sum, item) => sum + (Array.isArray(item['files']) ? item['files'].length : 0), 0)
    const openWhen = page.full.items.reduce((sum, item) => sum + (Array.isArray(item['open_when']) ? item['open_when'].length : 0), 0)
    say('o1', `${id}: ${count} items, ${fileCount} files, ${openWhen} open_when; budget ${limit}`)
    say('o2', `${id}: (a) budget for all ${count} items: ${full} tokens with every field (${overBy(full, limit)}); ${pageTokens(keepFiles)} with files and open_when but no hashes, words, placement or long why (${overBy(pageTokens(keepFiles), limit)}); ${pageTokens(allSteps)} with every optional field left out (${overBy(pageTokens(allSteps), limit)})`)
    const keptWords = cardWordsThatFit(keepFiles)
    const leanWords = cardWordsThatFit(allSteps)
    say('o3', `${id}: (b) most card words that fit ${limit}: ${keptWords < 0 ? 'none, even empty cards' : keptWords} with files and open_when kept; ${leanWords < 0 ? 'none, even empty cards' : leanWords} with every optional field left out`)
    const kept = itemsKept(keepFiles)
    say('o4', `${id}: (c) files kept without hashes, items moved before files and open_when: ${kept} of ${count} items stay (${count - kept} to the next page); the binary keeps ${page.answer.items.length} items with no files or open_when`)
  }
  say('o5', `${tenItems} answers with 10 items on their full first page`)
})

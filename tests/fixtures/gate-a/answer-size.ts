// Prints the estimated token size of each form's full answer page, with and without files and open_when, and after each shortening step, beside what the binary returned.
// Usage: node tests/fixtures/gate-a/answer-size.ts --forms <file or folder> [--pool <folder>] [--limit 10] [--library-only yes]
import { BASE_COMMIT, DEFAULT_POOL, flag, isObject, jsonFiles, LIBRARY, main, readFlags, readJson, requiredFlag, say, ScriptError, text } from './lib.ts'
import { budget, clonePage, fullPage, pageTokens, STEPS, withoutFilesAndOpenWhen } from './size-lib.ts'

main(() => {
  const flags = readFlags(process.argv.slice(2), ['forms', 'pool', 'limit', 'library-only'])
  const forms = requiredFlag(flags, 'forms')
  const limit = Number(flag(flags, 'limit', '10'))
  if (!Number.isInteger(limit) || limit < 1) throw new ScriptError('--limit must be a whole number of 1 or more')
  const libraries = flag(flags, 'library-only', 'no') === 'yes' ? [LIBRARY] : [LIBRARY, flag(flags, 'pool', DEFAULT_POOL)]
  say('z0', `sizes measured on base ${BASE_COMMIT}, before a parallel job adds open_when items to up to 12 library headers; libraries ${libraries.join(' then ')}; limit ${limit}`)
  let count = 0
  let overBudget = 0
  for (const file of jsonFiles(forms)) {
    const form = readJson(file)
    const id = isObject(form) ? (text(form['id']) ?? file) : file
    const page = fullPage(libraries, form, limit)
    const full = pageTokens(page.full)
    const lean = pageTokens(withoutFilesAndOpenWhen(page.full))
    const files = page.full.items.reduce((sum, item) => sum + (Array.isArray(item['files']) ? item['files'].length : 0), 0)
    const openWhen = page.full.items.reduce((sum, item) => sum + (Array.isArray(item['open_when']) ? item['open_when'].length : 0), 0)
    const stepped = clonePage(page.full)
    const afterSteps = STEPS.map((step) => {
      stepped.items.forEach(step.apply)
      return `${step.name} ${pageTokens(stepped)}`
    })
    const limitTokens = budget(page.full.items.length)
    count += 1
    if (full > limitTokens) overBudget += 1
    say('z1', `${id}: ${page.full.items.length} items (${files} files, ${openWhen} open_when); full ${full} tokens; without files and open_when ${lean}; budget ${limitTokens}`)
    say('z1', `${id}: after each step: ${afterSteps.join(', ')}; binary returned ${page.answer.items.length} items, ${pageTokens(page.answer.raw)} tokens, ${page.answer.more} more`)
    if (page.inexact.length > 0) say('z1', `${id}: rebuilt page inexact: ${page.inexact.join('; ')}`)
  }
  say('z2', `${count} answers sized; ${overBudget} full pages over their budget`)
})

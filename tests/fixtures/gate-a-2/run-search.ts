// Runs `rex skills search` over the library and the filler pool for each form of a folder; full answers, which hold pool text, go to the raw folder outside the repo.
// The --out folder gets results that name each pool item by skill, library, tier and rank only. Usage: see README.md in this folder.
import { writeFileSync } from 'node:fs'
import { basename, join } from 'node:path'

import { DEFAULT_POOL, DEFAULT_RAW, ensureFolder, firstResult, flag, formId, type Json, jsonFiles, LIBRARY, libraryName, main, readAnswer, readFlags, readJson, requiredFlag, say, search, settingArgs, shown, writeJson, yesNo } from './lib.ts'
import { nameOnly, searchSettings, SETTING_FLAGS } from './search-lib.ts'

main(() => {
  const known = ['forms', 'out', 'label', 'pool', 'raw', 'library-only', 'all-pages', ...SETTING_FLAGS]
  const flags = readFlags(process.argv.slice(2), known)
  const forms = requiredFlag(flags, 'forms')
  const out = requiredFlag(flags, 'out')
  const label = requiredFlag(flags, 'label')
  const pool = flag(flags, 'pool', DEFAULT_POOL)
  const settings = searchSettings(flags)
  const libraries = yesNo(flags, 'library-only') ? [LIBRARY] : [LIBRARY, pool]
  const allPages = yesNo(flags, 'all-pages')
  const rawFolder = join(flag(flags, 'raw', DEFAULT_RAW), label)
  ensureFolder(out)
  ensureFolder(rawFolder)
  const options = settingArgs(settings.keyword).join(' ') || 'none'
  say('s0', `libraries: ${libraries.map(shown).join(' then ')}; request ${JSON.stringify(settings.request)}; options ${options}; ${allPages ? 'every page' : 'first page only'}; raw answers to ${shown(rawFolder)}; results to ${shown(out)}`)
  const tiers = new Map<string, number>()
  let count = 0
  let overBudget = 0
  let indexVersion = ''
  for (const file of jsonFiles(forms)) {
    const form = readJson(file)
    const id = formId(form, file)
    const line = search(libraries, { ...settings.request, requests: [form] }, settings.keyword)
    const result = firstResult(line, id)
    const lines = [line]
    const later: Json[] = []
    let cursor = result.cursor
    while (allPages && cursor !== null) {
      const next = search(libraries, { ...settings.request, requests: [form], cursor }, settings.keyword)
      lines.push(next)
      const page = firstResult(next, id)
      later.push(...page.items.map((item) => item.raw))
      cursor = page.cursor
    }
    writeFileSync(join(rawFolder, basename(file)), `${lines.join('\n')}\n`)
    indexVersion = readAnswer(line).indexVersion
    result.items.forEach((item) => tiers.set(item.tier, (tiers.get(item.tier) ?? 0) + 1))
    if (result.overBudget) overBudget += 1
    writeJson(join(out, basename(file)), nameOnly(id, indexVersion, result.raw, later, libraryName(pool)))
    count += 1
  }
  writeJson(join(out, '_run.json'), { libraries: libraries.map(libraryName), request: settings.request, options, all_pages: allPages, forms: count, index_version: indexVersion })
  const tierLine = ['A', 'B', 'C'].map((tier) => `tier ${tier} ${tiers.get(tier) ?? 0}`).join(', ')
  say('s1', `${count} forms searched; items on first pages: ${tierLine}; first pages with over_budget: ${overBudget}; index ${indexVersion}`)
})

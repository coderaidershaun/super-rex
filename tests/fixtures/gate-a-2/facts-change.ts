// Compares two fills of the same forms and their search results: how many repo facts changed, and how many tier A or B items changed with them.
// Usage: node tests/fixtures/gate-a-2/facts-change.ts --old-forms <filled folder> --new-forms <filled folder> --old-results <results folder> --new-results <results folder>
import { basename, join } from 'node:path'

import { isArray, isObject, type Json, type JsonObject, jsonFiles, main, readFlags, readJson, requiredFlag, say, ScriptError, shown, text, textList } from './lib.ts'

function factsOf(file: string): JsonObject {
  const form = readJson(file)
  const facts = isObject(form) ? form['facts'] : undefined
  if (!isObject(facts)) throw new ScriptError(`${file} has no facts object`)
  return facts
}

/** `rank tier:library/skill` of each tier A and B item over all pages of a result of run-search.ts. */
function declaredOf(file: string): string[] {
  const result = readJson(file)
  if (!isObject(result) || !isArray(result['items'])) throw new ScriptError(`${file} is not a result of run-search.ts`)
  const later: readonly Json[] = isArray(result['later_items']) ? result['later_items'] : []
  return [...result['items'], ...later].filter(isObject).flatMap((item) => {
    const tier = text(item['tier']) ?? ''
    return tier === 'A' || tier === 'B' ? [`${typeof item['rank'] === 'number' ? item['rank'] : 0} ${tier}:${text(item['library']) ?? ''}/${text(item['skill']) ?? ''}`] : []
  })
}

function withoutRank(entry: string): string {
  return entry.slice(entry.indexOf(' ') + 1)
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['old-forms', 'new-forms', 'old-results', 'new-results'])
  const oldForms = requiredFlag(flags, 'old-forms')
  const newForms = requiredFlag(flags, 'new-forms')
  const oldResults = requiredFlag(flags, 'old-results')
  const newResults = requiredFlag(flags, 'new-results')
  say('c0', `facts of ${shown(oldForms)} against ${shown(newForms)}; tier A and B items of ${shown(oldResults)} against ${shown(newResults)}, over all pages`)
  let changedFacts = 0
  let formsWithChangedFacts = 0
  let formsWithChangedItems = 0
  let itemsAdded = 0
  let itemsRemoved = 0
  let itemsMoved = 0
  const addedPaths = new Set<string>()
  const removedPaths = new Set<string>()
  const files = jsonFiles(oldForms)
  for (const file of files) {
    const name = basename(file)
    const before = factsOf(file)
    const after = factsOf(join(newForms, name))
    const changes: string[] = []
    for (const fact of [...new Set([...Object.keys(before), ...Object.keys(after)])].sort()) {
      if (JSON.stringify(before[fact]) === JSON.stringify(after[fact])) continue
      changedFacts += 1
      const oldValues = new Set(textList(before[fact]))
      const newValues = new Set(textList(after[fact]))
      const added = [...newValues].filter((value) => !oldValues.has(value))
      const removed = [...oldValues].filter((value) => !newValues.has(value))
      if (fact === 'repo_has') {
        added.forEach((path) => addedPaths.add(path))
        removed.forEach((path) => removedPaths.add(path))
      }
      changes.push(isArray(before[fact]) || isArray(after[fact]) ? `${fact} +${added.length} -${removed.length}${fact === 'repo_has' ? '' : ` (${[...added.map((value) => `+${value}`), ...removed.map((value) => `-${value}`)].join(' ')})`}` : `${fact} ${JSON.stringify(before[fact] ?? null)} to ${JSON.stringify(after[fact] ?? null)}`)
    }
    if (changes.length > 0) formsWithChangedFacts += 1
    const oldItems = declaredOf(join(oldResults, name))
    const newItems = declaredOf(join(newResults, name))
    const oldNames = oldItems.map(withoutRank)
    const newNames = newItems.map(withoutRank)
    const added = newItems.filter((entry) => !oldNames.includes(withoutRank(entry)))
    const removed = oldItems.filter((entry) => !newNames.includes(withoutRank(entry)))
    const moved = newItems.filter((entry) => oldNames.includes(withoutRank(entry)) && !oldItems.includes(entry))
    const isSame = oldItems.join(' ') === newItems.join(' ')
    if (!isSame) formsWithChangedItems += 1
    itemsAdded += added.length
    itemsRemoved += removed.length
    itemsMoved += moved.length
    say('c1', `${name.replace(/\.json$/, '')}: facts changed: ${changes.join('; ') || 'none'}; tier A and B items ${isSame ? 'same' : `CHANGED (added ${added.join(', ') || 'none'}; removed ${removed.join(', ') || 'none'}; new rank ${moved.join(', ') || 'none'})`}`)
  }
  say('c2', `repo_has paths added: ${[...addedPaths].sort().join(' ') || 'none'}`)
  say('c2', `repo_has paths removed: ${[...removedPaths].sort().join(' ') || 'none'}`)
  say('c3', `${files.length} forms; ${changedFacts} changed facts in ${formsWithChangedFacts} forms (${addedPaths.size} repo paths added, ${removedPaths.size} removed); tier A and B items changed in ${formsWithChangedItems} forms: ${itemsAdded} added, ${itemsRemoved} removed, ${itemsMoved} at a new rank`)
})

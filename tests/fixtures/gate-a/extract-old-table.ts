// Writes old-table.json: the skills that the old table gave each item of tasks.json, from its Skills and Open when cells.
// Usage: node tests/fixtures/gate-a/extract-old-table.ts — run it only after tasks.json is written and its sha256 recorded.
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

import { column, decisionBlock, JOBS, readBreakdown } from './breakdown.ts'
import { HERE, type Json, main, say, ScriptError, sha256, TASKS_FILE, writeJson } from './lib.ts'

const OLD_TABLE_FILE = join(HERE, 'old-table.json')

/** Each skill name in a code span, such as `rex-code-tdd` or `rex-code-tdd/mocking.md`, in order and once. */
function skillNames(cell: string): string[] {
  const names = [...cell.matchAll(/`([a-z0-9][a-z0-9-]*(?:\/[A-Za-z0-9_./-]+\.md)?)`/g)].map((found) => found[1] ?? '')
  return [...new Set(names.filter((name) => /^[a-z0-9-]+(\/.*)?$/.test(name) && name.includes('-')))]
}

main(() => {
  if (!existsSync(TASKS_FILE)) throw new ScriptError(`${TASKS_FILE} must exist first`)
  say('c2', `tasks.json sha256 ${sha256(readFileSync(TASKS_FILE, 'utf8'))} (read before this file is written)`)
  const items: Json[] = []
  let named = 0
  for (const job of JOBS) {
    const breakdown = readBreakdown(decisionBlock(job))
    for (const row of breakdown.rows) {
      const skillsCell = column(breakdown, row, 'skills')
      const openWhenCell = column(breakdown, row, 'open when')
      if (skillNames(skillsCell).length > 0) named += 1
      items.push({
        id: `${job}-${row.task}`,
        skills: skillNames(skillsCell),
        open_when: skillNames(openWhenCell),
        skills_cell: skillsCell,
        open_when_cell: openWhenCell,
      })
    }
  }
  writeJson(OLD_TABLE_FILE, {
    about: 'The skills that the old table gave each item: names in code spans of its Skills and Open when cells, and both cells verbatim.',
    items,
  })
  say('c2', `wrote ${OLD_TABLE_FILE}: ${items.length} items, ${named} with at least one skill in the Skills cell, sha256 ${sha256(readFileSync(OLD_TABLE_FILE, 'utf8'))}`)
})

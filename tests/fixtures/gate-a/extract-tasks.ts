// Writes tasks.json: one item per task row of the WORK BREAKDOWN of each measured job, with its Outcome, Evidence and Owns cells verbatim.
// Usage: node tests/fixtures/gate-a/extract-tasks.ts [--sample] — with --sample it parses the format job only and writes nothing.
import { readFileSync } from 'node:fs'

import { column, decisionBlock, JOBS, readBreakdown, SAMPLE_JOB } from './breakdown.ts'
import { type Json, main, say, sha256, TASKS_FILE, writeJson } from './lib.ts'

function describe(job: string, number: string): { items: Json[]; source: Json } {
  const block = decisionBlock(job)
  const breakdown = readBreakdown(block)
  say(number, `${job}: ${block.file} (${block.note}), sha256 ${block.sha256}, ${breakdown.rows.length} task rows, columns: ${breakdown.headers.join(' | ')}`)
  const items = breakdown.rows.map((row) => ({
    id: `${job}-${row.task}`,
    job,
    task: row.task,
    row: row.kind,
    outcome: column(breakdown, row, 'outcome'),
    evidence: column(breakdown, row, 'evidence'),
    owns: column(breakdown, row, 'owns'),
  }))
  const empty = items.filter((item) => item.outcome === '' || item.evidence === '' || item.owns === '').length
  const roles = items.filter((item) => item.row === 'role').length
  say(number, `${job}: row IDs ${items.map((item) => item.task).join(' ')}; ${items.length - roles} task rows, ${roles} role rows; rows with an empty Outcome, Evidence or Owns cell: ${empty}`)
  return { items, source: { job, file: block.file, note: block.note, sha256: block.sha256 } }
}

main(() => {
  if (process.argv.includes('--sample')) {
    const sample = describe(SAMPLE_JOB, 'c0')
    say('c0', `format sample parsed: ${sample.items.length} items; nothing written`)
    return
  }
  const items: Json[] = []
  const sources: Json[] = []
  JOBS.forEach((job, at) => {
    const found = describe(job, `c1.${at + 1}`)
    items.push(...found.items)
    sources.push(found.source)
  })
  writeJson(TASKS_FILE, {
    about: 'One item per task row of the WORK BREAKDOWN table of each measured job: Outcome, Evidence and Owns, verbatim. The Skills and Open when columns are not here.',
    jobs: sources,
    items,
  })
  say('c1', `wrote ${TASKS_FILE}: ${items.length} items from ${JOBS.length} jobs, sha256 ${sha256(readFileSync(TASKS_FILE, 'utf8'))}`)
})

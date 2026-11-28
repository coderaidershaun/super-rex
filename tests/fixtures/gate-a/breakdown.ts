// Reads the WORK BREAKDOWN table of a decision block into rows of cells keyed by column name, so the task texts and the old table are cut from one parse.
// It never prints a cell, so a caller that writes only some columns shows no other column's text.
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

import { ScriptError, sha256 } from './lib.ts'

export const PLANS = '/Users/shaun/Code/DEVELOPMENT/super-rex/.rex/plans'

/** The measured jobs, in the order of the brief. 0ganyb6d6v, 0h0vicc9dg and 0h0vht64cg are left out on purpose. */
export const JOBS = ['0h0962klnd', '0h096dyeq9', '0h0b8x725g', '0h0d1109wo', '0h0dnqm24i', '0h0f8vnww4', '0h0fitv57s'] as const

/** The job whose block gave the format; the earlier tests used it, so it is never measured. */
export const SAMPLE_JOB = '0ganyb6d6v'

export type Block = { readonly job: string; readonly file: string; readonly sha256: string; readonly text: string; readonly note: string }

/** A `task` row has an ID such as T3. A `role` row names a role that has no task, such as a reviewer. */
export type RowKind = 'task' | 'role'

export type Row = { readonly task: string; readonly kind: RowKind; readonly cells: ReadonlyMap<string, string> }

export type Breakdown = { readonly headers: readonly string[]; readonly rows: readonly Row[] }

/** The corrected block `decision-maker-2.md`, or else the highest-numbered decision-maker file. */
export function decisionBlock(job: string): Block {
  const folder = join(PLANS, job)
  if (!existsSync(folder)) throw new ScriptError(`plan folder ${folder} does not exist`)
  const corrected = 'decision-maker-2.md'
  let file = corrected
  let note = 'corrected block'
  if (!existsSync(join(folder, corrected))) {
    const numbered = readdirSync(folder)
      .map((name) => /^decision-maker(?:-(\d+))?\.md$/.exec(name))
      .filter((found): found is RegExpExecArray => found !== null)
      .sort((left, right) => Number(right[1] ?? '1') - Number(left[1] ?? '1'))
    const best = numbered[0]
    if (best === undefined) throw new ScriptError(`plan folder ${folder} holds no decision-maker file`)
    file = best[0]
    note = `no ${corrected}; used the highest-numbered file`
  }
  const text = readFileSync(join(folder, file), 'utf8')
  return { job, file, sha256: sha256(text), text, note }
}

/** Splits one table line into cells at each `|` that is outside a code span and not escaped. Cells keep their text as written. */
function splitCells(line: string, respectCode: boolean): string[] {
  const cells: string[] = []
  let current = ''
  let openRun = 0
  for (let at = 0; at < line.length; at += 1) {
    const char = line[at] ?? ''
    if (char === '\\' && line[at + 1] === '|') {
      current += '\\|'
      at += 1
      continue
    }
    if (char === '`' && respectCode) {
      let run = 1
      while (line[at + run] === '`') run += 1
      if (openRun === 0) openRun = run
      else if (openRun === run) openRun = 0
      current += '`'.repeat(run)
      at += run - 1
      continue
    }
    if (char === '|' && openRun === 0) {
      cells.push(current.trim())
      current = ''
      continue
    }
    current += char
  }
  cells.push(current.trim())
  if (cells[0] === '') cells.shift()
  if (cells[cells.length - 1] === '') cells.pop()
  return cells
}

function cellsOf(line: string, width: number, where: string): string[] {
  const withCode = splitCells(line, true)
  if (withCode.length === width) return withCode
  const plain = splitCells(line, false)
  if (plain.length === width) return plain
  throw new ScriptError(`${where}: the row has ${withCode.length} cells (or ${plain.length} when code spans are ignored), but the header has ${width}`)
}

/** The first table after the `WORK BREAKDOWN:` line whose first column is `Task`. */
export function readBreakdown(block: Block): Breakdown {
  const lines = block.text.split('\n')
  const start = lines.findIndex((line) => line.trim().startsWith('WORK BREAKDOWN'))
  if (start < 0) throw new ScriptError(`${block.job}/${block.file}: no WORK BREAKDOWN line`)
  const headerAt = lines.findIndex((line, at) => at > start && /^\|\s*Task\s*\|/i.test(line.trim()))
  if (headerAt < 0) throw new ScriptError(`${block.job}/${block.file}: no table with a Task column after WORK BREAKDOWN`)
  const headers = splitCells(lines[headerAt]?.trim() ?? '', false)
  const separator = lines[headerAt + 1]?.trim() ?? ''
  if (!/^\|[\s:|-]+\|$/.test(separator)) throw new ScriptError(`${block.job}/${block.file}: the Task table has no separator line`)
  const rows: Row[] = []
  for (let at = headerAt + 2; at < lines.length; at += 1) {
    const line = lines[at]?.trim() ?? ''
    if (!line.startsWith('|')) break
    const where = `${block.job}/${block.file} line ${at + 1}`
    const cells = cellsOf(line, headers.length, where)
    const keyed = new Map(headers.map((header, column) => [header.toLowerCase(), cells[column] ?? '']))
    const task = cells[0] ?? ''
    const isTask = /^T\d+[a-z]?$/.test(task)
    if (!isTask && !/^[a-z][a-z0-9-]*$/.test(task)) {
      throw new ScriptError(`${where}: first cell is neither a task ID of the form T<number> nor a role name in lower case`)
    }
    rows.push({ task, kind: isTask ? 'task' : 'role', cells: keyed })
  }
  if (rows.length === 0) throw new ScriptError(`${block.job}/${block.file}: the Task table has no rows`)
  return { headers, rows }
}

/** The cell of the first column whose name starts with `prefix`, compared in lower case. */
export function column(breakdown: Breakdown, row: Row, prefix: string): string {
  const header = breakdown.headers.map((name) => name.toLowerCase()).find((name) => name.startsWith(prefix))
  if (header === undefined) throw new ScriptError(`no column starts with "${prefix}"; columns: ${breakdown.headers.join(', ')}`)
  return row.cells.get(header) ?? ''
}

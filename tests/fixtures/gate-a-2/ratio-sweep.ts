// Searches every form of a filled folder at seven keyword ratios with --keyword-scores, writes name-only results per ratio, and counts the tier C candidates of each need.
// It also checks that --keyword-ratio 0.5 gives the same answer as no option. Usage: node tests/fixtures/gate-a-2/ratio-sweep.ts --forms <filled folder> --out <folder> --label <set> [--pool <folder>] [--raw <folder>] [--limit 10]
import { writeFileSync } from 'node:fs'
import { basename, join } from 'node:path'

import {
  DEFAULT_POOL,
  DEFAULT_RAW,
  ensureFolder,
  flag,
  formId,
  isObject,
  type Item,
  type Json,
  jsonFiles,
  LIBRARY,
  libraryName,
  main,
  pages,
  readAnswer,
  readFlags,
  readJson,
  REX,
  requiredFlag,
  run,
  say,
  search,
  shown,
  text,
  textList,
  wholeNumber,
  writeJson,
} from './lib.ts'
import { nameOnly } from './search-lib.ts'

export const RATIOS = ['0.1', '0.25', '0.4', '0.5', '0.6', '0.75', '1.0']
/** The fields of a form whose unrecognised words search sends to keyword search, as it does free-text needs. */
const WORD_FIELDS = ['must_have', 'artefacts', 'activity', 'touches']
/** The cap on keyword candidates of one need, which the binary applies after the ratio. */
const CAP = 5

/** The need a tier C item was found for, from the `why` that the binary writes: `keyword: need <n>` or `keyword: <field>:<word>`. */
function needOf(item: Item): string {
  const why = text(item.raw['why']) ?? ''
  return why.startsWith('keyword: ') ? why.slice('keyword: '.length) : `(why ${JSON.stringify(why)})`
}

/** The needs that keyword search gets for a form: each free-text need, then each unrecognised word of a term field. */
function needsOf(form: Json, unrecognised: readonly string[]): string[] {
  const needs = isObject(form) ? textList(form['needs']) : []
  const words = unrecognised.filter((word) => WORD_FIELDS.some((field) => word.startsWith(`${field}:`)))
  return [...needs.map((_, at) => `need ${at + 1}`), ...words]
}

/** The ratio that `--help` names as built in, so the output says which value an answer with no option uses. */
function builtInRatio(): string {
  const help = run(REX, ['skills', 'search', '--help']).stdout
  return /in place of the built-in ([0-9.]+)/.exec(help)?.[1] ?? 'not named by --help'
}

type Tally = { needs: number; candidates: number; pool: number; atCap: number; none: number; perNeed: Map<number, number> }

main(() => {
  const flags = readFlags(process.argv.slice(2), ['forms', 'out', 'label', 'pool', 'raw', 'limit'])
  const forms = requiredFlag(flags, 'forms')
  const out = requiredFlag(flags, 'out')
  const label = requiredFlag(flags, 'label')
  const pool = flag(flags, 'pool', DEFAULT_POOL)
  const raw = flag(flags, 'raw', DEFAULT_RAW)
  const limit = wholeNumber(flags, 'limit', '10')
  const libraries = [LIBRARY, pool]
  const poolName = libraryName(pool)
  say('w0', `libraries ${libraries.map(shown).join(' then ')}; limit ${limit}; every page; options --keyword-scores and --keyword-ratio ${RATIOS.join(', ')}; built-in ratio named by --help: ${builtInRatio()}; a candidate is a tier C item, counted under the need its why names`)
  const files = jsonFiles(forms)
  let sameAsDefault = 0
  let sameWithScores = 0
  for (const ratio of RATIOS) {
    const folder = join(out, ratio)
    const rawFolder = join(raw, `${label}-ratio-${ratio}`)
    ensureFolder(folder)
    ensureFolder(rawFolder)
    const tally: Tally = { needs: 0, candidates: 0, pool: 0, atCap: 0, none: 0, perNeed: new Map() }
    for (const file of files) {
      const form = readJson(file)
      const id = formId(form, file)
      const answer = pages(libraries, form, { limit }, { ratio, scores: true })
      writeFileSync(join(rawFolder, basename(file)), `${answer.lines.join('\n')}\n`)
      const first = readAnswer(answer.lines[0] ?? '')
      const result = first.results[0]
      if (result === undefined) continue
      const later = answer.items.slice(result.items.length).map((item) => item.raw)
      writeJson(join(folder, basename(file)), nameOnly(id, first.indexVersion, result.raw, later, poolName))
      const keyword = answer.items.filter((item) => item.tier === 'C')
      for (const need of needsOf(form, textList(result.raw['unrecognised']))) {
        const count = keyword.filter((item) => needOf(item) === need).length
        tally.needs += 1
        tally.perNeed.set(count, (tally.perNeed.get(count) ?? 0) + 1)
        if (count >= CAP) tally.atCap += 1
        if (count === 0) tally.none += 1
      }
      tally.candidates += keyword.length
      tally.pool += keyword.filter((item) => item.library === poolName).length
      if (ratio === '0.5') {
        const withScores = pages(libraries, form, { limit }, { scores: true }).lines.join('\n')
        if (withScores === answer.lines.join('\n')) sameWithScores += 1
        else say('w2', `${id}: every page with --keyword-ratio 0.5 --keyword-scores DIFFERS from --keyword-scores alone`)
        const plain = search(libraries, { requests: [form], limit })
        if (plain === search(libraries, { requests: [form], limit }, { ratio })) sameAsDefault += 1
        else say('w2', `${id}: the first page with --keyword-ratio 0.5 DIFFERS from no option`)
      }
    }
    const perNeed = [...tally.perNeed.entries()].sort(([left], [right]) => left - right).map(([count, needs]) => `${count}: ${needs}`).join(', ')
    const share = tally.candidates === 0 ? 'none' : `${((100 * tally.pool) / tally.candidates).toFixed(1)}%`
    const average = tally.needs === 0 ? '0' : (tally.candidates / tally.needs).toFixed(2)
    say('w1', `ratio ${ratio}: ${files.length} forms, ${tally.needs} keyword needs, ${tally.candidates} tier C candidates over all pages (${average} per need); ${tally.pool} of them pool skills (${share}); needs at the cap of ${CAP}: ${tally.atCap}; needs with no candidate: ${tally.none}; needs by candidate count {${perNeed}}; results in ${shown(folder)}`)
  }
  say('w3', `--keyword-ratio 0.5 gives the same first page as no option for ${sameAsDefault} of ${files.length} forms, and the same pages as --keyword-scores alone for ${sameWithScores} of ${files.length}`)
  if (sameAsDefault !== files.length || sameWithScores !== files.length) process.exitCode = 1
})

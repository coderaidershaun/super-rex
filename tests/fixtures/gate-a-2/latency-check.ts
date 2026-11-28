// Checks the latency example of spec 8.3 on the two hand-written forms: the Rust matching loop gets rex-code-latency, and the Go service in milliseconds gets it only under gaps with its not_when line.
// Usage: node tests/fixtures/gate-a-2/latency-check.ts [--pool <folder>] [--time <runs>] — with --time it also prints the median wall time of one search call.
import { allPages, DEFAULT_POOL, flag, isArray, isObject, type Json, jsonFiles, LATENCY_FORMS, LIBRARY, main, readAnswer, readFlags, readJson, REX, run, say, ScriptError, search, shown, text } from './lib.ts'

const SKILL = 'rex-code-latency'
const LIMIT = 10

/** The not_when line of the skill's own header, read from the binary's static index. */
function headerNotWhen(): string {
  const index = JSON.parse(run(REX, ['skills', 'index', '--library', LIBRARY]).stdout) as Json
  const skills = isObject(index) && isArray(index['skills']) ? index['skills'] : []
  const skill = skills.find((entry) => isObject(entry) && entry['name'] === SKILL)
  const metadata = isObject(skill) ? skill['metadata'] : undefined
  const notWhen = isObject(metadata) ? text(metadata['not_when']) : undefined
  if (notWhen === undefined) throw new ScriptError(`${SKILL} has no not_when line in the static index`)
  return notWhen
}

function median(values: readonly number[]): number {
  const sorted = [...values].sort((left, right) => left - right)
  const middle = Math.floor(sorted.length / 2)
  return sorted.length % 2 === 1 ? (sorted[middle] ?? 0) : ((sorted[middle - 1] ?? 0) + (sorted[middle] ?? 0)) / 2
}

function shellQuote(word: string): string {
  return `'${word.replace(/'/g, `'\\''`)}'`
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['pool', 'time'])
  const libraries = [LIBRARY, flag(flags, 'pool', DEFAULT_POOL)]
  const runs = Number(flag(flags, 'time', '0'))
  if (!Number.isInteger(runs) || runs < 0) throw new ScriptError('--time must be a whole number of runs')
  const notWhen = headerNotWhen()
  say('l0', `libraries ${libraries.map(shown).join(' then ')}; limit ${LIMIT}; header not_when of ${SKILL}: ${JSON.stringify(notWhen)}`)
  let passed = 0
  const forms = jsonFiles(LATENCY_FORMS)
  for (const file of forms) {
    const form = readJson(file)
    const id = isObject(form) ? (text(form['id']) ?? file) : file
    const languages = isObject(form) && isObject(form['facts']) ? form['facts']['languages'] : undefined
    const isRust = isArray(languages) && languages.includes('rust')
    const result = readAnswer(search(libraries, { requests: [form], limit: LIMIT })).results[0]
    if (result === undefined) throw new ScriptError(`the answer for ${id} holds no result`)
    const onPage = result.items.find((item) => item.skill === SKILL)
    const anyPage = allPages(libraries, form, LIMIT).find((item) => item.skill === SKILL)
    const gaps = isArray(result.raw['gaps']) ? result.raw['gaps'] : []
    const gap = gaps.find((entry) => isObject(entry) && entry['skill'] === SKILL)
    if (isRust) {
      const ok = onPage !== undefined && onPage.tier === 'B' && onPage.slot === 'quality:latency' && gap === undefined
      say('l1', `${id}: ${SKILL} ${onPage === undefined ? 'NOT on the first page' : `at rank ${onPage.rank}, tier ${onPage.tier}, slot ${onPage.slot}, why ${JSON.stringify(text(onPage.raw['why']))}`}; in gaps: ${gap === undefined ? 'no' : 'YES'}; ${ok ? 'pass' : 'FAIL'}`)
      if (ok) passed += 1
    } else {
      const gapNotWhen = isObject(gap) ? text(gap['not_when']) : undefined
      const ok = anyPage === undefined && gapNotWhen === notWhen
      const reason = isObject(gap) ? text(gap['reason']) : undefined
      say('l1', `${id}: ${SKILL} on any page: ${anyPage === undefined ? 'no' : `YES, rank ${anyPage.rank}`}; gap: ${gap === undefined ? 'none' : `need ${JSON.stringify(isObject(gap) ? text(gap['need']) : '')}, reason ${JSON.stringify(reason)}, not_when ${JSON.stringify(gapNotWhen)}`}; not_when equals the header: ${gapNotWhen === notWhen ? 'yes' : 'NO'}; ${ok ? 'pass' : 'FAIL'}`)
      if (ok) passed += 1
    }
    if (runs > 0) {
      const request = JSON.stringify({ requests: [form], limit: LIMIT })
      const times: number[] = []
      for (let at = 0; at < runs; at += 1) {
        const start = performance.now()
        search(libraries, { requests: [form], limit: LIMIT })
        times.push(performance.now() - start)
      }
      const command = `printf '%s' ${shellQuote(request)} | ${REX} skills search ${libraries.map((folder) => `--library ${folder}`).join(' ')}`
      say('l3', `${id}: median ${median(times).toFixed(0)} ms over ${runs} runs (min ${Math.min(...times).toFixed(0)}, max ${Math.max(...times).toFixed(0)}), wall time of one process from start to exit; command: ${command}`)
    }
  }
  say('l2', `${passed} of ${forms.length} latency forms behave as spec 8.3 says`)
  if (passed !== forms.length) process.exitCode = 1
})

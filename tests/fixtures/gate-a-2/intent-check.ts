// Runs the intent check of section 10 on the library among the filler pool, then searches each intent phrase as a free-text need to show the rank of its own skill.
// Pool skills are named only. Usage: node tests/fixtures/gate-a-2/intent-check.ts [--pool <folder>]
import { DEFAULT_POOL, flag, isArray, isObject, type Json, LIBRARY, main, readAnswer, readFlags, REX, run, say, ScriptError, search, shown, text, textList } from './lib.ts'

const TOP = 5

function intents(): { skill: string; phrase: string }[] {
  const index = JSON.parse(run(REX, ['skills', 'index', '--library', LIBRARY]).stdout) as Json
  if (!isObject(index) || !isArray(index['skills'])) throw new ScriptError('the static index has no `skills` list')
  return index['skills'].flatMap((skill) => {
    if (!isObject(skill)) return []
    const metadata = skill['metadata']
    const name = text(skill['name']) ?? ''
    return isObject(metadata) ? textList(metadata['intents']).map((phrase) => ({ skill: name, phrase })) : []
  })
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['pool'])
  const pool = flag(flags, 'pool', DEFAULT_POOL)
  const args = ['skills', 'check', '--pool', pool, LIBRARY]
  const check = run(REX, args)
  const named = ['skills', 'check', '--pool', shown(pool), shown(LIBRARY)]
  const lines = `${check.stdout}${check.stderr}`.split('\n').filter((line) => line !== '')
  const okLines = lines.filter((line) => line.startsWith('ok ')).length
  const findings = lines.filter((line) => line.includes(': M14: '))
  say('e0', `rex ${named.join(' ')}: exit ${check.status}, ${lines.length} lines, ${okLines} ok, ${findings.length} intent findings (M14), ${lines.length - okLines - findings.length} other lines`)
  lines.filter((line) => !line.startsWith('ok ')).forEach((line) => say('e0', line))
  const all = intents()
  say('e1', `${all.length} intent phrases in ${new Set(all.map((entry) => entry.skill)).size} skills; each searched as one need over the library then the pool`)
  let inTop = 0
  for (const { skill, phrase } of all) {
    const form = { id: 'intent', needs: [phrase] }
    const result = readAnswer(search([LIBRARY, pool], { requests: [form], limit: 10 })).results[0]
    const found = (result?.items ?? []).filter((item) => item.tier === 'C')
    const place = found.findIndex((item) => item.skill === skill && item.library === 'library')
    if (place >= 0 && place < TOP) {
      inTop += 1
      say('e2', `${skill}: "${phrase}": rank ${place + 1} of ${found.length} tier C items`)
      continue
    }
    const above = found.slice(0, place >= 0 ? place : TOP).map((item) => `${item.library}/${item.skill}`)
    say('e2', `${skill}: "${phrase}": NOT in the top ${TOP} (${place >= 0 ? `rank ${place + 1}` : 'not returned'}); above it: ${above.join(' ') || 'nothing'}`)
  }
  say('e3', `${inTop} of ${all.length} intent phrases find their own skill in the top ${TOP} tier C items of the search answer`)
})

// Checks that adding the filler pool changes no tier A or tier B item: the same items, in the same order, with the library alone and with the pool after it.
// It follows every page by cursor, because pool items of tier C can push an A or B item to a later page. Usage: node tests/fixtures/gate-a/pool-invariance.ts --forms <file or folder> [--pool <folder>] [--limit 10]
import { allPages, DEFAULT_POOL, flag, isObject, type Item, type Json, jsonFiles, LIBRARY, main, readAnswer, readFlags, readJson, requiredFlag, say, ScriptError, search, text } from './lib.ts'

function firstPage(libraries: readonly string[], form: Json, limit: number): readonly Item[] {
  return readAnswer(search(libraries, { requests: [form], limit })).results[0]?.items ?? []
}

function declared(items: readonly Item[]): string[] {
  return items.filter((item) => item.tier === 'A' || item.tier === 'B').map((item) => `${item.tier}:${item.library}/${item.skill}`)
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['forms', 'pool', 'limit'])
  const forms = requiredFlag(flags, 'forms')
  const pool = flag(flags, 'pool', DEFAULT_POOL)
  const limit = Number(flag(flags, 'limit', '10'))
  if (!Number.isInteger(limit) || limit < 1) throw new ScriptError('--limit must be a whole number of 1 or more')
  say('p0', `library ${LIBRARY} alone, then with pool ${pool}; limit ${limit}; all pages followed`)
  let same = 0
  let firstPageSame = 0
  let total = 0
  for (const file of jsonFiles(forms)) {
    const form = readJson(file)
    const id = isObject(form) ? (text(form['id']) ?? file) : file
    const alone = allPages([LIBRARY], form, limit)
    const pooled = allPages([LIBRARY, pool], form, limit)
    const aloneDeclared = declared(alone)
    const pooledDeclared = declared(pooled)
    const isSame = aloneDeclared.join(' ') === pooledDeclared.join(' ')
    const firstAlone = declared(firstPage([LIBRARY], form, limit)).join(' ')
    const firstPooled = declared(firstPage([LIBRARY, pool], form, limit)).join(' ')
    total += 1
    if (isSame) same += 1
    if (firstAlone === firstPooled) firstPageSame += 1
    const pooledC = pooled.filter((item) => item.tier === 'C').length
    say('p1', `${id}: tier A and B ${isSame ? 'same' : 'DIFFERENT'} (${aloneDeclared.length} alone, ${pooledDeclared.length} with pool; ${pooledC} tier C with pool); first page ${firstAlone === firstPooled ? 'same' : 'differs'}`)
    if (!isSame) say('p1', `${id}: alone [${aloneDeclared.join(' ')}] with pool [${pooledDeclared.join(' ')}]`)
    if (firstAlone !== firstPooled) say('p1', `${id}: first page alone [${firstAlone}] with pool [${firstPooled}]`)
  }
  say('p2', `${same} of ${total} forms keep the same tier A and B items in the same order over all pages; ${firstPageSame} of ${total} also on the first page`)
  if (same !== total) process.exitCode = 1
})

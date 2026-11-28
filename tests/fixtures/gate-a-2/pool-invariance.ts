// Checks that adding the filler pool changes no tier A or tier B item: the same items in the same order, and the same rank for each of the first five of them.
// It follows every page by cursor and also compares the first page. Usage: node tests/fixtures/gate-a-2/pool-invariance.ts --forms <file or folder> [--pool <folder>] [--limit 10]
import { allPages, declared, DEFAULT_POOL, firstResult, flag, formId, type Item, type Json, jsonFiles, LIBRARY, main, readFlags, readJson, requiredFlag, say, search, shown, wholeNumber } from './lib.ts'

/** How many tier A and B items the rank check covers: the top five that the hit rate counts. */
const RANKED = 5

function firstPage(libraries: readonly string[], form: Json, limit: number): readonly Item[] {
  return firstResult(search(libraries, { requests: [form], limit }), 'the first page').items
}

/** `rank tier:library/skill` of the first five tier A and B items. The rank is the place over all tiers. */
function rankedHead(items: readonly Item[]): string {
  const head = items.filter((item) => item.tier === 'A' || item.tier === 'B').slice(0, RANKED)
  return head.map((item) => `${item.rank} ${item.tier}:${item.library}/${item.skill}`).join(', ')
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['forms', 'pool', 'limit'])
  const forms = requiredFlag(flags, 'forms')
  const pool = flag(flags, 'pool', DEFAULT_POOL)
  const limit = wholeNumber(flags, 'limit', '10')
  say('p0', `library ${shown(LIBRARY)} alone, then with pool ${shown(pool)}; limit ${limit}; all pages followed`)
  let same = 0
  let firstPageSame = 0
  let ranksSame = 0
  let firstRanksSame = 0
  let total = 0
  for (const file of jsonFiles(forms)) {
    const form = readJson(file)
    const id = formId(form, file)
    const alone = allPages([LIBRARY], form, limit)
    const pooled = allPages([LIBRARY, pool], form, limit)
    const aloneDeclared = declared(alone)
    const pooledDeclared = declared(pooled)
    const isSame = aloneDeclared.join(' ') === pooledDeclared.join(' ')
    const firstAlone = firstPage([LIBRARY], form, limit)
    const firstPooled = firstPage([LIBRARY, pool], form, limit)
    const isFirstSame = declared(firstAlone).join(' ') === declared(firstPooled).join(' ')
    const isRankSame = rankedHead(alone) === rankedHead(pooled)
    const isFirstRankSame = rankedHead(firstAlone) === rankedHead(firstPooled)
    total += 1
    if (isSame) same += 1
    if (isFirstSame) firstPageSame += 1
    if (isRankSame) ranksSame += 1
    if (isFirstRankSame) firstRanksSame += 1
    const pooledC = pooled.filter((item) => item.tier === 'C').length
    say('p1', `${id}: tier A and B ${isSame ? 'same' : 'DIFFERENT'} (${aloneDeclared.length} alone, ${pooledDeclared.length} with pool; ${pooledC} tier C with pool); first page ${isFirstSame ? 'same' : 'differs'}; ranks of the first ${RANKED} ${isRankSame ? 'same' : 'DIFFERENT'} over all pages, ${isFirstRankSame ? 'same' : 'DIFFERENT'} on the first page`)
    if (!isSame) say('p1', `${id}: alone [${aloneDeclared.join(' ')}] with pool [${pooledDeclared.join(' ')}]`)
    if (!isFirstSame) say('p1', `${id}: first page alone [${declared(firstAlone).join(' ')}] with pool [${declared(firstPooled).join(' ')}]`)
    if (!isRankSame) say('p1', `${id}: ranks alone [${rankedHead(alone)}] with pool [${rankedHead(pooled)}]`)
    if (!isFirstRankSame) say('p1', `${id}: first-page ranks alone [${rankedHead(firstAlone)}] with pool [${rankedHead(firstPooled)}]`)
  }
  say('p2', `${same} of ${total} forms keep the same tier A and B items in the same order over all pages; ${firstPageSame} of ${total} also on the first page`)
  say('p3', `${ranksSame} of ${total} forms keep the same rank for each of their first ${RANKED} tier A and B items over all pages; ${firstRanksSame} of ${total} on the first page`)
  if (same !== total || ranksSame !== total) process.exitCode = 1
})

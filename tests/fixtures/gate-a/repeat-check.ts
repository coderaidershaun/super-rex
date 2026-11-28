// Checks that the same form, searched twice, gives a byte-identical answer: spec 8.3 asks lookups to be repeatable and WP4 asks the same input to give the same output.
// Usage: node tests/fixtures/gate-a/repeat-check.ts --forms <file or folder> [--pool <folder>] [--limit 10] [--library-only yes]
import { DEFAULT_POOL, flag, isObject, jsonFiles, LIBRARY, main, readFlags, readJson, requiredFlag, say, ScriptError, search, sha256, text } from './lib.ts'

main(() => {
  const flags = readFlags(process.argv.slice(2), ['forms', 'pool', 'limit', 'library-only'])
  const forms = requiredFlag(flags, 'forms')
  const limit = Number(flag(flags, 'limit', '10'))
  if (!Number.isInteger(limit) || limit < 1) throw new ScriptError('--limit must be a whole number of 1 or more')
  const libraries = flag(flags, 'library-only', 'no') === 'yes' ? [LIBRARY] : [LIBRARY, flag(flags, 'pool', DEFAULT_POOL)]
  say('r0', `libraries: ${libraries.join(' then ')}; limit ${limit}; each form searched twice`)
  let same = 0
  let total = 0
  for (const file of jsonFiles(forms)) {
    const form = readJson(file)
    const id = isObject(form) ? (text(form['id']) ?? file) : file
    const first = search(libraries, { requests: [form], limit })
    const second = search(libraries, { requests: [form], limit })
    const isSame = first === second
    total += 1
    if (isSame) same += 1
    say('r1', `${id}: ${isSame ? 'byte-identical' : 'DIFFERENT'}, ${Buffer.byteLength(first)} bytes, sha256 ${sha256(first)}`)
  }
  say('r2', `${same} of ${total} forms gave byte-identical answers on two runs`)
  if (same !== total) process.exitCode = 1
})

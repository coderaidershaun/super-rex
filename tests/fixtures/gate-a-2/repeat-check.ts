// Checks that the same form, searched twice, gives a byte-identical answer: spec 8.3 asks lookups to be repeatable and WP4 asks the same input to give the same output.
// Usage: node tests/fixtures/gate-a-2/repeat-check.ts --forms <file or folder> [--pool <folder>] [--library-only yes] [the search flags of search-lib.ts]
import { DEFAULT_POOL, flag, formId, jsonFiles, LIBRARY, main, readFlags, readJson, requiredFlag, say, search, settingArgs, sha256, shown, yesNo } from './lib.ts'
import { searchSettings, SETTING_FLAGS } from './search-lib.ts'

main(() => {
  const flags = readFlags(process.argv.slice(2), ['forms', 'pool', 'library-only', ...SETTING_FLAGS])
  const forms = requiredFlag(flags, 'forms')
  const settings = searchSettings(flags)
  const libraries = yesNo(flags, 'library-only') ? [LIBRARY] : [LIBRARY, flag(flags, 'pool', DEFAULT_POOL)]
  say('r0', `libraries: ${libraries.map(shown).join(' then ')}; request ${JSON.stringify(settings.request)}; options ${settingArgs(settings.keyword).join(' ') || 'none'}; each form searched twice`)
  let same = 0
  let total = 0
  for (const file of jsonFiles(forms)) {
    const form = readJson(file)
    const request = { ...settings.request, requests: [form] }
    const first = search(libraries, request, settings.keyword)
    const second = search(libraries, request, settings.keyword)
    const isSame = first === second
    total += 1
    if (isSame) same += 1
    say('r1', `${formId(form, file)}: ${isSame ? 'byte-identical' : 'DIFFERENT'}, ${Buffer.byteLength(first)} bytes, sha256 ${sha256(first)}`)
  }
  say('r2', `${same} of ${total} forms gave byte-identical answers on two runs`)
  if (same !== total) process.exitCode = 1
})

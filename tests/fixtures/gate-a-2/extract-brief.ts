// Writes the first user message of a sub-agent transcript to a file, byte for byte, so the brief a form writer got can be committed and compared.
// Usage: node tests/fixtures/gate-a-2/extract-brief.ts --transcript <jsonl> --out <file> [--compare <file>]
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { basename } from 'node:path'

import { flag, main, readFlags, requiredFlag, say, ScriptError, sha256, shown } from './lib.ts'
import { firstUserMessage } from './transcript-lib.ts'

main(() => {
  const flags = readFlags(process.argv.slice(2), ['transcript', 'out', 'compare'])
  const transcript = requiredFlag(flags, 'transcript')
  const out = requiredFlag(flags, 'out')
  const brief = firstUserMessage(transcript)
  writeFileSync(out, brief)
  say('b1', `${basename(transcript)}: first user message written to ${basename(out)}, ${Buffer.byteLength(brief)} bytes, sha256 ${sha256(brief)}`)
  const compare = flag(flags, 'compare', '')
  if (compare === '') return
  if (!existsSync(compare)) throw new ScriptError(`file ${compare} does not exist`)
  const committed = readFileSync(compare, 'utf8')
  const isSame = committed === brief
  say('b2', `${shown(compare)}: ${Buffer.byteLength(committed)} bytes, sha256 ${sha256(committed)}; ${isSame ? 'the same bytes as the extract' : 'DIFFERENT from the extract'}`)
  if (!isSame) process.exitCode = 1
})

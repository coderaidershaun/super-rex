// Checks whether the binary lets a caller set KEYWORD_SCORE_RATIO or see the score of a tier C item, which part 5a of gate A needs to tune the ratio from the binary itself.
// Usage: node tests/fixtures/gate-a/ratio-check.ts
import { join } from 'node:path'

import { HERE, isArray, isObject, type Json, jsonFiles, JOB_DIR, LIBRARY, main, readJson, REX, run, say, search } from './lib.ts'

function grep(pattern: string, paths: readonly string[]): string[] {
  const found = run('git', ['-C', JOB_DIR, 'grep', '-n', '-E', pattern, '--', ...paths])
  return found.stdout.split('\n').filter((line) => line !== '')
}

function keysOf(value: Json, into: Set<string>): Set<string> {
  if (isArray(value)) value.forEach((entry) => keysOf(entry, into))
  else if (isObject(value))
    for (const [key, entry] of Object.entries(value)) {
      into.add(key)
      keysOf(entry, into)
    }
  return into
}

main(() => {
  const uses = grep('KEYWORD_SCORE_RATIO', ['src'])
  uses.forEach((line) => say('g0', `source: ${line}`))
  say('g0', `${uses.length} source lines name KEYWORD_SCORE_RATIO`)
  const envReads = grep('env::var|std::env', ['src/commands/skills'])
  envReads.forEach((line) => say('g1', `environment read in skills code: ${line}`))
  say('g1', `${envReads.length} environment reads in src/commands/skills; ${envReads.filter((line) => /RATIO|SCORE/i.test(line)).length} of them name a ratio or score`)
  let helpHits = 0
  for (const command of ['search', 'serve', 'check', 'index']) {
    const help = run(REX, ['skills', command, '--help'])
    const lines = `${help.stdout}${help.stderr}`.split('\n').filter((line) => /ratio|score|relevance/i.test(line))
    helpHits += lines.length
    say('g2', `rex skills ${command} --help: ${lines.length} lines name a ratio, score or relevance`)
  }
  const keys = new Set<string>()
  for (const file of jsonFiles(join(HERE, 'latency'))) keysOf(JSON.parse(search([LIBRARY], { requests: [readJson(file)], limit: 10 })) as Json, keys)
  const scoreKeys = [...keys].filter((key) => /score|relevance|rank|bm25/i.test(key))
  say('g3', `answer fields of the two latency answers: ${[...keys].sort().join(' ')}; fields that carry a score: ${scoreKeys.length}`)
  const canSet = envReads.some((line) => /RATIO|SCORE/i.test(line)) || helpHits > 0
  const canSee = scoreKeys.length > 0
  say('g4', `ratio settable from outside the code: ${canSet ? 'yes' : 'no'}; score of a tier C item in the output: ${canSee ? 'yes' : 'no'}`)
  if (!canSet && !canSee) {
    say('g4', 'part 5a: not run for the binary. It lacks a --keyword-score-ratio option or environment setting for KEYWORD_SCORE_RATIO, and it lacks the relevance of each tier C item in the answer (the candidate holds it; the item does not)')
  }
})

// Prints the median wall time of 10 runs of one search process over the library alone, for each latency form: the measure of latency-check.ts --time, without the pool.
// Usage: node tests/fixtures/gate-a/results/time-library-only.ts
import { join } from 'node:path'

import { HERE, isObject, jsonFiles, LIBRARY, main, readJson, run, REX, say, ScriptError, text } from '../lib.ts'

const RUNS = 10

main(() => {
  for (const file of jsonFiles(join(HERE, 'latency'))) {
    const form = readJson(file)
    const id = isObject(form) ? (text(form['id']) ?? file) : file
    const input = JSON.stringify({ requests: [form], limit: 10 })
    const times: number[] = []
    for (let at = 0; at < RUNS; at += 1) {
      const start = performance.now()
      const result = run(REX, ['skills', 'search', '--library', LIBRARY], input)
      times.push(performance.now() - start)
      if (result.status !== 0) throw new ScriptError(`rex skills search exited ${result.status}: ${result.stderr.trim()}`)
    }
    times.sort((left, right) => left - right)
    const median = ((times[RUNS / 2 - 1] ?? 0) + (times[RUNS / 2] ?? 0)) / 2
    say('t1', `${id}: library alone, median ${median.toFixed(0)} ms over ${RUNS} runs (min ${(times[0] ?? 0).toFixed(0)}, max ${(times[RUNS - 1] ?? 0).toFixed(0)}), wall time of one process from start to exit`)
  }
})

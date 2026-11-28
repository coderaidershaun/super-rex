// Reruns the measures of gate A that need no labels, steps (e) to (h), in order, so one command repeats them and gives the same output each time.
// Timing is left out because wall time varies; latency-check.ts --time 10 gives it. Usage: node tests/fixtures/gate-a/steps-e-h.ts
import { join } from 'node:path'

import { HERE, main, run, say } from './lib.ts'

const LATENCY = join(HERE, 'latency')

const STEPS: readonly (readonly [string, readonly string[]])[] = [
  ['intent-check.ts', []],
  ['latency-check.ts', []],
  ['repeat-check.ts', ['--forms', LATENCY]],
  ['repeat-check.ts', ['--forms', LATENCY, '--library-only', 'yes']],
  ['pool-invariance.ts', ['--forms', LATENCY]],
  ['ratio-check.ts', []],
  ['answer-size.ts', ['--forms', LATENCY]],
  ['size-proof.ts', []],
]

main(() => {
  let failed = 0
  for (const [script, args] of STEPS) {
    const result = run('node', [join(HERE, script), ...args])
    process.stdout.write(`== node ${script} ${args.join(' ')} (exit ${result.status})\n${result.stdout}${result.stderr}`)
    if (result.status !== 0) failed += 1
  }
  say('x', `${STEPS.length} steps run, ${failed} failed`)
  if (failed > 0) process.exitCode = 1
})

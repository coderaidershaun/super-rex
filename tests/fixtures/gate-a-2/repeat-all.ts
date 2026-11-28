// Remakes every output of this folder that needs no labels, in order of what each needs, and writes each script's output to its file under results/.
// A second run gives the same bytes; the wall time of latency-check.ts --time is the one output it leaves out. Usage: node tests/fixtures/gate-a-2/repeat-all.ts [--jobs 6] [--only <output file>]...
import { execFile } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { BASE_COMMIT, ensureFolder, flag, GATE_A, HERE, JOB_DIR, LATENCY_FORMS, readFlags, say, ScriptError, sha256 } from './lib.ts'

const RESULTS = join(HERE, 'results')
const POOL_OUT = '/Users/shaun/Code/DEVELOPMENT/gate-a-pool/gate-a-2'
/** The first measurement's filled forms, used unchanged so the repo facts stay those of its commit. */
const FIRST_FILLED = '/Users/shaun/Code/DEVELOPMENT/gate-a-pool/filled'
const TRANSCRIPTS = '/Users/shaun/.claude/projects/-Users-shaun-Code-scratch-skilled/90965a73-8bf5-4eb6-aef9-e8b37390eb29/subagents'
const FIRST_SCRATCH = '/private/tmp/claude-501/-Users-shaun-Code-scratch-skilled/90965a73-8bf5-4eb6-aef9-e8b37390eb29/scratchpad'
const FIRST_PLANS = '/Users/shaun/Code/DEVELOPMENT/super-rex/.rex/plans/0h0vht64cg'
/** The three old form sets and the folder of each one's forms. */
const SETS: readonly (readonly [string, string])[] = [
  ['run-1', join(GATE_A, 'forms/run-1')],
  ['run-2', join(GATE_A, 'forms/run-2')],
  ['reference', join(GATE_A, 'reference')],
]
const FIRST_RUNS: readonly (readonly [string, string])[] = [
  ['run-1', 'agent-a0a8b4c997abef5ee'],
  ['run-2', 'agent-acd4eceb203625040'],
]

type Step = { readonly output: string; readonly script: string; readonly args: readonly string[]; readonly after: readonly string[] }

function step(output: string, script: string, args: readonly string[], after: readonly string[] = []): Step {
  return { output, script, args, after }
}

function steps(): Step[] {
  const list: Step[] = [step('binary.txt', 'binary-info.ts', [])]
  for (const [run, agent] of FIRST_RUNS) {
    const transcript = join(TRANSCRIPTS, `${agent}.jsonl`)
    const brief = join(HERE, `briefs/first-form-writer-${run}.txt`)
    list.push(step(`brief-check-${run}.txt`, 'extract-brief.ts', ['--transcript', transcript, '--out', join(POOL_OUT, `tmp/first-form-writer-${run}.txt`), '--compare', brief]))
    const sealed = `/Users/shaun/Code/DEVELOPMENT/gate-a-sealed/${run}`
    const allowed = ['--allow', `${sealed}/out`, '--allow', FIRST_SCRATCH, '--allow', FIRST_PLANS]
    list.push(step(`transcript-check-first-${run}.txt`, 'transcript-check.ts', ['--transcript', transcript, '--input', `${sealed}/input`, ...allowed, '--expect', join(GATE_A, 'transcripts.txt'), '--section', run]))
  }
  for (const [set, forms] of SETS) {
    const filled = join(FIRST_FILLED, set)
    const filledNew = join(POOL_OUT, 'filled-54d74f2', set)
    list.push(step(`validate-${set}.txt`, 'validate-form.ts', [forms]))
    list.push(step(`run-search-${set}.txt`, 'run-search.ts', ['--forms', filled, '--out', join(RESULTS, set), '--label', set, '--all-pages', 'yes']))
    list.push(step(`repeat-${set}.txt`, 'repeat-check.ts', ['--forms', filled]))
    list.push(step(`pool-invariance-${set}.txt`, 'pool-invariance.ts', ['--forms', filled]))
    list.push(step(`ratio-sweep-${set}.txt`, 'ratio-sweep.ts', ['--forms', filled, '--out', join(RESULTS, `ratio-${set}`), '--label', set]))
    list.push(step(`fill-54d74f2-${set}.txt`, 'fill-facts.ts', ['--forms', forms, '--out', filledNew, '--commit', BASE_COMMIT]))
    list.push(step(`run-search-${set}-54d74f2.txt`, 'run-search.ts', ['--forms', filledNew, '--out', join(RESULTS, `${set}-54d74f2`), '--label', `${set}-54d74f2`, '--all-pages', 'yes'], [`fill-54d74f2-${set}.txt`]))
    const changeArgs = ['--old-forms', filled, '--new-forms', filledNew, '--old-results', join(RESULTS, set), '--new-results', join(RESULTS, `${set}-54d74f2`)]
    list.push(step(`facts-change-${set}.txt`, 'facts-change.ts', changeArgs, [`run-search-${set}.txt`, `run-search-${set}-54d74f2.txt`]))
  }
  const pairsOf = (suffix: string): string[] => SETS.flatMap(([set]) => ['--results', join(RESULTS, `${set}${suffix}`)])
  list.push(step('pairs.txt', 'pairs.ts', pairsOf(''), SETS.map(([set]) => `run-search-${set}.txt`)))
  list.push(step('pairs-54d74f2.txt', 'pairs.ts', pairsOf('-54d74f2'), SETS.map(([set]) => `run-search-${set}-54d74f2.txt`)))
  list.push(step('size-proof.txt', 'size-proof.ts', SETS.flatMap(([set]) => ['--set', `${set}=${join(FIRST_FILLED, set)}`])))
  list.push(step('compare-forms-run-1-run-2.txt', 'compare-forms.ts', ['--run', join(GATE_A, 'forms/run-1'), '--run', join(GATE_A, 'forms/run-2')]))
  list.push(step('intent-check.txt', 'intent-check.ts', []))
  list.push(step('latency-check.txt', 'latency-check.ts', []))
  list.push(step('repeat-latency.txt', 'repeat-check.ts', ['--forms', LATENCY_FORMS]))
  list.push(step('repeat-latency-library-only.txt', 'repeat-check.ts', ['--forms', LATENCY_FORMS, '--library-only', 'yes']))
  list.push(step('pool-invariance-latency.txt', 'pool-invariance.ts', ['--forms', LATENCY_FORMS]))
  return list
}

/** The output as committed: each line without trailing white space, which the repo's commit hook would remove. */
function tidy(stdout: string, stderr: string): string {
  const all = stderr === '' ? stdout : `${stdout}== stderr\n${stderr}`
  return `${all.split('\n').map((line) => line.trimEnd()).join('\n').replace(/\n+$/, '')}\n`
}

function runStep(entry: Step): Promise<number> {
  return new Promise((resolve) => {
    const options = { cwd: JOB_DIR, encoding: 'utf8' as const, maxBuffer: 256 * 1024 * 1024 }
    execFile('node', [join(HERE, entry.script), ...entry.args], options, (error, stdout, stderr) => {
      const code = error === null ? 0 : typeof error.code === 'number' ? error.code : -1
      writeFileSync(join(RESULTS, entry.output), tidy(stdout, stderr))
      resolve(code)
    })
  })
}

async function runAll(list: readonly Step[], jobs: number): Promise<Map<string, number>> {
  const exits = new Map<string, number>()
  const started = new Set<string>()
  const running = new Map<string, Promise<void>>()
  const isReady = (entry: Step): boolean => entry.after.every((name) => exits.has(name) || !list.some((other) => other.output === name))
  while (exits.size < list.length) {
    for (const entry of list) {
      if (running.size >= jobs) break
      if (started.has(entry.output) || !isReady(entry)) continue
      started.add(entry.output)
      running.set(entry.output, runStep(entry).then((code) => {
        exits.set(entry.output, code)
        running.delete(entry.output)
      }))
    }
    if (running.size === 0) throw new ScriptError('no step can start: a step waits for an output that never comes')
    await Promise.race(running.values())
  }
  return exits
}

async function repeatAll(): Promise<void> {
  const flags = readFlags(process.argv.slice(2), ['jobs', 'only'])
  const jobs = Number(flag(flags, 'jobs', '6'))
  if (!Number.isInteger(jobs) || jobs < 1) throw new ScriptError('--jobs must be a whole number of 1 or more')
  const only = flags.values.get('only') ?? []
  const list = steps().filter((entry) => only.length === 0 || only.includes(entry.output))
  ensureFolder(RESULTS)
  ensureFolder(join(POOL_OUT, 'tmp'))
  const exits = await runAll(list, jobs)
  const lines = list.map((entry) => `${entry.output}: node tests/fixtures/gate-a-2/${entry.script} (exit ${exits.get(entry.output) ?? -1}), sha256 ${sha256(readFileSync(join(RESULTS, entry.output), 'utf8'))}`)
  if (only.length === 0) writeFileSync(join(RESULTS, 'repeat-all.txt'), `${lines.join('\n')}\n`)
  lines.forEach((line) => say('a1', line))
  say('a2', `${list.length} steps run; the wall time of latency-check.ts --time 10 is left out, because it changes from run to run`)
}

repeatAll().catch((error: unknown) => {
  if (!(error instanceof ScriptError)) throw error
  process.stderr.write(`error: ${error.message}\n`)
  process.exitCode = 1
})

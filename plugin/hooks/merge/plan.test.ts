// Proves that rex_merge merges only a job that has a plan: a job the builder opened and never planned is refused before any git call.
import { expect, test } from 'claude-code/testing'

import type { RexBook } from '../../types'
import type { ProcessCall } from '../book/kit'
import { ROOT, START_MS, world } from '../book/kit'
import { plannedBook } from '../pipeline/assembly.fixture'
import { files, JOB, ok, seededBook } from '../tools/act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const

function mergeYes(book: RexBook): RexBook {
  return { ...book, jobs: book.jobs.map(job => ({ ...job, merge: { answer: 'yes', source: 'dialog', question: null, policyHash: null, at: START_MS } })) }
}

/** Answers every git call of a merge as one clean squash commit over the base on origin, and records each call. */
function cleanGit(calls: ProcessCall[]): (call: ProcessCall) => ReturnType<typeof ok> {
  return call => {
    calls.push(call)
    if (call.argv.includes('remote')) return ok('origin\n')
    if (call.argv.includes('for-each-ref')) return ok('refs/remotes/origin/main\n')
    if (call.argv.includes('rev-list')) return ok('1\n')
    if (call.argv.includes('rev-parse')) return ok('c1\n')
    return ok()
  }
}

test('rex_merge refuses a job that has no plan, before any git call', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  world(on, { seed: mergeYes(seededBook()), processAnswer: cleanGit(calls) })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(refused.deny).toBe(`rex_merge: job ${JOB} has no plan, got status created; record the plan with rex_job_plan, then call again`)
  expect(calls).toEqual([])
})

test('rex_merge merges the same job once it is planned', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  world(on, { seed: mergeYes(plannedBook('model-high-effort-high')), processAnswer: cleanGit(calls) })
  await $.session.start(START)

  const merged = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(merged.deny).toBeUndefined()
  expect(calls.filter(call => call.argv.includes('push')).map(call => call.argv)).toEqual([['git', '-C', ROOT, 'push', 'origin', `rex/${JOB}:refs/heads/main`]])
})

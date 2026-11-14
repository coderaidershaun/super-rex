// Fitness tests of the first Never line: rex_merge pushes only on a merge answer of yes, and only the caller's own job branch to that job's base.
import { expect, test } from 'claude-code/testing'

import type { RexBook, RexMergeAnswer } from '../../types'
import { openJobIn } from '../book/jobs'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { ROOT, START_MS, world } from '../book/kit'
import { files, JOB, ok, promptsPass, seededBook } from '../tools/act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const

/** Answers every git call of a merge as a clean one squash commit over the base, and records each call. */
function cleanGit(calls: ProcessCall[]): (call: ProcessCall) => ProcessAnswer {
  return call => {
    calls.push(call)
    if (call.argv.includes('rev-list')) return ok('1\n')
    if (call.argv.includes('rev-parse')) return ok('c1\n')
    if (call.argv[0] === 'rex') return ok('last_sequence=0\n')
    return ok()
  }
}

function pushes(calls: readonly ProcessCall[]): readonly (readonly string[])[] {
  return calls.filter(call => call.argv.includes('push')).map(call => call.argv)
}

const PERSON_OR_POLICY: readonly RexMergeAnswer['source'][] = ['dialog', 'person-prompt', 'sdk-prompt', 'question', 'policy']

for (const source of PERSON_OR_POLICY) {
  test(`no_merge_without_person_or_policy_answer: a no from ${source} pushes nothing, whatever the arguments say`, async ($, on) => {
    files(on, {})
    const calls: ProcessCall[] = []
    const seed = seededBook()
    const no: RexBook = { ...seed, jobs: seed.jobs.map(job => ({ ...job, merge: { answer: 'no', source, question: null, policyHash: null, at: START_MS } })) }
    world(on, { seed: no, processAnswer: cleanGit(calls) })
    await $.session.start(START)

    const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_merge', merge: 'yes', answer: 'yes', source: 'dialog' })

    expect(refused.deny).toMatch(new RegExp(`^rex_merge: job ${JOB} has no merge answer, got no \\(${source}\\)`))
    expect(pushes(calls)).toEqual([])
  })
}

for (const source of PERSON_OR_POLICY) {
  test(`no_merge_without_person_or_policy_answer: a yes from ${source} pushes the job branch once`, async ($, on) => {
    files(on, {})
    const calls: ProcessCall[] = []
    const seed = seededBook()
    const yes: RexBook = { ...seed, jobs: seed.jobs.map(job => ({ ...job, merge: { answer: 'yes', source, question: null, policyHash: null, at: START_MS } })) }
    world(on, { seed: yes, processAnswer: cleanGit(calls) })
    await $.session.start(START)

    await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

    expect(pushes(calls)).toEqual([['git', '-C', ROOT, 'push', 'origin', `rex/${JOB}:refs/heads/main`]])
  })
}

test('no_merge_without_person_or_policy_answer: absent, then the person says no, then yes; only the yes pushes', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const calls: ProcessCall[] = []
  world(on, { seed: seededBook(), processAnswer: cleanGit(calls) })
  await $.session.start(START)

  const absent = await $.tool.call({ tool: 'mcp__super-rex__rex_merge', merge: true, merge_answer: 'yes' })
  await $.prompt.submit({ text: 'merge: no', origin: { kind: 'composer' }, wait: false })
  const no = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })
  expect(pushes(calls)).toEqual([])
  await $.prompt.submit({ text: 'merge: yes', origin: { kind: 'composer' }, wait: false })
  const yes = await $.tool.call({ tool: 'mcp__super-rex__rex_merge' })

  expect(absent.deny).toMatch(/has no merge answer, got none;/)
  expect(no.deny).toMatch(/has no merge answer, got no \(person-prompt\);/)
  expect(yes.deny).toBeUndefined()
  expect(pushes(calls)).toEqual([['git', '-C', ROOT, 'push', 'origin', `rex/${JOB}:refs/heads/main`]])
})

test('rex_merge_pushes_only_the_callers_job_branch', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  const yes = { answer: 'yes', source: 'policy', question: null, policyHash: 'h1', at: START_MS } as const
  const first = seededBook({ builder: 'b1' })
  const input = { builder: 'b2', tree: 't1', origin: 'master', request: 'Other', repo: '/other', base: 'release', type: null, size: null, consequence: null, langs: [], prefs: { duration: null, budget: null, autonomy: null }, limit: { nano: 1, source: 'default' }, dependsOn: [], mergeAfter: [] } as const
  const both = openJobIn(first, input, '0gb0000000', START_MS).book
  const seed: RexBook = { ...both, jobs: both.jobs.map(job => ({ ...job, merge: yes })) }
  world(on, { seed, processAnswer: cleanGit(calls) })
  await $.session.start(START)

  const otherJob = await $.tool.call({ tool: 'mcp__super-rex__rex_merge', agentId: 'b1', job: '0gb0000000' })
  const extraKeys = await $.tool.call({ tool: 'mcp__super-rex__rex_merge', agentId: 'b1', branch: 'rex/0gb0000000', repo: '/other', base: 'release' })
  const notBuilder = await $.tool.call({ tool: 'mcp__super-rex__rex_merge', agentId: 'a9', job: JOB })

  expect(otherJob.deny).toBe(`rex_merge: job 0gb0000000 is not your job, got 0gb0000000; call it for ${JOB}`)
  expect(extraKeys.deny).toBeUndefined()
  expect(notBuilder.deny).toBe('rex_merge: only a builder may call it, got agent')
  expect(pushes(calls)).toEqual([['git', '-C', ROOT, 'push', 'origin', `rex/${JOB}:refs/heads/main`]])
  expect(calls.every(call => call.argv[0] !== 'git' || call.argv[2] === ROOT)).toBe(true)
})

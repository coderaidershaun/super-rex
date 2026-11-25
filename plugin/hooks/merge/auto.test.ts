// Proves auto-merge of a job a person starts: in a repo whose config holds auto_merge, a job with no merge answer merges with no question, and a no of the person still holds.
import { expect, test } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { parseConfig } from '../book/config'
import { autoMergeIn } from '../book/jobs'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { CONFIG_AUTO_MERGE, CONFIG_ON, ROOT, START_MS, world } from '../book/kit'
import { files, JOB, ok, plannedAll, promptsPass, seededBook } from '../tools/act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const MERGE = 'mcp__super-rex__rex_merge' as const

/** Answers every git call of a merge as a clean one squash commit over the base on origin, and records each call. */
function cleanGit(calls: ProcessCall[]): (call: ProcessCall) => ProcessAnswer {
  return call => {
    calls.push(call)
    if (call.argv.includes('remote')) return ok('origin\n')
    if (call.argv.includes('for-each-ref')) return ok('refs/remotes/origin/main\n')
    if (call.argv.includes('rev-list')) return ok('1\n')
    if (call.argv.includes('rev-parse')) return ok('c1\n')
    if (call.argv[0] === 'rex') return ok('last_sequence=0\n')
    return ok()
  }
}

function pushes(calls: readonly ProcessCall[]): readonly (readonly string[])[] {
  return calls.filter(call => call.argv.includes('push')).map(call => call.argv)
}

test('the config key auto_merge is read as true or false, is false when absent, and any other value makes the config unreadable', () => {
  const on = parseConfig(CONFIG_AUTO_MERGE)
  const absent = parseConfig(CONFIG_ON)
  const bad = parseConfig('{ "storage": "on", "auto_merge": "yes" }')

  expect(on.kind === 'read' && on.config.autoMerge).toBe(true)
  expect(absent.kind === 'read' && absent.config.autoMerge).toBe(false)
  expect(bad).toEqual({ kind: 'unreadable', reason: 'auto_merge must be true or false, got "yes"' })
})

test('auto-merge answers yes only for a job with no merge answer, and only in a book that has it on', () => {
  const seed = seededBook()
  const off = autoMergeIn(seed, JOB, START_MS)
  const answered = autoMergeIn({ ...seed, autoMerge: true }, JOB, START_MS)
  const no = { answer: 'no', source: 'person-prompt', question: null, policyHash: null, at: START_MS } as const
  const held: RexBook = { ...seed, autoMerge: true, jobs: seed.jobs.map(job => ({ ...job, merge: no })) }

  expect(off.jobs[0]?.merge).toBeNull()
  expect(answered.jobs[0]?.merge).toEqual({ answer: 'yes', source: 'policy', question: null, policyHash: null, at: START_MS })
  expect(autoMergeIn(held, JOB, START_MS).jobs[0]?.merge).toEqual(no)
})

test('in a repo with auto-merge a job with no merge answer merges at rex_merge with no question', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  const session = world(on, { config: CONFIG_AUTO_MERGE, seed: plannedAll(seededBook()), processAnswer: cleanGit(calls) })
  await $.session.start(START)

  const merged = await $.tool.call({ tool: MERGE })

  expect(merged.deny).toBeUndefined()
  expect(pushes(calls)).toEqual([['git', '-C', ROOT, 'push', 'origin', `rex/${JOB}:refs/heads/main`]])
  const job = session.book().jobs[0]
  expect([job?.merge?.answer, job?.merge?.source, job?.merge?.policyHash]).toEqual(['yes', 'policy', null])
  expect(session.book().questions).toEqual([])
})

test('in a repo with auto-merge the person\'s merge: no still holds, and their later merge: yes merges', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const calls: ProcessCall[] = []
  world(on, { config: CONFIG_AUTO_MERGE, seed: plannedAll(seededBook()), processAnswer: cleanGit(calls) })
  await $.session.start(START)

  await $.prompt.submit({ text: 'merge: no', origin: { kind: 'composer' }, wait: false })
  const no = await $.tool.call({ tool: MERGE })
  expect(pushes(calls)).toEqual([])
  await $.prompt.submit({ text: 'merge: yes', origin: { kind: 'composer' }, wait: false })
  const yes = await $.tool.call({ tool: MERGE })

  expect(no.deny).toMatch(/has no merge answer, got no \(person-prompt\);/)
  expect(yes.deny).toBeUndefined()
  expect(pushes(calls)).toHaveLength(1)
})

test('without auto-merge a job with no merge answer is still refused', async ($, on) => {
  files(on, {})
  const calls: ProcessCall[] = []
  world(on, { config: CONFIG_ON, seed: plannedAll(seededBook()), processAnswer: cleanGit(calls) })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: MERGE })

  expect(refused.deny).toMatch(/has no merge answer, got none;/)
  expect(pushes(calls)).toEqual([])
})

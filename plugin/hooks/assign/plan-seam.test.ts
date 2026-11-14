// Proves that rex_job_plan reads what rex_assign and overrides record: a sibling file reaches the plan and its manifest with its file, and an override reaches the plan of the call that carries it.
import { expect, test } from 'claude-code/testing'

import type { RexDecision } from '../../types'
import { CONFIG_OFF, ROOT, world } from '../book/kit'
import { sharedJson } from '../book/shared'
import { callFile, files, ok, scripted, seededBook } from '../tools/act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const OMS = callFile(sharedJson('fixtures/shared/calls/oms.json'))
const PREFS = { duration: null, budget: 'high', autonomy: 'high' } as const
const SKILLS = {
  'plugin:library/rex-code-tdd/SKILL.md': '# TDD\n',
  'plugin:library/rex-code-tdd/mocking.md': '# Mocking\n',
  'plugin:library/rex-gap-ts/SKILL.md': '# Gap\n',
}
const PLAN = {
  orchestrator: 'lead',
  breakdown: 'T2 engineer',
  tasks: [{ id: 'T2', role: 'engineer', skills: ['rex-code-tdd'], open_when: [] }],
  roles: [{ role: 'engineer', intelligence: 'high', reliability: 'high', consequence: 'high', verification: 'low' }],
}

function recorded(item: string, value: string): RexDecision {
  return { assignmentId: `id-${item}`, subject: 'task:T2', item, value, source: 'decision-agent', reason: 'answered', tableVersion: 'sha256:t', overlayVersion: null }
}

test('a sibling file in Open when reaches the plan and its manifest with its file', async ($, on) => {
  files(on, SKILLS)
  const seed = seededBook({ prefs: PREFS })
  const decisions = [recorded('skill', 'rex-code-tdd'), recorded('file.rex-code-tdd/mocking.md', 'a test crosses the file system')]
  const session = world(on, { config: CONFIG_OFF, seed: { ...seed, jobs: seed.jobs.map(job => ({ ...job, decisions })) }, processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }]) })
  await $.session.start(START)

  await $.tool.call({ tool: 'mcp__super-rex__rex_job_plan', ...PLAN })

  const plan = session.book().jobs[0]?.plan
  expect(plan?.tasks).toEqual([{ id: 'T2', role: 'engineer', skills: ['rex-code-tdd'], openWhen: [{ skill: 'rex-code-tdd', file: 'mocking.md', when: 'a test crosses the file system' }] }])
  expect(plan?.manifest.map(file => `${file.skill}/${file.file}`)).toEqual(['rex-code-tdd/SKILL.md', 'rex-code-tdd/mocking.md'])
})

test('an override reaches the plan of the call that carries it', async ($, on) => {
  files(on, SKILLS)
  const session = world(on, { config: CONFIG_OFF, seed: seededBook({ prefs: PREFS }), processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }]) })
  await $.session.start(START)

  await $.tool.call({
    tool: 'mcp__super-rex__rex_job_plan',
    ...PLAN,
    overrides: [
      { subject: 'task:T2', item: 'skill', value: 'rex-code-tdd', reason: 'Not in table: kept from the plan' },
      { subject: 'task:T2', item: 'skill', value: 'rex-gap-ts', reason: 'Not in table: a TypeScript fixture' },
    ],
  })

  expect(session.book().jobs[0]?.plan?.tasks).toEqual([{ id: 'T2', role: 'engineer', skills: ['rex-code-tdd', 'rex-gap-ts'], openWhen: [] }])
})

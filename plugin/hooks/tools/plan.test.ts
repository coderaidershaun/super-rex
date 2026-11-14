// Proves rex_job_plan (once, then only with amend; grid agents from rex oms; the skill manifest), rex_recall and rex_job_close against their shared call files.
import { expect, test } from 'claude-code/testing'

import { CONFIG_OFF, CONFIG_ON, ROOT, world } from '../book/kit'
import type { ProcessCall } from '../book/kit'
import { sharedJson } from '../book/shared'
import { callFile, exit, files, JOB, lineOf, ok, resultText, scripted, seededBook } from './act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const OMS = callFile(sharedJson('fixtures/shared/calls/oms.json'))
const SKILLS = { 'plugin:library/rex-code-tdd/SKILL.md': '# TDD\n', 'plugin:library/rex-code-tdd/mocking.md': '# Mocking\n' }
const PLAN = {
  orchestrator: 'orchestration-builder 0.8.0',
  breakdown: 'T2 engineer',
  tasks: [{ id: 'T2', role: 'engineer', skills: ['rex-code-tdd'], open_when: [{ skill: 'rex-code-tdd', file: 'mocking.md', when: 'a test crosses a process' }] }],
  roles: [{ role: 'engineer', intelligence: 'high', reliability: 'high', consequence: 'high', verification: 'low' }],
}

test('rex_job_plan works once, takes the grid agent from rex oms and records the manifest', async ($, on) => {
  files(on, SKILLS)
  const calls: ProcessCall[] = []
  const session = world(on, {
    config: CONFIG_OFF,
    seed: seededBook({ prefs: { duration: null, budget: 'high', autonomy: 'high' } }),
    processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }, { argv: OMS.argv, answer: ok(OMS.stdout) }], calls),
  })
  await $.session.start(START)

  const planned = await $.tool.call({ tool: 'mcp__super-rex__rex_job_plan', ...PLAN })
  const second = await $.tool.call({ tool: 'mcp__super-rex__rex_job_plan', ...PLAN })
  const amended = await $.tool.call({ tool: 'mcp__super-rex__rex_job_plan', ...PLAN, amend: true })

  expect(resultText(planned)).toBe(`job=${JOB}\nstatus=planned\norchestrator=orchestration-builder 0.8.0\nrole.engineer=super-rex:model-xhigh-effort-max\nmanifest_files=2\n`)
  expect(second.deny).toBe(`rex_job_plan: job ${JOB} has a plan already, got a second plan; call it with amend: true to replace the rows it names`)
  expect(lineOf(amended, 'role.engineer')).toBe('super-rex:model-xhigh-effort-max')
  expect(calls.map(call => call.argv)).toEqual([OMS.argv, OMS.argv])
  const plan = session.book().jobs[0]?.plan
  expect(plan?.roles.map(role => [role.role, role.gridAgent, role.omsVersion])).toEqual([['engineer', 'model-xhigh-effort-max', 2]])
  expect(plan?.manifest.map(file => [file.skill, file.file, file.source, file.hash.slice(0, 7)])).toEqual([
    ['rex-code-tdd', 'SKILL.md', 'plugin', 'sha256:'],
    ['rex-code-tdd', 'mocking.md', 'plugin', 'sha256:'],
  ])
})

test('rex_job_plan takes the skills rex_assign recorded over the ones in its arguments', async ($, on) => {
  files(on, { ...SKILLS, 'plugin:library/rex-gap-ts/SKILL.md': '# Gap\n' })
  const seed = seededBook({ prefs: { duration: null, budget: 'high', autonomy: 'high' } })
  const decision = { assignmentId: 'a1', subject: 'task:T2', item: 'skill', value: 'rex-gap-ts', source: 'rule', reason: 'TypeScript task', tableVersion: '1', overlayVersion: null } as const
  const withDecision = { ...seed, jobs: seed.jobs.map(job => ({ ...job, decisions: [decision] })) }
  const session = world(on, { config: CONFIG_OFF, seed: withDecision, processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }]) })
  await $.session.start(START)

  await $.tool.call({ tool: 'mcp__super-rex__rex_job_plan', ...PLAN })

  expect(session.book().jobs[0]?.plan?.tasks).toEqual([{ id: 'T2', role: 'engineer', skills: ['rex-gap-ts'], openWhen: [] }])
})

test('rex_job_plan refuses a skill no folder holds and changes nothing', async ($, on) => {
  files(on, {})
  const session = world(on, { config: CONFIG_OFF, seed: seededBook({ prefs: { duration: null, budget: 'high', autonomy: 'high' } }), processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }]) })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_job_plan', ...PLAN })

  expect(refused.deny).toBe('rex_job_plan: no skill rex-code-tdd holds SKILL.md, got rex-code-tdd/SKILL.md; name a skill of plugin/skills, plugin/library or .rex/skills')
  expect(session.book().jobs[0]?.plan).toBeNull()
})

test('rex_recall answers the binary unchanged with storage on and off with storage off', async ($, on) => {
  const recall = callFile(sharedJson('fixtures/shared/calls/recall.json'))
  files(on, {})
  world(on, { config: CONFIG_ON, seed: seededBook({ storage: 'on' }), processAnswer: scripted([{ argv: recall.argv, answer: ok(recall.stdout) }]) })
  await $.session.start(START)

  const read = await $.tool.call({ tool: 'mcp__super-rex__rex_recall', job: JOB })
  const other = await $.tool.call({ tool: 'mcp__super-rex__rex_recall', job: '0gb0000000' })

  expect(resultText(read)).toBe('job=0gaq820001\nrecorded=yes\nlessons=0\norchestrators=0\nunused=0\n')
  expect(other.deny).toBe(`rex_recall: job 0gb0000000 is not your job, got 0gb0000000; call it for ${JOB}`)
})

test('tools answer with storage off', async ($, on) => {
  files(on, {})
  world(on, { config: CONFIG_OFF, seed: seededBook() })
  await $.session.start(START)

  const recall = await $.tool.call({ tool: 'mcp__super-rex__rex_recall', job: JOB })
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })
  const close = await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', status: 'blocked', outcome: 'stopped', deliveries: [] })

  expect(resultText(recall)).toBe('recall=off\nreason=storage is off; run rex init --with-storage\n')
  expect(resultText(status)).toBe(`job=${JOB}\nstatus=created\nphase=plan\norigin=composer\nagents_open=0\nagents_done=0\nspent_usd=0.00\nlimit_usd=10.00\ndegraded=none\n`)
  expect(lineOf(close, 'status')).toBe('blocked')
  expect(lineOf(close, 'cost_of_record_usd')).toBeUndefined()
})

test('rex_job_close is safe to repeat, refuses a merged delivery with no merge, and reads the cost of record', async ($, on) => {
  const show = callFile(sharedJson('fixtures/shared/calls/job-show.json'))
  const shown = show.stdout.replace(/\}\s*$/, ',"cost":{"usd":1.5,"unpriced_requests":0}}\n')
  files(on, {})
  const session = world(on, { config: CONFIG_ON, seed: seededBook({ storage: 'on' }), processAnswer: scripted([{ argv: show.argv, answer: ok(shown) }, { argv: show.argv, answer: exit(2, 'store is version 1; run rex migrate\n') }]) })
  await $.session.start(START)

  const merged = await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', status: 'delivered', outcome: 'done', deliveries: [{ repo: ROOT, outcome: 'merged' }] })
  const closed = await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', status: 'delivered', outcome: 'done', deliveries: [{ repo: ROOT, branch: `rex/${JOB}`, outcome: 'kept-local' }] })
  const again = await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', status: 'delivered', outcome: 'done', deliveries: [] })
  const changed = await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', status: 'failed', outcome: 'done', deliveries: [] })

  expect(merged.deny).toBe(`rex_job_close: delivery to ${ROOT} is merged, got no merge by rex_merge for job ${JOB}; merge it with rex_merge, or close it kept-local`)
  expect(resultText(closed)).toBe(`job=${JOB}\nstatus=delivered\nspent_usd=0.00\nruns=0\nnot_assembled=0\ndegraded=none\ncost_of_record_usd=1.50\n`)
  expect(lineOf(again, 'status')).toBe('delivered')
  expect(lineOf(again, 'cost_of_record_usd')).toBe('unknown')
  expect(changed.deny).toBe(`rex_job_close: job ${JOB} is closed delivered, got failed; call it again with the same status and outcome, or open a new job`)
  expect(session.book().jobs[0]?.deliveries).toEqual([{ repo: ROOT, branch: `rex/${JOB}`, pr: null, outcome: 'kept-local', mergeCommit: null, note: null }])
})

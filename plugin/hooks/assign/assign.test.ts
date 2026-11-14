// Proves rex_assign through the engine: questions then skills with their sources, a repeated assignment id that gives the stored answer, the callers, bad answers and overlays refused, the record that rex_job_plan and rex_status read, and every key of the shared tools table.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'

import type { RexBook } from '../../types'
import { CONFIG_OFF, ROOT, START_MS, world } from '../book/kit'
import { startRunIn } from '../book/runs'
import { sharedJson } from '../book/shared'
import { callFile, files, lineOf, ok, resultText, scripted, seededBook } from '../tools/act-kit'
import { jsonField } from '../tools/binary'
import { decisionItems } from './serve'
import { TABLE_TEXT } from './table.fixture'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const OMS = callFile(sharedJson('fixtures/shared/calls/oms.json'))
const PREFS = { duration: null, budget: 'high', autonomy: 'high' } as const
const ROLES = [{ role: 'engineer', intelligence: 'high', reliability: 'high', consequence: 'high', verification: 'low' }]
const TASKS = [{ id: 'T2', role: 'engineer', writes_code: true, langs: ['ts'] }]
const TOOL = 'mcp__super-rex__rex_assign'

/** The plugin's table at its place, and an overlay in the repo when one is given. */
function tableFiles(on: On, overlay: string | null = null): void {
  on('fs.read', { path: /\/assign\/table\.json$/ }, () => ({ value: TABLE_TEXT }))
  on('fs.exists', { path: /\/\.rex\/assign\.json$/ }, () => ({ value: overlay !== null }))
  on('fs.read', { path: /\/\.rex\/assign\.json$/ }, () => (overlay === null ? { deny: 'ENOENT' } : { value: overlay }))
}

function seed(): RexBook {
  return seededBook({ prefs: PREFS })
}

type Pick = (row: string, criterion: string) => 'yes' | 'no'

/** The tdd row and the failure policy, both in each step; every other question no. */
const BUILDER_PICK: Pick = (row, criterion) => (row === 'tdd' && (criterion === 'builds-feature' || criterion === 'each-step')) || (row === 'failure-policy' && (criterion === 'outside-input' || criterion === 'each-step')) ? 'yes' : 'no'

function questionsOf(result: unknown): readonly { readonly key: string; readonly row: string; readonly criterion: string }[] {
  return resultText(result)
    .split('\n')
    .filter(line => line.startsWith('question.'))
    .map(line => {
      const key = /=(task:\S+ row:(\S+) criterion:(\S+))/.exec(line)
      return { key: key?.[1] ?? '', row: key?.[2] ?? '', criterion: key?.[3] ?? '' }
    })
}

/** Calls rex_assign until no question is left, answering each by `pick`; answers each result on the way. */
async function assignAll(call: (args: object) => Promise<unknown>, pick: Pick, args: object = {}): Promise<readonly unknown[]> {
  const results: unknown[] = [await call({ tool: TOOL, tasks: TASKS, roles: ROLES, ...args })]
  for (let round = 0; round < 4 && lineOf(results[results.length - 1], 'questions') !== '0'; round += 1) {
    const answers = questionsOf(results[results.length - 1]).map(question => ({ question: question.key, answer: pick(question.row, question.criterion), reason: `${question.row} ${question.criterion} by the test` }))
    results.push(await call({ tool: TOOL, tasks: TASKS, roles: ROLES, answers, ...args }))
  }
  return results
}

test('rex_assign asks first, then answers each task skill with its source and the grid agent of each role, and records each with the table version', async ($, on) => {
  tableFiles(on)
  files(on, {})
  const session = world(on, { config: CONFIG_OFF, seed: seed(), processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }]) })
  await $.session.start(START)

  const results = await assignAll(args => $.tool.call({ tool: TOOL, ...args }), BUILDER_PICK)

  expect(results).toHaveLength(3)
  expect(lineOf(results[0], 'questions')).toBe(String(questionsOf(results[0]).length))
  expect(questionsOf(results[1]).map(question => `${question.row}/${question.criterion}`)).toEqual(['tdd/each-step', 'failure-policy/each-step'])
  expect(resultText(results[2])).toBe(
    [
      'questions=0',
      'task.T2.skills=rex-code-philosophy,rex-code-commenting,rex-code-ergonomics,rex-code-error-writing,rex-gap-ts,rex-code-tdd',
      'task.T2.open_when.rex-code-failure-policy=handles outside input',
      'task.T2.open_when.rex-code-failure-policy/failure-table.md=it plans such a feature: writes its failure table',
      'source.T2.rex-code-philosophy=rule',
      'source.T2.rex-code-commenting=rule',
      'source.T2.rex-code-ergonomics=rule',
      'source.T2.rex-code-error-writing=rule',
      'source.T2.rex-gap-ts=rule',
      'source.T2.rex-code-tdd=decision-agent',
      'source.T2.rex-code-failure-policy=decision-agent',
      'source.T2.rex-code-failure-policy/failure-table.md=decision-agent',
      'role.engineer=super-rex:model-xhigh-effort-max',
      '',
    ].join('\n'),
  )
  const decisions = session.book().jobs[0]?.decisions ?? []
  const tdd = decisions.find(decision => decision.subject === 'task:T2' && decision.value === 'rex-code-tdd')
  expect([tdd?.item, tdd?.source, tdd?.reason, tdd?.tableVersion?.slice(0, 7), tdd?.overlayVersion]).toEqual(['skill', 'decision-agent', 'tdd/builds-feature: tdd builds-feature by the test; each step: tdd each-step by the test', 'sha256:', null])
  const policy = decisions.find(decision => decision.item === 'open_when.rex-code-failure-policy')
  expect(policy?.reason).toBe('failure-policy/outside-input: failure-policy outside-input by the test; each step: failure-policy each-step by the test; over the cap of 6 read-first skills')
  expect(decisions.filter(decision => decision.item.startsWith('answer.')).length).toBe(questionsOf(results[0]).length + 2)
  expect(decisions.find(decision => decision.subject === 'role:engineer')).toMatchObject({ item: 'grid_agent', value: 'super-rex:model-xhigh-effort-max', source: 'decision-agent', tableVersion: null })
  expect(decisions.find(decision => decision.item === 'file.rex-code-failure-policy/failure-table.md')).toMatchObject({ value: 'it plans such a feature: writes its failure table', source: 'decision-agent' })
})

test('a repeated assignment id gives the stored answer', async ($, on) => {
  tableFiles(on)
  files(on, {})
  const session = world(on, { config: CONFIG_OFF, seed: seed(), processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }, { argv: OMS.argv, answer: ok(OMS.stdout) }]) })
  await $.session.start(START)
  const first = await assignAll(args => $.tool.call({ tool: TOOL, ...args }), BUILDER_PICK)
  const asked = questionsOf(first[0])
  const stored = session.book().jobs[0]?.decisions ?? []

  const flipped = asked.map(question => ({ question: question.key, answer: BUILDER_PICK(question.row, question.criterion) === 'yes' ? 'no' : 'yes', reason: 'a second opinion' }))
  const again = await $.tool.call({ tool: TOOL, tasks: TASKS, roles: ROLES, answers: flipped })

  expect(resultText(again)).toBe(resultText(first[first.length - 1]))
  expect(session.book().jobs[0]?.decisions).toEqual(stored)
  const tdd = session.book().jobs[0]?.decisions.find(decision => decision.item === 'answer.tdd.builds-feature')
  expect([tdd?.value, tdd?.reason]).toEqual(['yes', 'tdd builds-feature by the test'])
})

test('rex_assign answers a builder and the decision agent of its job, before a plan by its Task line, and no one else', async ($, on) => {
  tableFiles(on)
  files(on, {})
  const at = START_MS + 1000
  const spawn = { toolUseId: null, parentAgentId: null, agentType: 'super-rex:model-high-effort-high', isOwnSpawn: false } as const
  const withDecider = startRunIn(seed(), { ...spawn, agentId: 'd1', description: 'decide', task: 'decision-maker' }, at)
  const withOther = startRunIn(withDecider, { ...spawn, agentId: 'w1', description: 'work', task: 'T2' }, at)
  const session = world(on, { config: CONFIG_OFF, seed: withOther })
  await $.session.start(START)

  const decider = await $.tool.call({ tool: TOOL, agentId: 'd1', tasks: TASKS, roles: [] })
  const other = await $.tool.call({ tool: TOOL, agentId: 'w1', tasks: TASKS, roles: [] })
  const stranger = await $.tool.call({ tool: TOOL, agentId: 'x9', tasks: TASKS, roles: [] })
  const builder = await $.tool.call({ tool: TOOL, tasks: TASKS, roles: [] })

  expect(lineOf(decider, 'questions')).toBe(lineOf(builder, 'questions'))
  expect(other.deny).toBe('rex_assign: only a builder or the decision-maker agent of its job may call it, got agent')
  expect(stranger.deny).toBe('rex_assign: only a builder or the decision-maker agent of its job may call it, got agent')
  expect(session.book().jobs[0]?.decisions).toEqual([])
})

test('a bad answer is refused with the value seen, and nothing is recorded', async ($, on) => {
  tableFiles(on)
  files(on, {})
  const session = world(on, { config: CONFIG_OFF, seed: seed() })
  await $.session.start(START)
  const key = 'task:T2 row:tdd criterion:builds-feature'

  const maybe = await $.tool.call({ tool: TOOL, tasks: TASKS, roles: [], answers: [{ question: key, answer: 'maybe', reason: 'unsure' }] })
  const silent = await $.tool.call({ tool: TOOL, tasks: TASKS, roles: [], answers: [{ question: key, answer: 'yes', reason: ' ' }] })
  const unknown = await $.tool.call({ tool: TOOL, tasks: TASKS, roles: [], answers: [{ question: 'task:T2 row:tdd criterion:nope', answer: 'yes', reason: 'r' }] })
  const ruled = await $.tool.call({ tool: TOOL, tasks: TASKS, roles: [], answers: [{ question: 'task:T2 row:code-rulebooks criterion:writes-code', answer: 'no', reason: 'skip them' }] })
  const twice = await $.tool.call({ tool: TOOL, tasks: TASKS, roles: [], answers: [{ question: key, answer: 'yes', reason: 'r' }, { question: key, answer: 'no', reason: 'r' }] })
  const noCode = await $.tool.call({ tool: TOOL, tasks: [{ id: 'T2', role: 'engineer' }], roles: [] })

  expect(maybe.deny).toBe(`rex_assign: the answer to ${key} must be yes or no, got maybe; give each answer the question, yes or no, and a one-line reason`)
  expect(silent.deny).toBe(`rex_assign: the answer to ${key} needs a reason, got  ; give each answer the question, yes or no, and a one-line reason`)
  expect(unknown.deny).toBe('rex_assign: an answer names no question of these tasks, got task:T2 row:tdd criterion:nope; copy the task:<id> row:<row> criterion:<id> text of a question line')
  expect(ruled.deny).toBe('rex_assign: an answer names no question of these tasks, got task:T2 row:code-rulebooks criterion:writes-code; copy the task:<id> row:<row> criterion:<id> text of a question line')
  expect(twice.deny).toBe(`rex_assign: ${key} has two answers, got yes and no; give each question one answer`)
  expect(noCode.deny).toBe('rex_assign: task T2 writes_code must be true or false, got none; give each task id, role and writes_code, and langs when they differ from the job')
  expect(session.book().jobs[0]?.decisions).toEqual([])
})

test('an overlay of the repo changes one row for the job and is recorded with its version; a broken one is refused', async ($, on) => {
  const table: unknown = JSON.parse(TABLE_TEXT)
  const rows: unknown = jsonField(table, 'rows')
  const tdd: unknown = Array.isArray(rows) ? rows.find((row: unknown) => jsonField(row, 'id') === 'tdd') : undefined
  const overlay = JSON.stringify({ format: 1, rows: [{ ...(typeof tdd === 'object' && tdd !== null ? tdd : {}), skills: ['rex-house-tdd'] }] })
  tableFiles(on, overlay)
  files(on, {})
  const session = world(on, { config: CONFIG_OFF, seed: seed(), processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }]) })
  await $.session.start(START)

  const results = await assignAll(args => $.tool.call({ tool: TOOL, ...args }), BUILDER_PICK)

  expect(lineOf(results[results.length - 1], 'task.T2.skills')).toBe('rex-code-philosophy,rex-code-commenting,rex-code-ergonomics,rex-code-error-writing,rex-gap-ts,rex-house-tdd')
  const house = session.book().jobs[0]?.decisions.find(decision => decision.value === 'rex-house-tdd')
  expect(house?.overlayVersion?.slice(0, 7)).toBe('sha256:')
})

test('a broken overlay is refused with its problem and the file to fix', async ($, on) => {
  tableFiles(on, '{"format": 1, "remove": ["no-such-row"]}')
  files(on, {})
  const session = world(on, { config: CONFIG_OFF, seed: seed() })
  await $.session.start(START)

  const refused = await $.tool.call({ tool: TOOL, tasks: TASKS, roles: [] })

  expect(refused.deny).toBe(`rex_assign: the assignment table is broken, got overlay.remove names no row of the table, got "no-such-row"; fix ${ROOT}/.rex/assign.json`)
  expect(session.book().jobs[0]?.decisions).toEqual([])
})

test('rex_job_plan takes the skills rex_assign recorded, with their sources, and rex_status gives each decision with its source and reason', async ($, on) => {
  tableFiles(on)
  const skillFiles = Object.fromEntries(
    ['rex-code-philosophy', 'rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts', 'rex-code-tdd', 'rex-code-spike'].map(skill => [`plugin:library/${skill}/SKILL.md`, `# ${skill}\n`]),
  )
  files(on, { ...skillFiles, 'plugin:library/rex-code-tdd/mocking.md': '# Mocking\n' })
  const session = world(on, { config: CONFIG_OFF, seed: seed(), processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }, { argv: OMS.argv, answer: ok(OMS.stdout) }]) })
  await $.session.start(START)
  const pick: Pick = (row, criterion) => (row === 'tdd' && (criterion === 'builds-feature' || criterion === 'each-step')) || (row === 'spike' && criterion === 'unproven') || (row === 'boundary-tests' && criterion === 'file-system') ? 'yes' : 'no'
  await assignAll(args => $.tool.call({ tool: TOOL, ...args }), pick)

  const planned = await $.tool.call({ tool: 'mcp__super-rex__rex_job_plan', orchestrator: 'lead', breakdown: 'T2', tasks: [{ id: 'T2', role: 'engineer', skills: [], open_when: [] }], roles: ROLES })
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })

  expect(lineOf(planned, 'manifest_files')).toBe('8')
  expect(session.book().jobs[0]?.plan?.tasks).toEqual([
    {
      id: 'T2',
      role: 'engineer',
      skills: ['rex-code-philosophy', 'rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts', 'rex-code-tdd'],
      openWhen: [
        { skill: 'rex-code-spike', file: null, when: 'rests on an approach, crate or outside service that is not proven' },
        { skill: 'rex-code-tdd', file: 'mocking.md', when: 'writes a test that crosses the file system' },
      ],
    },
  ])
  const lines = resultText(status).split('\n')
  expect(lines).toContain('decision.T2.rex-gap-ts=rule: read first; gap-skill/language-not-rust: writes code in a language that is not rust')
  expect(lines).toContain('decision.T2.rex-code-spike=decision-agent: open when rests on an approach, crate or outside service that is not proven; spike/unproven: spike unproven by the test; one case: spike each-step by the test')
  expect(lines).toContain('decision.T2.tdd.fixes-bug=decision-agent: no; tdd fixes-bug by the test')
  expect(lines).toContain('decision.engineer.grid_agent=decision-agent: super-rex:model-xhigh-effort-max; rated intelligence high, reliability high, consequence high, verification low')
})

/** The key of each `key=value` line of a result. */
function keysOf(result: unknown): readonly string[] {
  return resultText(result)
    .split('\n')
    .filter(line => line.includes('='))
    .map(line => line.slice(0, line.indexOf('=')))
}

test('rex_assign answers each key of the shared tools table', async ($, on) => {
  tableFiles(on)
  files(on, {})
  world(on, { config: CONFIG_OFF, seed: seed(), processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }]) })
  await $.session.start(START)

  const results = await assignAll(args => $.tool.call({ tool: TOOL, ...args }), BUILDER_PICK)

  const listed = jsonField(jsonField(jsonField(sharedJson('fixtures/shared/tables/tools.json'), 'tools'), 'rex_assign'), 'keys')
  const seen = results.flatMap(keysOf)
  const matches = (key: string) => new RegExp(`^${key.split(/<[^>]+>/).map(part => part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('.+')}$`)
  expect((Array.isArray(listed) ? listed.map(String) : []).filter(key => !seen.some(one => matches(key).test(one)))).toEqual([])
})

const PANE = {
  plugin: 'super-rex',
  component: 'Pane',
  requestId: 'rex',
  props: { title: 'SuperRex', isFocused: true, bodyColumns: 120, placement: 'dock', scroll: { offset: 0, bodyRows: 40 }, view: {}, id: 'rex', plugin: 'super-rex' },
} as const

test('the Decisions view shows each recorded assignment with its source and reason', async ($, on) => {
  tableFiles(on)
  files(on, {})
  world(on, { config: CONFIG_OFF, seed: seed(), processAnswer: scripted([{ argv: OMS.argv, answer: ok(OMS.stdout) }]) })
  await $.session.start(START)
  await assignAll(args => $.tool.call({ tool: TOOL, ...args }), BUILDER_PICK)

  const ui = await $.ui.mount({ ...PANE, surface: 'terminal' })
  await ui.press({ key: 'view-decisions' })
  const rows = (await ui.findAll({ type: 'Text' })).map(found => found.text)
  await ui.unmount()

  expect(rows).toContain('0gaq820001 task:T2 skill=rex-gap-ts · rule: gap-skill/language-not-rust: writes code in a language that is not rust')
  expect(rows).toContain('0gaq820001 task:T2 skill=rex-code-tdd · decision-agent: tdd/builds-feature: tdd builds-feature by the test; each step: tdd each-step by the test')
  expect(rows).toContain('0gaq820001 task:T2 answer.tdd.fixes-bug=no · decision-agent: tdd fixes-bug by the test')
})

test('a sibling file is recorded as a file of its skill, beside the skill when the task reads it first', () => {
  const base = { source: 'decision-agent', reason: 'r', row: 'boundary-tests' } as const
  const items = decisionItems([
    { ...base, skill: 'rex-code-tdd', file: null, place: 'read-first', when: null },
    { ...base, skill: 'rex-code-tdd', file: 'mocking.md', place: 'open-when', when: 'a test crosses the file system' },
  ])

  expect(items.map(one => [one.item, one.value])).toEqual([
    ['skill', 'rex-code-tdd'],
    ['file.rex-code-tdd/mocking.md', 'a test crosses the file system'],
  ])
})

// Snapshots of the band and of each pane view, drawn from one book on two surfaces, and an answer given by a press in the pane.
import { expect, test } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { emptyBook } from '../book/events'
import { decisionIn, openJobIn, planJobIn, questionIn } from '../book/jobs'
import { ROOT, SESSION, START_MS, world } from '../book/kit'
import { startRunIn, stepRunIn } from '../book/runs'
import { sharedJson } from '../book/shared'
import { createParts } from '../parts'
import { DECISIONS_VIEW } from './decisions'
import { drawPane } from './draw'
import { PROBE_VIEW } from './probe-view.fixture'
import { HEADER_VIEW, JOBS_VIEW, QUESTIONS_VIEW, TREE_VIEW } from './views'

function sharedArgv(path: string): unknown {
  const call = sharedJson(path)
  return typeof call === 'object' && call !== null ? Reflect.get(call, 'argv') : undefined
}

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const SURFACES = ['terminal', 'desktop'] as const
const BAND = {
  plugin: 'super-rex',
  component: 'AbovePrompt',
  props: { hasSurvey: false, isCollapsed: false, isWorking: true, maxRows: 3, bodyColumns: 100, scroll: { offset: 0, bodyRows: 3 }, view: {} },
} as const
const PANE = {
  plugin: 'super-rex',
  component: 'Pane',
  requestId: 'rex',
  props: { title: 'SuperRex', isFocused: true, bodyColumns: 120, placement: 'dock', scroll: { offset: 0, bodyRows: 40 }, view: {}, id: 'rex', plugin: 'super-rex' },
} as const

const JOB_INPUT = {
  builder: 'main',
  tree: null,
  origin: 'composer',
  request: 'Fix the parser',
  repo: ROOT,
  base: 'main',
  type: 'fix',
  size: 'small',
  consequence: 'low',
  langs: [],
  prefs: { duration: null, budget: null, autonomy: null },
  limit: { nano: 10_000_000_000, source: 'default' },
  dependsOn: [],
  mergeAfter: [],
} as const

const RATINGS = { intelligence: 'high', reliability: 'high', consequence: 'medium', verification: 'low' } as const

/** A job of the main loop with a plan, a decision, a lead with one worker, and a merge question. */
function watchedBook(): RexBook {
  const base: RexBook = { ...emptyBook(), session: SESSION, prompts: 'observe' }
  const opened = openJobIn(base, JOB_INPUT, '0gaq820001', START_MS).book
  const plan = {
    orchestrator: 'lead-and-workers',
    breakdown: 'one lead, one worker',
    tasks: [{ id: 'T1', role: 'engineer', skills: ['rex-code-tdd'], openWhen: [] }],
    roles: [{ role: 'engineer', ratings: RATINGS, gridAgent: 'model-high-effort-high', conflict: null, omsVersion: 2 }],
    manifest: [],
  }
  const planned = planJobIn(opened, '0gaq820001', plan, START_MS + 1000)
  const decided = decisionIn(
    planned,
    '0gaq820001',
    { assignmentId: 'a1', subject: 'task:T1', item: 'skill', value: 'rex-code-tdd', source: 'rule', reason: 'the task writes code', tableVersion: '1', overlayVersion: null },
    START_MS + 1000,
  ).book
  const spawn = { toolUseId: 'toolu_1', isOwnSpawn: false, description: 'lead' }
  const lead = startRunIn(decided, { ...spawn, agentId: 'a1', parentAgentId: null, agentType: 'super-rex:model-high-effort-high' }, START_MS + 2000)
  const worker = startRunIn(lead, { ...spawn, agentId: 'a2', parentAgentId: 'a1', agentType: 'super-rex:model-low-effort-low', description: 'worker' }, START_MS + 3000)
  const usage = { model: 'claude-opus-5-5', input_tokens: 1000, output_tokens: 500, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 }
  const stepped = stepRunIn(worker, 'a1', { model: 'claude-opus-5-5', effort: 'high', usage }, START_MS + 4000)
  return questionIn(
    stepped,
    { tree: null, job: '0gaq820001', question: 'Merge job 0gaq820001?', options: ['Merge', 'Keep the branch'], recommendation: 'Merge', category: 'merge', mergeFor: '0gaq820001' },
    START_MS + 5000,
  ).book
}

test('the band names the job, its agents, spend, depth, questions and permission asks', async ($, on) => {
  world(on, { seed: watchedBook() })
  on('tool.check', (_$, e) => ({ decision: e.tool === 'Bash' ? 'ask' : 'allow' }))
  await $.session.start(START)
  await $.tool.check({ tool: 'Bash', input: { command: 'rm -rf build' }, agentId: 'a1' })
  await $.tool.check({ tool: 'Read', input: { file_path: '/r/a.rs' }, agentId: 'a1' })

  for (const surface of SURFACES) {
    const ui = await $.ui.mount({ ...BAND, surface })
    expect((await ui.find({ type: 'Text' }))?.text).toBe('rex 0gaq820001 planned · 0/2 agents · $0.01/$10.00 · depth 2 · 1 question · 1 permission prompt')
    await ui.unmount()
  }
})

test('the band stays empty while no job is open', async ($, on) => {
  on('ui.render', ($h, e) => {
    const { Text } = $h.ui.resolve(e)
    return <Text>engine</Text>
  })
  world(on)
  await $.session.start(START)

  const ui = await $.ui.mount({ ...BAND, surface: 'terminal' })
  expect((await ui.find({ type: 'Text' }))?.text).toBe('engine')
  await ui.unmount()
})

async function texts(ui: { readonly findAll: (query: { readonly type: string }) => Promise<readonly { readonly text: string }[]> }): Promise<string[]> {
  return (await ui.findAll({ type: 'Text' })).map(found => found.text)
}

const VERSIONS = 'mod 0.8.0 · binary 0.8.0 · Claude Code 2.1.323'
const HEADER = ['SuperRex', 'storage off · prompts observe · depth 2 · 1 jobs · 1 questions', VERSIONS]

test('each pane view draws its rows from the book', async ($, on) => {
  const session = world(on, { seed: watchedBook() })
  await $.session.start(START)
  expect(session.versionCalls.map(call => call.argv)).toEqual([sharedArgv('fixtures/shared/calls/version.json')])

  for (const surface of SURFACES) {
    const ui = await $.ui.mount({ ...PANE, surface })
    expect(await texts(ui)).toEqual([...HEADER, '0gaq820001 planned · lead-and-workers · agents 2/2 · $0.01/$10.00 · 1 questions'])

    await ui.press({ key: 'view-tree' })
    expect(await texts(ui)).toEqual([
      ...HEADER,
      'super-rex:model-high-effort-high a1 · engineer · planned model-high-effort-high · ran claude-opus-5-5/high · 1500 tokens · $0.01 · open',
      '  super-rex:model-low-effort-low a2 · no role · planned none · ran not run · 0 tokens · $0.00 · open',
    ])

    await ui.press({ key: 'view-decisions' })
    expect(await texts(ui)).toEqual([...HEADER, '0gaq820001 task:T1 skill=rex-code-tdd · rule: the task writes code'])

    await ui.press({ key: 'view-questions' })
    expect(await texts(ui)).toEqual([...HEADER, 'q1-000000000001: Merge job 0gaq820001?', 'recommended: Merge'])
    expect((await ui.findAll({ type: 'Button' })).map(button => button.key)).toEqual(['view-jobs', 'view-tree', 'view-decisions', 'view-questions', 'answer-q1-000000000001-0', 'answer-q1-000000000001-1'])

    await ui.press({ key: 'view-jobs' })
    await ui.unmount()
  }
})

test('a press on an answer in the pane answers the question and sets the merge answer', async ($, on) => {
  const session = world(on, { seed: watchedBook() })
  await $.session.start(START)

  const ui = await $.ui.mount({ ...PANE, surface: 'terminal' })
  await ui.press({ key: 'view-questions' })
  await ui.press({ key: 'answer-q1-000000000001-0' })

  expect(session.book().questions[0]?.answer).toMatchObject({ option: 'Merge', source: 'pane' })
  expect(session.book().jobs[0]?.merge).toMatchObject({ answer: 'yes', source: 'question', question: 'q1-000000000001' })
  expect(await texts(ui)).toEqual(['SuperRex', 'storage off · prompts observe · depth 2 · 1 jobs · 0 questions', VERSIONS, 'q1-000000000001: Merge job 0gaq820001? answered Merge (pane)'])
  await ui.unmount()
})

test('the header says the binary version is unknown when rex --version fails', async ($, on) => {
  world(on, { seed: watchedBook(), binaryVersion: null })
  await $.session.start(START)

  const ui = await $.ui.mount({ ...PANE, surface: 'terminal' })
  expect((await texts(ui))[2]).toBe('mod 0.8.0 · binary unknown · Claude Code 2.1.323')
  await ui.unmount()
})

test('a pane view from another module is drawn as a tab with the pane context', async ($, on) => {
  const views = [HEADER_VIEW, JOBS_VIEW, TREE_VIEW, DECISIONS_VIEW, QUESTIONS_VIEW, PROBE_VIEW]
  const parts = createParts()
  for (const view of views) {
    parts.paneViews.add(view)
  }
  let chosen = 'jobs'
  const answered: string[] = []
  on('ui.render', { component: 'Pane', requestId: 'probe' }, ($h, e) => {
    const { Box, Text, Button } = $h.ui.resolve(e)
    const actions = { answer: (question: string, option: string): void => { answered.push(`${question}=${option}`) } }
    return drawPane({ elements: { Box, Text, Button }, book: watchedBook(), columns: e.props.bodyColumns, actions, views: parts.paneViews.entries(), chosen, choose: name => { chosen = name } })
  })

  const ui = await $.ui.mount({ ...PANE, plugin: 'test', requestId: 'probe', surface: 'terminal' })
  expect((await ui.findAll({ type: 'Button' })).map(button => button.key)).toEqual(['view-jobs', 'view-tree', 'view-decisions', 'view-probe', 'view-questions'])
  await ui.press({ key: 'view-probe' })
  await ui.redraw()
  expect(await texts(ui)).toEqual(['SuperRex', 'storage off · prompts observe · depth 2 · 1 jobs · 1 questions', 'mod unknown · binary unknown · Claude Code unknown', 'probe sees 1 jobs at 120 columns'])
  await ui.press({ key: 'probe-answer' })
  expect(answered).toEqual(['q1=Merge'])
  await ui.unmount()
})

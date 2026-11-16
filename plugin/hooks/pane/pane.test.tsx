// Proves the band and the watch pane: the band's short form of the outline, the pane at each level with its breadcrumb, rail and keys, a press down and up, pages of rows past nine, the fallback, and an answer given by a press.
import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { emptyBook } from '../book/events'
import { decisionIn, openJobIn, planJobIn, questionIn } from '../book/jobs'
import { ROOT, SESSION, START_MS, world } from '../book/kit'
import { startRunIn, stepRunIn } from '../book/runs'
import { sharedJson } from '../book/shared'
import { createParts } from '../parts'
import { liveBook, WRITTEN_AT } from '../live/live-book.fixture'
import { DECISIONS_VIEW } from './decisions'
import { drawPane } from './draw'
import { crumbLabels, TREE_VIEW } from './drill'
import { outlineOf, pathTo } from './outline'
import { PROBE_VIEW } from './probe-view.fixture'
import { HEADER_VIEW, QUESTIONS_VIEW } from './views'

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

/** The fixture's book in the kit's session, its times moved so that the kit's clock reads the fixture's `written_at`. */
function fixtureBook(): RexBook {
  return liveBook(SESSION, START_MS - WRITTEN_AT)
}

/** The fixture's book with ten more delivered jobs, so the session has twelve rows, more than the digits reach. */
function wideBook(): RexBook {
  const book = fixtureBook()
  const solo = book.jobs.find(job => job.id === '0gegkccde2')
  if (solo === undefined) {
    throw new Error('the fixture book lost its job with no tree')
  }
  return { ...book, jobs: [...book.jobs, ...Array.from({ length: 10 }, (_, index) => ({ ...solo, id: `0gwide${String(index).padStart(4, '0')}` }))] }
}

type Found = { readonly text: string; readonly key?: string | undefined; readonly props: Record<string, unknown> }
type Mounted = {
  readonly findAll: (query: { readonly type: string }) => Promise<readonly Found[]>
}

async function texts(ui: Mounted): Promise<string[]> {
  return (await ui.findAll({ type: 'Text' })).map(found => found.text)
}

async function keys(ui: Mounted): Promise<string[]> {
  return (await ui.findAll({ type: 'Button' })).map(found => found.key ?? '')
}

/** Each Button that has a hotkey, as `hotkey=key`. */
async function hotkeys(ui: Mounted): Promise<string[]> {
  return (await ui.findAll({ type: 'Button' })).flatMap(found => (typeof found.props['hotkey'] === 'string' ? [`${found.props['hotkey']}=${found.key ?? ''}`] : []))
}

async function bandTexts($: Engine, maxRows: number, bodyColumns = 160): Promise<string[]> {
  const ui = await $.ui.mount({ ...BAND, surface: 'terminal', props: { ...BAND.props, maxRows, bodyColumns, scroll: { offset: 0, bodyRows: maxRows } } })
  const shown = await texts(ui)
  await ui.unmount()
  return shown
}

const HEADER = ['SuperRex', 'storage off · prompts off · 2 jobs · deepest level helper · 1 question waits', 'mod 0.9.0 · binary 0.8.0 · Claude Code 2.1.325']
const TREE_NODE = 'session/tree:t0geh4lh3i'
const JOB_NODE = `${TREE_NODE}/job:0geh5fafkb`
const LEAD_NODE = `${JOB_NODE}/agent:alead01`
const WORKER_NODE = `${LEAD_NODE}/agent:aworker01`
const RAIL = /^(level \d|the session)/

test('the band of a tree: the tree, each open job and what its builder does, and the oldest question', async ($, on) => {
  world(on, { seed: fixtureBook() })
  await $.session.start(START)

  expect(await bandTexts($, 5)).toEqual([
    'rex master t0geh4lh3i running · 0/1 delivered · 2 agents open, deepest worker · $3.65/$150.00',
    '  builder 0geh5fafkb planned · leads engineer-lead · 1 worker open · $3.25/$28.00',
    '? 1 question waits on you · Merge job 0geh5fafkb (rex/0geh5fafkb) to main of /home/ana/code/shop? · /rex answer q1-553d90ce69d1 <1-2>',
  ])
  expect(await bandTexts($, 2)).toEqual([
    'rex master t0geh4lh3i running · 0/1 delivered · 2 agents open, deepest worker · $3.65/$150.00',
    '? 1 question waits on you · Merge job 0geh5fafkb (rex/0geh5fafkb) to main of /home/ana/code/shop? · /rex answer q1-553d90ce69d1 <1-2>',
  ])
  expect(await bandTexts($, 1, 60)).toEqual(['rex master t0geh4lh3i running · 0/1 delivered · 2 agents op…'])
})

test('the band of a job with no tree names the deepest level, and a question line follows', async ($, on) => {
  world(on, { seed: watchedBook() })
  on('tool.check', (_$, e) => ({ decision: e.tool === 'Bash' ? 'ask' : 'allow' }))
  await $.session.start(START)
  await $.tool.check({ tool: 'Bash', input: { command: 'rm -rf build' }, agentId: 'a1' })

  for (const surface of SURFACES) {
    const ui = await $.ui.mount({ ...BAND, surface, props: { ...BAND.props, bodyColumns: 120 } })
    expect(await texts(ui)).toEqual([
      'rex builder 0gaq820001 planned · 0/2 agents done · deepest worker · $0.01/$10.00 · 1 permission prompt',
      '? 1 question waits on you · Merge job 0gaq820001? · /rex answer q1-000000000001 <1-2>',
    ])
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

test('the pane goes one level down at each press, always with its breadcrumb, its level rail and a digit on each row, and one up again', async ($, on) => {
  const session = world(on, { seed: fixtureBook() })
  await $.session.start(START)
  expect(session.versionCalls.map(call => call.argv)).toEqual([sharedArgv('fixtures/shared/calls/version.json')])

  const ui = await $.ui.mount({ ...PANE, surface: 'terminal' })
  const seen: { readonly rail: string | undefined; readonly crumbs: number; readonly keys: string[] }[] = []
  const look = async (): Promise<void> => {
    const crumbs = (await keys(ui)).filter(key => key.startsWith('crumb:')).length
    seen.push({ rail: (await texts(ui)).find(text => RAIL.test(text)), crumbs, keys: (await hotkeys(ui)).filter(key => !key.includes('=view-')) })
  }
  await look()
  for (const node of [TREE_NODE, JOB_NODE, LEAD_NODE, WORKER_NODE]) {
    await ui.press({ key: `go:${node}` })
    await look()
  }
  await ui.press({ key: 'up' })
  await look()
  await ui.press({ key: 'crumb:session' })
  await look()

  expect(seen).toEqual([
    { rail: 'the session, above every level · master › builder › lead › worker › helper', crumbs: 0, keys: [`1=go:${TREE_NODE}`, '2=go:session/job:0gegkccde2'] },
    { rail: 'level 1 of 5 · [master] › builder › lead › worker › helper', crumbs: 1, keys: ['u=up', `1=go:${JOB_NODE}`] },
    { rail: 'level 2 of 5 · master › [builder] › lead › worker › helper', crumbs: 2, keys: ['u=up', `1=go:${JOB_NODE}/agent:adecision01`, `2=go:${LEAD_NODE}`] },
    { rail: 'level 3 of 5 · master › builder › [lead] › worker › helper', crumbs: 3, keys: ['u=up', `1=go:${WORKER_NODE}`] },
    { rail: 'level 4 of 5 · master › builder › lead › [worker] › helper', crumbs: 4, keys: ['u=up', `1=go:${WORKER_NODE}/agent:ahelper01`] },
    { rail: 'level 3 of 5 · master › builder › [lead] › worker › helper', crumbs: 3, keys: ['u=up', `1=go:${WORKER_NODE}`] },
    { rail: 'the session, above every level · master › builder › lead › worker › helper', crumbs: 0, keys: [`1=go:${TREE_NODE}`, '2=go:session/job:0gegkccde2'] },
  ])
  await ui.unmount()
})

test('a job with no tree has three levels on its rail, and an agent row leads with its role', async ($, on) => {
  world(on, { seed: fixtureBook() })
  await $.session.start(START)

  const ui = await $.ui.mount({ ...PANE, surface: 'terminal' })
  await ui.press({ key: 'go:session/job:0gegkccde2' })
  const shown = await texts(ui)
  const rows = (await ui.findAll({ type: 'Button' })).filter(found => found.key?.startsWith('go:') === true).map(found => found.text)

  expect(shown.find(text => RAIL.test(text))).toBe('level 1 of 3 · [builder] › lead › worker')
  expect(rows).toEqual(['✓ lead engineer T1 · done (answer) · $0.20 · 44s'])
  // A short inline pane shows its top first, so the way down stands above the facts.
  const drawn = JSON.stringify(await ui.drawn())
  expect(drawn.indexOf('go:session/job:0gegkccde2/agent:asolo01')).toBeLessThan(drawn.indexOf('status: delivered'))
  await ui.unmount()
})

test('rows past nine are reached nine at a time: n shows the next rows, each with a digit, and turns back to the first', async ($, on) => {
  world(on, { seed: wideBook() })
  await $.session.start(START)

  const ui = await $.ui.mount({ ...PANE, surface: 'terminal' })
  const rowKeys = async (): Promise<string[]> => (await hotkeys(ui)).filter(key => !key.includes('=view-'))
  const first = await rowKeys()
  await ui.press({ key: 'more' })
  const second = await rowKeys()
  await ui.press({ key: 'more' })

  expect(first).toHaveLength(10)
  expect(first.slice(-2)).toEqual(['9=go:session/job:0gwide0006', 'n=more'])
  expect(second).toEqual(['1=go:session/job:0gwide0007', '2=go:session/job:0gwide0008', '3=go:session/job:0gwide0009', 'n=more'])
  expect(await rowKeys()).toEqual(first)
  await ui.unmount()
})

test('the breadcrumb fits a narrow pane: the parts above the node in view shrink to their level', () => {
  const path = pathTo(outlineOf(fixtureBook(), START_MS), `${WORKER_NODE}/agent:ahelper01`)

  expect(crumbLabels(path, 200)).toEqual(['session', 'master t0geh4lh3i', 'builder 0geh5fafkb', 'lead engineer-lead', 'worker engineer T1', 'helper model-low-effort-low'])
  expect(crumbLabels(path, 80)).toEqual(['session', 'master', 'builder', 'lead', 'worker', 'helper model-low-effort-low'])
})

test('a node that is gone falls back to its nearest parent that exists', async ($, on) => {
  on('ui.render', { component: 'Pane', requestId: 'probe' }, ($h, e) => {
    const { Box, Text, Button } = $h.ui.resolve(e)
    const actions = { answer: (): void => undefined, go: (): void => undefined, turn: (): void => undefined }
    const views = [HEADER_VIEW, TREE_VIEW]
    return drawPane({ elements: { Box, Text, Button }, book: liveBook(), columns: e.props.bodyColumns, isFocused: true, actions, at: `${JOB_NODE}/agent:agone99`, page: 0, outline: outlineOf(liveBook(), WRITTEN_AT), views, chosen: 'tree', choose: () => undefined })
  })

  const ui = await $.ui.mount({ ...PANE, plugin: 'test', requestId: 'probe', surface: 'terminal' })
  expect((await keys(ui)).filter(key => key.startsWith('crumb:'))).toEqual([`crumb:session`, `crumb:${TREE_NODE}`])
  expect((await texts(ui)).find(text => RAIL.test(text))).toBe('level 2 of 5 · master › [builder] › lead › worker › helper')
  await ui.unmount()
})

test('the questions tab answers the oldest question with one digit and sets the merge answer', async ($, on) => {
  const session = world(on, { seed: fixtureBook() })
  await $.session.start(START)

  const ui = await $.ui.mount({ ...PANE, surface: 'terminal' })
  await ui.press({ key: 'view-questions' })
  expect((await texts(ui)).slice(HEADER.length)).toEqual([
    '? q1-553d90ce69d1: Merge job 0geh5fafkb (rex/0geh5fafkb) to main of /home/ana/code/shop?',
    'for job 0geh5fafkb · recommended: Merge · press a digit, or type /rex answer q1-553d90ce69d1 <1-2>',
    'q2-553d90ce69d1: The charter names no test command. Which one proves each step? answered cargo test and cargo clippy (command)',
  ])
  expect((await hotkeys(ui)).filter(key => key.includes('=answer-'))).toEqual(['1=answer-q1-553d90ce69d1-0', '2=answer-q1-553d90ce69d1-1'])
  await ui.press({ key: 'answer-q1-553d90ce69d1-0' })

  expect(session.book().questions[0]?.answer).toMatchObject({ option: 'Merge', source: 'pane' })
  expect(session.book().jobs[0]?.merge).toMatchObject({ answer: 'yes', source: 'question', question: 'q1-553d90ce69d1' })
  await ui.unmount()
})

test('the header says the binary version is unknown when rex --version fails', async ($, on) => {
  world(on, { seed: watchedBook(), binaryVersion: null })
  await $.session.start(START)

  const ui = await $.ui.mount({ ...PANE, surface: 'terminal' })
  expect((await texts(ui))[2]).toBe('mod 0.8.0 · binary unknown · Claude Code 2.1.323')
  await ui.unmount()
})

test('a pane view from another module is drawn as a tab with a letter key, and a pane without the keyboard says how to give it', async ($, on) => {
  const views = [HEADER_VIEW, TREE_VIEW, DECISIONS_VIEW, QUESTIONS_VIEW, PROBE_VIEW]
  const parts = createParts()
  for (const view of views) {
    parts.paneViews.add(view)
  }
  let chosen = 'tree'
  const answered: string[] = []
  on('ui.render', { component: 'Pane', requestId: 'probe' }, ($h, e) => {
    const { Box, Text, Button } = $h.ui.resolve(e)
    const actions = { answer: (question: string, option: string): void => { answered.push(`${question}=${option}`) }, go: (): void => undefined, turn: (): void => undefined }
    return drawPane({ elements: { Box, Text, Button }, book: watchedBook(), columns: e.props.bodyColumns, isFocused: false, actions, at: 'session', page: 0, outline: outlineOf(watchedBook(), START_MS), views: parts.paneViews.entries(), chosen, choose: name => { chosen = name } })
  })

  const ui = await $.ui.mount({ ...PANE, plugin: 'test', requestId: 'probe', surface: 'terminal' })
  expect((await hotkeys(ui)).filter(key => key.includes('=view-'))).toEqual(['t=view-tree', 'd=view-decisions', 'p=view-probe', 'q=view-questions'])
  await ui.press({ key: 'view-probe' })
  await ui.redraw()
  expect(await texts(ui)).toEqual([
    'SuperRex',
    'storage off · prompts observe · 1 job · deepest level worker · 1 question waits',
    'mod unknown · binary unknown · Claude Code unknown',
    'ctrl+x tab: give this pane the keys',
    'probe sees 1 jobs at 120 columns',
  ])
  await ui.press({ key: 'probe-answer' })
  expect(answered).toEqual(['q1=Merge'])
  await ui.unmount()
})

test('the decisions tab lists each recorded decision of each job', async ($, on) => {
  world(on, { seed: watchedBook() })
  await $.session.start(START)

  const ui = await $.ui.mount({ ...PANE, surface: 'terminal' })
  await ui.press({ key: 'view-decisions' })
  expect((await texts(ui)).slice(HEADER.length)).toEqual(['0gaq820001 task:T1 skill=rex-code-tdd · rule: the task writes code'])
  await ui.unmount()
})

// Proves the budget and the time a master tree opens with: two dialogs in order when a person is at the terminal, each tier or typed value landing in the tree, its answer lines and its brief; a dismissed or unreadable answer opens nothing; headless takes tree_usd and no time; a live tree asks nothing; a resume asks both again.
import { expect, test } from 'claude-code/testing'

import { ROOT, SESSION } from '../book/kit'
import { BUDGET_QUESTION, byRule, confirmedPolicy, HEADLESS_START, lineOf, masterWorld, openedTree, placesRules, POLICY_KEY, ran, resultText, START, TIME_QUESTION, TREE_OPEN } from './master-kit'
import type { MasterOptions } from './master-kit'
import { whenText } from './tree-limits'

const DOLLAR = 1_000_000_000
const MINUTE = 60_000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR
const CEILING_ENV = { CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH: '4', CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS: '0' }
const BUDGET_LABELS = ['Low ($10)', 'Medium ($250)', 'High ($5,000)', 'Extra high ($100,000)']
const TIME_LABELS = ['Short term (1 hour)', 'Medium term (8 hours)', 'Long term (3 days)', 'Unlimited']

async function world(on: Parameters<typeof masterWorld>[0], options: MasterOptions = {}) {
  return masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() }, ...options })
}

function limitsBlock(result: unknown): string | undefined {
  return /\n<limits>\n([\s\S]*?)\n<\/limits>\n/.exec(resultText(result))?.[1]
}

test('a new tree asks the budget, then the time, each with its four options and its chip', async ($, on) => {
  const session = await world(on)
  await $.session.start(START)

  await openedTree($, 'Ship the parser')

  expect(session.dialogs).toEqual([
    { question: BUDGET_QUESTION, header: 'Budget', labels: BUDGET_LABELS },
    { question: TIME_QUESTION, header: 'Time', labels: TIME_LABELS },
  ])
})

const BUDGETS = [...BUDGET_LABELS.map((label, index) => [label, [10, 250, 5_000, 100_000][index] ?? 0] as const), ['$1,500', 1_500] as const, ['750', 750] as const]

for (const [answer, usd] of BUDGETS) {
  test(`the budget answer ${answer} is the budget of the tree, in its answer lines and its limits block`, async ($, on) => {
    const session = await world(on, { budget: answer })
    await $.session.start(START)

    const opened = await openedTree($, 'Ship the parser')

    expect(session.book().trees[0]?.capNano).toBe(usd * DOLLAR)
    expect(lineOf(opened, 'budget_usd')).toBe(`${String(usd)}.00`)
    expect(limitsBlock(opened)).toBe(`budget_usd=${String(usd)}.00\ntime=unlimited\nends_at=none`)
  })
}

const TIMES = [
  ['Short term (1 hour)', HOUR, '1 hour'],
  ['Medium term (8 hours)', 8 * HOUR, '8 hours'],
  ['Long term (3 days)', 3 * DAY, '3 days'],
  ['90m', 90 * MINUTE, '90 minutes'],
  ['36h', 36 * HOUR, '36 hours'],
  ['2d', 2 * DAY, '2 days'],
] as const

for (const [answer, timeMs, words] of TIMES) {
  test(`the time answer ${answer} is the time of the tree, counted from the answer, in its answer lines and its limits block`, async ($, on) => {
    const session = await world(on, { time: answer })
    await $.session.start(START)
    await session.clock.advance(5 * MINUTE)
    const now = session.clock.now()

    const opened = await openedTree($, 'Ship the parser')

    const tree = session.book().trees[0]
    expect([tree?.timeMs, tree?.deadlineAt]).toEqual([timeMs, now + timeMs])
    expect([lineOf(opened, 'time'), lineOf(opened, 'ends_at')]).toEqual([words, whenText(now + timeMs)])
    expect(limitsBlock(opened)).toBe(`budget_usd=100000.00\ntime=${words}\nends_at=${whenText(now + timeMs)}`)
  })
}

test('the time answer Unlimited gives the tree no deadline', async ($, on) => {
  const session = await world(on, { time: 'Unlimited' })
  await $.session.start(START)

  const opened = await openedTree($, 'Ship the parser')

  expect([session.book().trees[0]?.timeMs, session.book().trees[0]?.deadlineAt]).toEqual([null, null])
  expect([lineOf(opened, 'time'), lineOf(opened, 'ends_at')]).toEqual(['unlimited', 'none'])
})

test('a dismissed budget dialog refuses, asks no time and opens nothing, and the next call asks again and opens the tree', async ($, on) => {
  const session = await world(on, { budget: null })
  await $.session.start(START)

  const dismissed = await openedTree($, 'Ship the parser')
  const asked = session.dialogs.map(each => each.question)
  const treesAfter = session.book().trees.length
  session.limitAnswers.budget = 'Medium ($250)'
  const again = await $.tool.call({ tool: TREE_OPEN })

  expect(dismissed.deny).toBe('rex_tree_open: the person chose no budget for the tree, got a dismissed question; call rex_tree_open again, and the person chooses one of low, medium, high, xhigh, or an amount in dollars such as 750')
  expect(asked).toEqual([BUDGET_QUESTION])
  expect(treesAfter).toBe(0)
  expect(session.dialogs.map(each => each.question)).toEqual([BUDGET_QUESTION, BUDGET_QUESTION, TIME_QUESTION])
  expect(lineOf(again, 'state')).toBe('running')
  expect(session.book().trees.map(tree => [tree.charter, tree.capNano])).toEqual([['Ship the parser', 250 * DOLLAR]])
})

test('a dismissed time dialog refuses and opens nothing, and the next call asks both again', async ($, on) => {
  const session = await world(on, { time: null })
  await $.session.start(START)

  const dismissed = await openedTree($, 'Ship the parser')
  const treesAfter = session.book().trees.length
  session.limitAnswers.time = '2d'
  const again = await $.tool.call({ tool: TREE_OPEN })

  expect(dismissed.deny).toBe('rex_tree_open: the person chose no time for the tree, got a dismissed question; call rex_tree_open again, and the person chooses one of short, medium, long, unlimited, or a span such as 90m, 36h or 2d')
  expect(treesAfter).toBe(0)
  expect(session.dialogs.map(each => each.question)).toEqual([BUDGET_QUESTION, TIME_QUESTION, BUDGET_QUESTION, TIME_QUESTION])
  expect(session.book().trees.map(tree => tree.timeMs)).toEqual([2 * DAY])
  expect(lineOf(again, 'time')).toBe('2 days')
})

test('an unreadable budget or time answer refuses with what the person typed, and opens nothing', async ($, on) => {
  const session = await world(on, { budget: 'lots' })
  await $.session.start(START)

  const budget = await openedTree($, 'Ship the parser')
  session.limitAnswers.budget = 'low'
  session.limitAnswers.time = 'soon'
  const time = await $.tool.call({ tool: TREE_OPEN })
  session.limitAnswers.budget = '0'
  const zero = await $.tool.call({ tool: TREE_OPEN })

  expect(budget.deny).toBe('rex_tree_open: the person chose no budget for the tree, got lots; call rex_tree_open again, and the person chooses one of low, medium, high, xhigh, or an amount in dollars such as 750')
  expect(time.deny).toBe('rex_tree_open: the person chose no time for the tree, got soon; call rex_tree_open again, and the person chooses one of short, medium, long, unlimited, or a span such as 90m, 36h or 2d')
  expect(zero.deny).toBe('rex_tree_open: the person chose no budget for the tree, got 0; call rex_tree_open again, and the person chooses one of low, medium, high, xhigh, or an amount in dollars such as 750')
  expect(session.book().trees).toEqual([])
})

test('a headless tree asks nothing, takes tree_usd of the policy as its budget, and has no time limit', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy({ allow_headless: true, tree_usd: 42 }) }, env: CEILING_ENV })
  await $.session.start(HEADLESS_START)

  const opened = await openedTree($, 'Ship the parser', { kind: 'sdk' })

  expect(session.dialogs).toEqual([])
  expect([session.book().trees[0]?.capNano, session.book().trees[0]?.timeMs, session.book().trees[0]?.deadlineAt]).toEqual([42 * DOLLAR, null, null])
  expect([lineOf(opened, 'budget_usd'), lineOf(opened, 'time'), lineOf(opened, 'ends_at'), lineOf(opened, 'person')]).toEqual(['42.00', 'unlimited', 'none', 'no'])
})

test('rex_tree_open again on the live tree of the session asks nothing and answers the limits it holds', async ($, on) => {
  const session = await world(on, { budget: 'High ($5,000)', time: '36h' })
  await $.session.start(START)

  await openedTree($, 'Ship the parser')
  session.limitAnswers.budget = 'low'
  const again = await $.tool.call({ tool: TREE_OPEN })

  expect(session.dialogs).toHaveLength(2)
  expect([lineOf(again, 'existing'), lineOf(again, 'budget_usd'), lineOf(again, 'time')]).toEqual(['yes', '5000.00', '36 hours'])
  expect(limitsBlock(again)?.startsWith('budget_usd=5000.00\ntime=36 hours\nends_at=')).toBe(true)
})

test('resume from the store asks the budget and the time again, and the restored tree holds the answers, not the stored cap', async ($, on) => {
  const policy = await confirmedPolicy({ tree_usd: 400 })
  const shown = { tree: { id: 't1', session_id: 'old', master_agent_id: null, charter: 'Ship the parser.', charter_hash: 'sha256:c', policy_hash: policy.confirmed_hash, state: 'running', cap_usd: 150, report_file: null, started_at: '2026-11-13T19:00:00.000Z', closed_at: null }, jobs: [], questions: [] }
  const session = masterWorld(on, { store: { [POLICY_KEY]: policy }, budget: 'Medium ($250)', time: 'Long term (3 days)', processAnswer: byRule([{ starts: ['rex', 'tree', 'show'], answer: ran(`${JSON.stringify(shown)}\n`) }, ...placesRules('origin')]) })
  await $.session.start(START)
  const now = session.clock.now()

  const resumed = await openedTree($, 'resume')

  const tree = session.book().trees.find(each => each.id === 't1')
  expect(session.dialogs.map(each => each.question)).toEqual([BUDGET_QUESTION, TIME_QUESTION])
  expect([tree?.session, tree?.capNano, tree?.timeMs, tree?.deadlineAt]).toEqual([SESSION, 250 * DOLLAR, 3 * DAY, now + 3 * DAY])
  expect([lineOf(resumed, 'resumed'), lineOf(resumed, 'budget_usd'), lineOf(resumed, 'time')]).toEqual(['yes', '250.00', '3 days'])
  expect(resultText(resumed)).toContain(`\n<repo>\nrepo=${ROOT}\n`)
})

test('a resume whose budget dialog is dismissed restores nothing', async ($, on) => {
  const policy = await confirmedPolicy()
  const shown = { tree: { id: 't1', session_id: 'old', master_agent_id: null, charter: 'Ship the parser.', charter_hash: 'sha256:c', policy_hash: policy.confirmed_hash, state: 'running', cap_usd: 150, report_file: null, started_at: '2026-11-13T19:00:00.000Z', closed_at: null }, jobs: [], questions: [] }
  const session = masterWorld(on, { store: { [POLICY_KEY]: policy }, budget: null, processAnswer: byRule([{ starts: ['rex', 'tree', 'show'], answer: ran(`${JSON.stringify(shown)}\n`) }, ...placesRules('origin')]) })
  await $.session.start(START)

  const resumed = await openedTree($, 'resume')

  expect(resumed.deny).toMatch(/^rex_tree_open: the person chose no budget for the tree, got a dismissed question;/)
  expect(session.book().trees).toEqual([])
})

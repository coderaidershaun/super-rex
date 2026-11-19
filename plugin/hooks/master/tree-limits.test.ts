// Proves the pure rules of a master tree's budget and time: how an option, a tier name, a typed amount or span is read and what is refused, how amounts, spans and moments read, when a tree stands at a limit, the raise, the mod's question and its refusal, and the master's requests.
import { expect, test } from 'claude-code/testing'

import type { RexBook, RexQuestion, RexTree } from '../../types'
import { openJobIn } from '../book/jobs'
import { ROOT, START_MS } from '../book/kit'
import { treeIn } from '../book/trees'
import { sessionBook } from './master-kit'
import {
  allowedValue,
  allowOption,
  budgetOf,
  isTreeLimitQuestion,
  limitCommands,
  limitQuestionText,
  moneyText,
  raisedCapNano,
  requestKind,
  requestText,
  spanText,
  timeOf,
  treeLimitRefusal,
  treeOver,
  whenText,
} from './tree-limits'

const DOLLAR = 1_000_000_000
const MINUTE = 60_000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR
const DEADLINE = START_MS + HOUR

/** A book with tree t1 of a budget of $10 and a time of one hour from START_MS, which has spent `ledger` itself and `job` in one job of it. */
function treeAt(ledger: number, job = 0, limits: Partial<Pick<RexTree, 'timeMs' | 'deadlineAt'>> = {}): { readonly book: RexBook; readonly tree: RexTree } {
  const opened = treeIn(sessionBook(), { id: 't1', charter: 'c', charterHash: 'h', policyHash: 'p', state: 'running', capNano: 10 * DOLLAR, timeMs: HOUR, deadlineAt: DEADLINE, ...limits }, START_MS)
  const input = {
    builder: 'b1',
    tree: 't1',
    origin: 'master',
    request: 'Fix the lexer',
    repo: ROOT,
    base: 'main',
    type: 'fix',
    size: 'small',
    consequence: null,
    langs: ['rust'],
    prefs: { duration: null, budget: null, autonomy: 'high' },
    limit: { nano: 100 * DOLLAR, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  const withJob = openJobIn(opened, input, '0gaq800001', START_MS).book
  const book = { ...withJob, trees: withJob.trees.map(each => ({ ...each, ledgerNano: ledger })), jobs: withJob.jobs.map(each => ({ ...each, spentNano: job })) }
  const tree = book.trees[0]
  if (tree === undefined) {
    throw new Error('no tree')
  }
  return { book, tree }
}

function question(keys: Partial<RexQuestion>): RexQuestion {
  return { id: 'q1', tree: 't1', job: null, question: 'Q?', options: ['Raise it', 'Stop the tree'], recommendation: 'Stop the tree', category: 'spend', mergeFor: null, askedAt: START_MS, answer: null, ...keys }
}

test('budgetOf reads each option of the budget dialog, each tier name and label in any case, and a typed amount in dollars', () => {
  const read = ['Low ($10)', 'Medium ($250)', 'High ($5,000)', 'Extra high ($100,000)', 'low', 'MEDIUM', 'high', 'xhigh', 'Extra high', ' 750 ', '$1,500', '12.50', '9000000'].map(budgetOf)

  expect(read).toEqual([10, 250, 5_000, 100_000, 10, 250, 5_000, 100_000, 100_000, 750, 1_500, 12.5, 9_000_000].map(usd => usd * DOLLAR))
})

test('budgetOf refuses nothing, a negative amount, words, and more than $9,000,000', () => {
  expect(['0', '$0', '-5', 'lots', 'ten dollars', '', '9000000.01', '$9,000,001', '1e6'].map(budgetOf)).toEqual([null, null, null, null, null, null, null, null, null])
})

test('timeOf reads each option of the time dialog, each tier name, the words for no limit, and a typed span; a number alone is hours', () => {
  const read = ['Short term (1 hour)', 'Medium term (8 hours)', 'Long term (3 days)', 'Unlimited', 'short', 'medium', 'LONG', 'unlimited', 'none', 'no limit', 'forever', '90m', '36h', '2d', '2 days', '1w', '3', '1.5h', '365d'].map(timeOf)

  expect(read).toEqual([HOUR, 8 * HOUR, 3 * DAY, null, HOUR, 8 * HOUR, 3 * DAY, null, null, null, null, 90 * MINUTE, 36 * HOUR, 2 * DAY, 2 * DAY, 7 * DAY, 3 * HOUR, 90 * MINUTE, 365 * DAY].map(timeMs => ({ timeMs })))
})

test('timeOf refuses nothing, a negative span, words, less than a minute and more than a year', () => {
  expect(['0', '0h', '-1h', 'soon', 'a while', '', '30s', '0.5m', '366d', '53w', '1 year'].map(timeOf)).toEqual([null, null, null, null, null, null, null, null, null, null, null])
})

test('moneyText groups the dollars and shows cents only when there are some', () => {
  expect([0, 10 * DOLLAR, 12.5 * DOLLAR, 10.02 * DOLLAR, 100_000 * DOLLAR, 1_234_567.89 * DOLLAR].map(moneyText)).toEqual(['$0', '$10', '$12.50', '$10.02', '$100,000', '$1,234,567.89'])
})

test('spanText reads whole days, then whole hours, then minutes, and no limit as unlimited', () => {
  expect([null, MINUTE, 90 * MINUTE, HOUR, 8 * HOUR, 36 * HOUR, DAY, 3 * DAY, 30_000].map(spanText)).toEqual(['unlimited', '1 minute', '90 minutes', '1 hour', '8 hours', '36 hours', '1 day', '3 days', '1 minute'])
})

test('whenText reads a moment in UTC to the minute', () => {
  expect(whenText(Date.UTC(2026, 10, 19, 14, 5, 42))).toBe('2026-11-19 14:05 UTC')
})

test('treeOver counts what the master and every job spent against the budget, at the exact amount, and the clock against the end of the time', () => {
  const under = treeAt(4 * DOLLAR, 5.99 * DOLLAR)
  const atBudget = treeAt(4 * DOLLAR, 6 * DOLLAR)
  const timed = treeAt(0)

  expect(treeOver(under.book, under.tree, START_MS)).toBeNull()
  expect(treeOver(atBudget.book, atBudget.tree, START_MS)).toEqual({ kind: 'budget', spentNano: 10 * DOLLAR, capNano: 10 * DOLLAR })
  expect(treeOver(timed.book, timed.tree, DEADLINE - 1)).toBeNull()
  expect(treeOver(timed.book, timed.tree, DEADLINE)).toEqual({ kind: 'time', timeMs: HOUR, deadlineAt: DEADLINE })
})

test('treeOver looks at the budget first when a tree stands at both, and a tree with no deadline never runs out of time', () => {
  const both = treeAt(10 * DOLLAR)
  const open = treeAt(0, 0, { timeMs: null, deadlineAt: null })

  expect(treeOver(both.book, both.tree, DEADLINE + DAY)?.kind).toBe('budget')
  expect(treeOver(open.book, open.tree, DEADLINE + 365 * DAY)).toBeNull()
})

test('raisedCapNano goes to the next budget of the dialog above, else doubles, and never past $9,000,000', () => {
  expect([0.5, 10, 250, 7_000, 5_000, 100_000, 5_000_000, 9_000_000].map(usd => raisedCapNano(usd * DOLLAR) / DOLLAR)).toEqual([10, 250, 5_000, 100_000, 100_000, 200_000, 9_000_000, 9_000_000])
})

test('limitQuestionText names the limit, so a raised budget or a later end gets a question of its own', () => {
  const { tree } = treeAt(0)

  expect(limitQuestionText(tree, { kind: 'budget', spentNano: 11 * DOLLAR, capNano: 10 * DOLLAR })).toBe('Master tree t1 has spent its budget of $10. Raise it to $250?')
  expect(limitQuestionText(tree, { kind: 'budget', spentNano: 300 * DOLLAR, capNano: 250 * DOLLAR })).toBe('Master tree t1 has spent its budget of $250. Raise it to $5,000?')
  expect(limitQuestionText(tree, { kind: 'time', timeMs: HOUR, deadlineAt: DEADLINE })).toBe(`Master tree t1 has run for its time of 1 hour, which ended at ${whenText(DEADLINE)}. Give it 1 hour more?`)
})

test("isTreeLimitQuestion is true only for the mod's spend question of a tree with no job and the two options it acts on", () => {
  const yes = [question({}), question({ options: ['Give it more time', 'Stop the tree'] })]
  const no = [
    question({ job: '0gaq800001' }),
    question({ tree: null }),
    question({ category: 'other' }),
    question({ options: ['Raise', 'Keep the limit'] }),
    question({ options: ['Stop the tree', 'Raise it'] }),
    question({ options: ['Raise it', 'Stop the tree', 'Wait'] }),
    question({ options: ['Raise it to $500', 'Keep the budget'] }),
  ]

  expect(yes.map(isTreeLimitQuestion)).toEqual([true, true])
  expect(no.map(isTreeLimitQuestion)).toEqual(no.map(() => false))
})

test('treeLimitRefusal is null under both limits, and names the limit and the command that raises it at either', () => {
  const under = treeAt(0)
  const spent = treeAt(10 * DOLLAR)
  const end = "nothing new starts until the person raises it (they answer the mod's question, or type"

  expect(treeLimitRefusal(under.book, under.tree, START_MS)).toBeNull()
  expect(treeLimitRefusal(spent.book, spent.tree, START_MS)).toBe(`rex gate: master tree t1 has spent $10 of its budget of $10; ${end} /rex budget <usd>) or stops the tree; end your turn, and the mod wakes you`)
  expect(treeLimitRefusal(under.book, under.tree, DEADLINE)).toBe(`rex gate: master tree t1 has run for its time of 1 hour, which ended at ${whenText(DEADLINE)}; ${end} /rex time <span>) or stops the tree; end your turn, and the mod wakes you`)
})

test("allowOption names the value a master's request allows, and requestText says who asks, from what to what, and why on one line", () => {
  expect([allowOption('budget', 500 * DOLLAR), allowOption('job', 20 * DOLLAR), allowOption('time', 8 * HOUR), allowOption('time', null)]).toEqual(['Raise it to $500', 'Raise it to $20', 'Give it 8 hours', 'Give it unlimited'])
  expect(requestText('t1', 'budget', '$250', '$500', '  two more\n jobs   are needed ', null)).toBe('The master of tree t1 asks you to raise its budget from $250 to $500. Its reason: two more jobs are needed')
  expect(requestText('t1', 'time', 'unlimited', '8 hours counted from your answer', 'x', null)).toBe('The master of tree t1 asks you to change its time from unlimited to 8 hours counted from your answer. Its reason: x')
  expect(requestText('t1', 'job', '$10', '$20', 'x', '0gaq800001')).toBe('The master of tree t1 asks you to raise the spend limit of job 0gaq800001 from $10 to $20. Its reason: x')
})

test("requestKind knows a master's request by its text, its keeping option and its job, and allowedValue reads the value only from the allowing option", () => {
  const text = 'The master of tree t1 asks you to raise its budget from $250 to $500. Its reason: x'
  const budget = question({ question: text, options: ['Raise it to $500', 'Keep the budget'] })
  const time = question({ question: text, options: ['Give it 8 hours', 'Keep the time'] })
  const forever = question({ question: text, options: ['Give it unlimited', 'Keep the time'] })
  const job = question({ question: text, job: '0gaq800001', options: ['Raise it to $20', 'Keep the limit'] })

  expect([budget, time, job].map(requestKind)).toEqual(['budget', 'time', 'job'])
  expect([
    question({ question: text, options: ['Raise it to $20', 'Keep the limit'] }),
    question({ question: text, job: '0gaq800001', options: ['Raise it to $500', 'Keep the budget'] }),
    question({ options: ['Raise it to $500', 'Keep the budget'] }),
    question({ question: text, category: 'scope', options: ['Raise it to $500', 'Keep the budget'] }),
    question({ question: text, options: ['Raise it to $500', 'Keep the budget', 'Wait'] }),
  ].map(requestKind)).toEqual([null, null, null, null, null])
  expect([allowedValue(budget, 'Raise it to $500'), allowedValue(time, 'Give it 8 hours'), allowedValue(forever, 'Give it unlimited'), allowedValue(job, 'Raise it to $20')]).toEqual([{ nano: 500 * DOLLAR }, { timeMs: 8 * HOUR }, { timeMs: null }, { nano: 20 * DOLLAR }])
  expect([allowedValue(budget, 'Keep the budget'), allowedValue(time, 'Keep the time'), allowedValue(question({}), 'Raise it')]).toEqual([null, null, null])
})

test('limitCommands names the question by its id, the next budget, and every command that sets a limit', () => {
  const { tree } = treeAt(10 * DOLLAR)

  const lines = limitCommands(tree, { kind: 'budget', spentNano: 10 * DOLLAR, capNano: 10 * DOLLAR }, 'q7')
  const timed = limitCommands(tree, { kind: 'time', timeMs: HOUR, deadlineAt: DEADLINE }, 'q8')

  expect(lines.slice(0, 3)).toEqual(['rex: master tree t1 has spent $10 of its budget of $10. Nothing new starts. What you can type:', '  /rex answer q7 1      raise the budget to $250', '  /rex answer q7 2      stop the tree'])
  expect(lines.slice(3).map(line => line.trim().split(' ')[0])).toEqual(['/rex', '/rex', '/rex', '/rex', 'You'])
  expect(lines[3]).toContain('low ($10), medium ($250), high ($5,000), xhigh ($100,000)')
  expect(timed[1]).toBe('  /rex answer q8 1      give the tree 1 hour more')
})

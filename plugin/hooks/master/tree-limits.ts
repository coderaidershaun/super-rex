// The budget and the time of a master tree as pure decisions: the options the person chooses from when a tree opens, how a typed amount or span is read, whether a tree has reached one of the two, and the one question the mod asks then.
import type { RexBook, RexQuestion, RexTree } from '../../types'
import { treeSpentNano } from './gates'

/** Every amount in dollars stays at most this, so nanodollars stay safe integers. */
const MAX_USD = 9_000_000
const MINUTE_MS = 60_000
const HOUR_MS = 60 * MINUTE_MS
const DAY_MS = 24 * HOUR_MS
/** No tree is given less than a minute or more than a year. */
const MAX_TIME_MS = 365 * DAY_MS

export type BudgetTier = { readonly name: string; readonly label: string; readonly usd: number }
export type TimeTier = { readonly name: string; readonly label: string; readonly timeMs: number | null }

/** The four budgets of the question, lowest first. */
export const BUDGET_TIERS: readonly BudgetTier[] = [
  { name: 'low', label: 'Low', usd: 10 },
  { name: 'medium', label: 'Medium', usd: 250 },
  { name: 'high', label: 'High', usd: 5_000 },
  { name: 'xhigh', label: 'Extra high', usd: 100_000 },
]

/** The four times of the question, shortest first. */
export const TIME_TIERS: readonly TimeTier[] = [
  { name: 'short', label: 'Short term', timeMs: HOUR_MS },
  { name: 'medium', label: 'Medium term', timeMs: 8 * HOUR_MS },
  { name: 'long', label: 'Long term', timeMs: 3 * DAY_MS },
  { name: 'unlimited', label: 'Unlimited', timeMs: null },
]

export const BUDGET_QUESTION = 'How much may this master tree spend?'
export const TIME_QUESTION = 'How long may this master tree run?'

/** An amount as the person reads it: `$5,000`, or `$12.50` when it has cents. */
export function moneyText(nano: number): string {
  const usd = nano / 1e9
  const [whole = '0', cents = '00'] = usd.toFixed(2).split('.')
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ',')
  return cents === '00' ? `$${grouped}` : `$${grouped}.${cents}`
}

function counted(count: number, unit: string): string {
  return `${String(count)} ${unit}${count === 1 ? '' : 's'}`
}

/** A span as the person reads it: `3 days`, `8 hours`, `90 minutes`. */
export function spanText(timeMs: number | null): string {
  if (timeMs === null) {
    return 'unlimited'
  }
  if (timeMs % DAY_MS === 0) {
    return counted(timeMs / DAY_MS, 'day')
  }
  if (timeMs % HOUR_MS === 0) {
    return counted(timeMs / HOUR_MS, 'hour')
  }
  return counted(Math.max(1, Math.round(timeMs / MINUTE_MS)), 'minute')
}

/** A moment as the person reads it, in UTC: `2026-11-19 14:05 UTC`. */
export function whenText(at: number): string {
  return `${new Date(at).toISOString().slice(0, 16).replace('T', ' ')} UTC`
}

export function budgetLabel(tier: BudgetTier): string {
  return `${tier.label} (${moneyText(tier.usd * 1e9)})`
}

export function timeLabel(tier: TimeTier): string {
  return tier.timeMs === null ? tier.label : `${tier.label} (${spanText(tier.timeMs)})`
}

function sameWords(a: string, b: string): boolean {
  return a.trim().toLowerCase() === b.trim().toLowerCase()
}

/**
 * The budget a typed answer names, in nanodollars: an option of the question,
 * the name of one (`low`, `medium`, `high`, `xhigh`), or an amount in dollars
 * such as `750` or `$1,500`. Null when it names none.
 */
export function budgetOf(typed: string): number | null {
  const tier = BUDGET_TIERS.find(each => sameWords(typed, budgetLabel(each)) || sameWords(typed, each.name) || sameWords(typed, each.label))
  if (tier !== undefined) {
    return tier.usd * 1e9
  }
  const amount = typed.trim().replace(/^\$/, '').replace(/,/g, '')
  if (!/^\d+(\.\d+)?$/.test(amount)) {
    return null
  }
  const usd = Number(amount)
  return usd > 0 && usd <= MAX_USD ? Math.round(usd * 1e9) : null
}

const UNIT_MS: readonly (readonly [RegExp, number])[] = [
  [/^(m|min|mins|minute|minutes)$/, MINUTE_MS],
  [/^(h|hr|hrs|hour|hours)$/, HOUR_MS],
  [/^(d|day|days)$/, DAY_MS],
  [/^(w|week|weeks)$/, 7 * DAY_MS],
]

/**
 * The time a typed answer names: an option of the question, the name of one
 * (`short`, `medium`, `long`, `unlimited`), or a span such as `90m`, `36h`,
 * `2 days`; a number alone is hours. `{ timeMs: null }` is no limit; null
 * means the text names no time.
 */
export function timeOf(typed: string): { readonly timeMs: number | null } | null {
  const tier = TIME_TIERS.find(each => sameWords(typed, timeLabel(each)) || sameWords(typed, each.name) || sameWords(typed, each.label))
  if (tier !== undefined) {
    return { timeMs: tier.timeMs }
  }
  if (/^(none|no limit|forever)$/i.test(typed.trim())) {
    return { timeMs: null }
  }
  const span = /^(\d+(?:\.\d+)?)\s*([a-z]*)$/.exec(typed.trim().toLowerCase())
  const unit = span === null ? undefined : span[2] === '' ? HOUR_MS : UNIT_MS.find(([name]) => name.test(span[2] ?? ''))?.[1]
  if (span === null || unit === undefined) {
    return null
  }
  const timeMs = Math.round(Number(span[1]) * unit)
  return timeMs >= MINUTE_MS && timeMs <= MAX_TIME_MS ? { timeMs } : null
}

/** How a budget and a time may be typed, for a refusal. */
export const BUDGET_FORMS = `${BUDGET_TIERS.map(tier => tier.name).join(', ')}, or an amount in dollars such as 750`
export const TIME_FORMS = `${TIME_TIERS.map(tier => tier.name).join(', ')}, or a span such as 90m, 36h or 2d`

/** The limit a tree has reached: its budget, by what the master and every job spent, or its time. */
export type TreeOver =
  | { readonly kind: 'budget'; readonly spentNano: number; readonly capNano: number }
  | { readonly kind: 'time'; readonly timeMs: number; readonly deadlineAt: number }

/** The budget is looked at first: a tree over both is asked about its budget, then about its time. */
export function treeOver(book: RexBook, tree: RexTree, now: number): TreeOver | null {
  const spentNano = treeSpentNano(book, tree)
  if (spentNano >= tree.capNano) {
    return { kind: 'budget', spentNano, capNano: tree.capNano }
  }
  if (tree.deadlineAt !== null && tree.timeMs !== null && now >= tree.deadlineAt) {
    return { kind: 'time', timeMs: tree.timeMs, deadlineAt: tree.deadlineAt }
  }
  return null
}

/** What happened, to follow `master tree <id>`: `has spent $10.02 of its budget of $10`. */
export function overWords(over: TreeOver): string {
  return over.kind === 'budget'
    ? `has spent ${moneyText(over.spentNano)} of its budget of ${moneyText(over.capNano)}`
    : `has run for its time of ${spanText(over.timeMs)}, which ended at ${whenText(over.deadlineAt)}`
}

/** The budget after one raise: the next budget of the question above it, else double; never past what an amount may be. */
export function raisedCapNano(capNano: number): number {
  const next = BUDGET_TIERS.map(tier => tier.usd * 1e9).find(nano => nano > capNano)
  return Math.min(next ?? capNano * 2, MAX_USD * 1e9)
}

export const RAISE_BUDGET = 'Raise it'
export const MORE_TIME = 'Give it more time'
export const STOP_TREE = 'Stop the tree'

/** The option that lets the tree go on. */
export function goOnOption(over: TreeOver): string {
  return over.kind === 'budget' ? RAISE_BUDGET : MORE_TIME
}

/** The question for one limit: its text names the limit, so a raised budget or a later end gets a question of its own. */
export function limitQuestionText(tree: RexTree, over: TreeOver): string {
  return over.kind === 'budget'
    ? `Master tree ${tree.id} has spent its budget of ${moneyText(over.capNano)}. Raise it to ${moneyText(raisedCapNano(over.capNano))}?`
    : `Master tree ${tree.id} has run for its time of ${spanText(over.timeMs)}, which ended at ${whenText(over.deadlineAt)}. Give it ${spanText(over.timeMs)} more?`
}

/** True for the mod's question about a tree at its budget or its time, which carries the options the mod acts on. */
export function isTreeLimitQuestion(question: RexQuestion): boolean {
  const [first, second, ...rest] = question.options
  return question.category === 'spend' && question.job === null && question.tree !== null && (first === RAISE_BUDGET || first === MORE_TIME) && second === STOP_TREE && rest.length === 0
}

/** True while the question's own limit is the one its tree stands at now: an answer to it can still change something. */
export function isLimitStillReached(book: RexBook, question: RexQuestion, now: number): boolean {
  const tree = book.trees.find(each => each.id === question.tree && (each.state === 'running' || each.state === 'paused'))
  const over = tree === undefined ? null : treeOver(book, tree, now)
  return tree !== undefined && over !== null && limitQuestionText(tree, over) === question.question
}

/** Why rex_build starts no job and a job of the tree starts no agent, or null while the tree is under its budget and its time. */
export function treeLimitRefusal(book: RexBook, tree: RexTree, now: number): string | null {
  const over = treeOver(book, tree, now)
  if (over === null) {
    return null
  }
  const command = over.kind === 'budget' ? '/rex budget <usd>' : '/rex time <span>'
  return `rex gate: master tree ${tree.id} ${overWords(over)}; nothing new starts until the person raises it (they answer the mod's question, or type ${command}) or stops the tree; end your turn, and the mod wakes you`
}

/** What a master may ask the person to raise with rex_raise. */
export type RaiseKind = 'budget' | 'time' | 'job'

/** The option that leaves a limit as it is, for each kind of the master's request. */
export const KEEP_OPTION: Readonly<Record<RaiseKind, string>> = { budget: 'Keep the budget', time: 'Keep the time', job: 'Keep the limit' }

const REQUEST_START = 'The master of tree '
const RAISE_TO = 'Raise it to '
const GIVE_IT = 'Give it '

/** The option that allows the master's request. It names the new value, so the person reads what they allow and the mod reads what to set. */
export function allowOption(kind: RaiseKind, value: number | null): string {
  return kind === 'time' ? `${GIVE_IT}${spanText(value)}` : `${RAISE_TO}${moneyText(value ?? 0)}`
}

/** The question of one request of the master: who asks, what for, from what to what, and the master's reason as it gave it. */
export function requestText(tree: string, kind: RaiseKind, from: string, to: string, reason: string, job: string | null): string {
  const what = kind === 'budget' ? 'raise its budget' : kind === 'time' ? 'change its time' : `raise the spend limit of job ${job ?? ''}`
  return `${REQUEST_START}${tree} asks you to ${what} from ${from} to ${to}. Its reason: ${reason.replace(/\s+/g, ' ').trim()}`
}

/** The kind of a question that is a request of the master to raise a limit, or null for any other question. */
export function requestKind(question: RexQuestion): RaiseKind | null {
  const [, keep, ...rest] = question.options
  if (question.category !== 'spend' || question.tree === null || !question.question.startsWith(REQUEST_START) || rest.length > 0) {
    return null
  }
  const kind = (['budget', 'time', 'job'] as const).find(each => KEEP_OPTION[each] === keep) ?? null
  return kind === null || (kind === 'job') !== (question.job !== null) ? null : kind
}

/** What the allowing option of a request sets: nanodollars for a budget or a job's limit, a span for a time. Null when the option is not the allowing one. */
export function allowedValue(question: RexQuestion, option: string): { readonly nano: number } | { readonly timeMs: number | null } | null {
  const kind = requestKind(question)
  if (kind === null || option !== question.options[0]) {
    return null
  }
  if (kind === 'time') {
    return option.startsWith(GIVE_IT) ? timeOf(option.slice(GIVE_IT.length)) : null
  }
  const nano = option.startsWith(RAISE_TO) ? budgetOf(option.slice(RAISE_TO.length)) : null
  return nano === null ? null : { nano }
}

/**
 * The commands a person can type when a tree stands at a limit, one line
 * each, for the transcript: the two answers of the question, then the
 * commands that set any budget, time or job limit, and the policy.
 */
export function limitCommands(tree: RexTree, over: TreeOver, question: string): readonly string[] {
  const goOn = over.kind === 'budget' ? `raise the budget to ${moneyText(raisedCapNano(over.capNano))}` : `give the tree ${spanText(over.timeMs)} more`
  return [
    `rex: master tree ${tree.id} ${overWords(over)}. Nothing new starts. What you can type:`,
    `  /rex answer ${question} 1      ${goOn}`,
    `  /rex answer ${question} 2      stop the tree`,
    `  /rex budget <amount>        set the budget: ${BUDGET_TIERS.map(tier => `${tier.name} (${moneyText(tier.usd * 1e9)})`).join(', ')}, or dollars`,
    `  /rex time <span>            set how long the tree may run from now: ${TIME_TIERS.map(tier => tier.name).join(', ')}, or 90m, 36h, 2d`,
    '  /rex limit <job> <usd>      set the spend limit of one job',
    '  /rex policy show            see what the master may decide as you',
    '  You can also tell the master to raise it: it asks the mod with rex_raise, and you confirm in a dialog.',
  ]
}

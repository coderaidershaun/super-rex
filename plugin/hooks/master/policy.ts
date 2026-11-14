// What a master may decide in the person's place, kept in the plugin's own store outside every working folder, so no agent can change it, and pinned by its hash.
import type { RexLevel } from '../../types'

export type MergeRule = 'never' | 'ask' | 'allowed_if_checks_pass'
export type RaiseRule = 'never' | 'ask' | 'allowed'
export type Tier = 'low' | 'medium' | 'high' | 'xhigh'

export type RetrySchedule = {
  readonly first_minutes: number
  readonly first_tries: number
  readonly then_minutes: number
  readonly max_tries: number
}

/** The keys keep the spelling the person types with `/rex policy set`, so the stored policy and the command read alike. */
export type Policy = {
  readonly merge: MergeRule
  readonly merge_max_consequence: RexLevel
  readonly always_ask_paths: readonly string[]
  readonly consequence_ceiling: RexLevel
  readonly per_job_usd: number | null
  readonly tree_usd: number
  readonly raise_limits: RaiseRule
  readonly max_parallel_builders: number | 'auto'
  readonly pause_at_rate_limit_pct: number
  readonly max_jobs: number
  readonly repos: readonly string[]
  readonly allow_headless: boolean
  readonly master_model: string
  readonly tier_cap: Tier
  readonly on_dead_builder: 'ask' | 'continue'
  readonly watchdog_minutes: number
  readonly stuck_ticks: number
  readonly retry: RetrySchedule
  readonly required_allow: readonly string[]
  readonly merge_checks: readonly (readonly string[])[]
  readonly confirmed_hash: string | null
}

export const DEFAULT_POLICY: Policy = {
  merge: 'ask',
  merge_max_consequence: 'medium',
  always_ask_paths: ['contracts/**', 'infra/**', '**/*.env*'],
  consequence_ceiling: 'high',
  per_job_usd: null,
  tree_usd: 150,
  raise_limits: 'ask',
  max_parallel_builders: 'auto',
  pause_at_rate_limit_pct: 85,
  max_jobs: 8,
  repos: ['.'],
  allow_headless: false,
  master_model: 'opus',
  tier_cap: 'high',
  on_dead_builder: 'ask',
  watchdog_minutes: 30,
  stuck_ticks: 3,
  retry: { first_minutes: 2, first_tries: 3, then_minutes: 10, max_tries: 12 },
  required_allow: [],
  merge_checks: [],
  confirmed_hash: null,
}

export type PolicyKey = keyof Policy

export const POLICY_KEYS: readonly PolicyKey[] = Object.keys(DEFAULT_POLICY).filter((key): key is PolicyKey => key in DEFAULT_POLICY)

const LEVELS: readonly RexLevel[] = ['low', 'medium', 'high']
const TIERS: readonly Tier[] = ['low', 'medium', 'high', 'xhigh']
/** Every amount in dollars stays at most this, so nanodollars stay safe integers. */
const MAX_USD = 9_000_000

export function levelRank(level: RexLevel): number {
  return LEVELS.indexOf(level)
}

export function tierRank(tier: Tier): number {
  return TIERS.indexOf(tier)
}

export function isTier(value: string): value is Tier {
  return TIERS.some(tier => tier === value)
}

/** A stored or typed policy that is not the policy's shape; `key` names the first key found wrong. */
export class PolicyShapeError extends Error {
  readonly key: string
  readonly seen: string

  constructor(key: string, wanted: string, seen: unknown) {
    const seenText = JSON.stringify(seen) ?? String(seen)
    super(`policy key ${key} must be ${wanted}, got ${seenText}`)
    this.name = 'PolicyShapeError'
    this.key = key
    this.seen = seenText
  }
}

function fieldOf(value: object, key: string): unknown {
  return Reflect.get(value, key)
}

function oneOf<T extends string>(key: string, value: unknown, words: readonly T[]): T {
  const word = words.find(each => each === value)
  if (word === undefined) {
    throw new PolicyShapeError(key, `one of ${words.join(', ')}`, value)
  }
  return word
}

function wholeAtLeast(key: string, value: unknown, least: number): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < least) {
    throw new PolicyShapeError(key, `a whole number of at least ${String(least)}`, value)
  }
  return value
}

function dollars(key: string, value: unknown): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0 || value > MAX_USD) {
    throw new PolicyShapeError(key, `an amount from 0 to ${String(MAX_USD)} dollars`, value)
  }
  return value
}

function percent(key: string, value: unknown): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0 || value > 100) {
    throw new PolicyShapeError(key, 'a percent from 0 to 100', value)
  }
  return value
}

function textList(key: string, value: unknown): readonly string[] {
  if (!Array.isArray(value) || !value.every((item: unknown) => typeof item === 'string')) {
    throw new PolicyShapeError(key, 'a list of text', value)
  }
  return value.map(String)
}

function argvList(key: string, value: unknown): readonly (readonly string[])[] {
  const isArgvList = Array.isArray(value) && value.every((argv: unknown) => Array.isArray(argv) && argv.length > 0 && argv.every((word: unknown) => typeof word === 'string'))
  if (!isArgvList) {
    throw new PolicyShapeError(key, 'a list of commands, each a list of words such as ["cargo", "test"]', value)
  }
  return value.map((argv: readonly unknown[]) => argv.map(String))
}

function retryOf(value: unknown): RetrySchedule {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new PolicyShapeError('retry', 'an object with first_minutes, first_tries, then_minutes and max_tries', value)
  }
  return {
    first_minutes: wholeAtLeast('retry.first_minutes', fieldOf(value, 'first_minutes'), 1),
    first_tries: wholeAtLeast('retry.first_tries', fieldOf(value, 'first_tries'), 0),
    then_minutes: wholeAtLeast('retry.then_minutes', fieldOf(value, 'then_minutes'), 1),
    max_tries: wholeAtLeast('retry.max_tries', fieldOf(value, 'max_tries'), 1),
  }
}

/** The policy a stored or typed value holds; a key left out takes its default, a key of the wrong shape throws `PolicyShapeError`. */
export function policyOf(value: unknown): Policy {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new PolicyShapeError('policy', 'an object', value)
  }
  const unknownKey = Object.keys(value).find(key => !POLICY_KEYS.some(known => known === key))
  if (unknownKey !== undefined) {
    throw new PolicyShapeError(unknownKey, `one of the keys ${POLICY_KEYS.join(', ')}`, unknownKey)
  }
  const given = (key: PolicyKey): unknown => (Object.hasOwn(value, key) ? fieldOf(value, key) : DEFAULT_POLICY[key])
  const parallel = given('max_parallel_builders')
  const perJob = given('per_job_usd')
  const confirmed = given('confirmed_hash')
  const masterModel = given('master_model')
  const allowHeadless = given('allow_headless')
  if (typeof masterModel !== 'string' || masterModel === '') {
    throw new PolicyShapeError('master_model', 'a model name such as opus', masterModel)
  }
  if (typeof allowHeadless !== 'boolean') {
    throw new PolicyShapeError('allow_headless', 'true or false', allowHeadless)
  }
  if (confirmed !== null && typeof confirmed !== 'string') {
    throw new PolicyShapeError('confirmed_hash', 'a hash or null', confirmed)
  }
  return {
    merge: oneOf('merge', given('merge'), ['never', 'ask', 'allowed_if_checks_pass'] as const),
    merge_max_consequence: oneOf('merge_max_consequence', given('merge_max_consequence'), LEVELS),
    always_ask_paths: textList('always_ask_paths', given('always_ask_paths')),
    consequence_ceiling: oneOf('consequence_ceiling', given('consequence_ceiling'), LEVELS),
    per_job_usd: perJob === null ? null : dollars('per_job_usd', perJob),
    tree_usd: dollars('tree_usd', given('tree_usd')),
    raise_limits: oneOf('raise_limits', given('raise_limits'), ['never', 'ask', 'allowed'] as const),
    max_parallel_builders: parallel === 'auto' ? 'auto' : wholeAtLeast('max_parallel_builders', parallel, 1),
    pause_at_rate_limit_pct: percent('pause_at_rate_limit_pct', given('pause_at_rate_limit_pct')),
    max_jobs: wholeAtLeast('max_jobs', given('max_jobs'), 1),
    repos: textList('repos', given('repos')),
    allow_headless: allowHeadless,
    master_model: masterModel,
    tier_cap: oneOf('tier_cap', given('tier_cap'), TIERS),
    on_dead_builder: oneOf('on_dead_builder', given('on_dead_builder'), ['ask', 'continue'] as const),
    watchdog_minutes: wholeAtLeast('watchdog_minutes', given('watchdog_minutes'), 1),
    stuck_ticks: wholeAtLeast('stuck_ticks', given('stuck_ticks'), 1),
    retry: retryOf(given('retry')),
    required_allow: textList('required_allow', given('required_allow')),
    merge_checks: argvList('merge_checks', given('merge_checks')),
    confirmed_hash: confirmed,
  }
}

/** The JSON a value is hashed and stored as: object keys sorted at every depth, so one policy always gives one hash. */
export function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) {
    return `[${value.map(canonicalJson).join(',')}]`
  }
  if (typeof value === 'object' && value !== null) {
    const keys = Object.keys(value).sort()
    return `{${keys.map(key => `${JSON.stringify(key)}:${canonicalJson(fieldOf(value, key))}`).join(',')}}`
  }
  return JSON.stringify(value) ?? 'null'
}

/** What the hash covers: every key but `confirmed_hash`, which records the hash itself. */
export function hashedText(policy: Policy): string {
  const { confirmed_hash: _confirmed, ...rest } = policy
  return canonicalJson(rest)
}

/** The value `/rex policy set <key> <value>` stores: JSON when the text is JSON (a number, a list, null, true), else the text itself. */
export function typedValue(text: string): unknown {
  try {
    const parsed: unknown = JSON.parse(text)
    return parsed
  } catch (error) {
    if (error instanceof SyntaxError) {
      return text
    }
    throw error
  }
}

/** The policy with one key set as the person typed it; throws `PolicyShapeError` when the key or value does not fit. */
export function policyWith(policy: Policy, key: string, text: string): Policy {
  if (key === 'confirmed_hash') {
    throw new PolicyShapeError(key, 'set by /rex policy confirm only', text)
  }
  return policyOf({ ...policy, [key]: typedValue(text) })
}

/** One `key=value` line for each key, as `/rex policy show` prints them. */
export function policyLines(policy: Policy): readonly string[] {
  return POLICY_KEYS.filter(key => key !== 'confirmed_hash').map(key => `${key}=${canonicalJson(policy[key]).replace(/^"(.*)"$/, '$1')}`)
}

/** Builders that may run at once: a number, or by the size of the newest job when `auto`. */
export function parallelLimit(policy: Policy, size: 'small' | 'medium' | 'large' | null): number {
  if (policy.max_parallel_builders !== 'auto') {
    return policy.max_parallel_builders
  }
  switch (size ?? 'medium') {
    case 'large':
      return 1
    case 'medium':
      return 2
    case 'small':
      return 3
  }
}

/** True when a path of a diff matches a glob of `always_ask_paths`: `**` crosses folders, `*` and `?` stay inside one. */
export function matchesGlob(path: string, glob: string): boolean {
  let pattern = ''
  for (let index = 0; index < glob.length; index += 1) {
    const char = glob[index] ?? ''
    if (glob.startsWith('**/', index)) {
      pattern += '(?:.*/)?'
      index += 2
    } else if (glob.startsWith('**', index)) {
      pattern += '.*'
      index += 1
    } else if (char === '*') {
      pattern += '[^/]*'
    } else if (char === '?') {
      pattern += '[^/]'
    } else {
      pattern += char.replace(/[.+^${}()|[\]\\]/g, '\\$&')
    }
  }
  return new RegExp(`^${pattern}$`).test(path)
}

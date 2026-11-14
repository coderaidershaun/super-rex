// Proves the tables the mod keeps beside the binary equal the shared fixtures: the default limits, the list prices and the 44 cases of rex oms.
import { expect, test } from 'claude-code/testing'

import type { RexRating } from '../../types'
import { priceNano } from '../book/prices'
import { sharedJson } from '../book/shared'
import { jsonField } from './binary'
import { DEFAULT_LIMIT_NANO } from './limits'
import { omsAnswerOf, omsArgv } from './plan'

test('limit table equals the shared limits table', () => {
  const table = sharedJson('fixtures/shared/tables/limits.json')

  expect(jsonField(table, 'unit')).toBe('nanodollars')
  expect(DEFAULT_LIMIT_NANO).toEqual(jsonField(table, 'default_limit_nano'))
})

test('price table equals the shared prices table for every model and the second card', () => {
  const table = sharedJson('fixtures/shared/tables/prices.json')
  const models = jsonField(table, 'models')
  const names = typeof models === 'object' && models !== null ? Object.keys(models) : []
  const usage = { input_tokens: 1000, output_tokens: 100, cache_read_input_tokens: 10, cache_creation_input_tokens: 1 }
  const cost = (rates: unknown): number =>
    1000 * Number(jsonField(rates, 'input')) + 100 * Number(jsonField(rates, 'output')) + 10 * Number(jsonField(rates, 'cache_read')) + Number(jsonField(rates, 'cache_write_5m'))

  expect(names).toHaveLength(16)
  for (const model of names) {
    expect([model, priceNano({ ...usage, model })]).toEqual([model, cost(jsonField(models, model))])
  }
  const second = jsonField(jsonField(table, 'second_card'), 'claude-haiku-5-5')
  const above = Number(jsonField(second, 'above_prompt_tokens'))
  const big = { model: 'claude-haiku-5-5', input_tokens: above + 1, output_tokens: 100, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 }
  const rates = jsonField(second, 'rates')
  expect(priceNano(big)).toBe((above + 1) * Number(jsonField(rates, 'input')) + 100 * Number(jsonField(rates, 'output')))
})

const RATINGS: readonly RexRating[] = ['low', 'medium', 'high', 'none']

function rating(argv: readonly string[], flag: string): RexRating {
  const value = argv[argv.indexOf(flag) + 1]
  const found = RATINGS.find(each => each === value)
  if (found === undefined) {
    throw new Error(`case argv has no rating after ${flag}: ${argv.join(' ')}`)
  }
  return found
}

function level(argv: readonly string[], flag: string): 'low' | 'medium' | 'high' | null {
  const value = rating(argv, flag)
  return value === 'none' ? null : value
}

test('the 44 rex oms cases through --json: each argv the mod builds and each answer it reads', () => {
  const cases = jsonField(sharedJson('fixtures/shared/oms/cases.json'), 'cases')
  const list: readonly unknown[] = Array.isArray(cases) ? cases : []

  expect(list).toHaveLength(44)
  for (const one of list) {
    const argvValue = jsonField(one, 'argv')
    const argv = Array.isArray(argvValue) ? argvValue.map(String) : []
    const prefs = { duration: level(argv, '--duration'), budget: level(argv, '--budget'), autonomy: level(argv, '--autonomy') }
    const ratings = { intelligence: rating(argv, '--intelligence'), reliability: rating(argv, '--reliability'), consequence: rating(argv, '--consequence'), verification: rating(argv, '--verification') }
    const expected = jsonField(jsonField(one, 'expect'), 'stdout_json')
    const conflict = jsonField(expected, 'conflict')

    expect(omsArgv({ prefs, ratings })).toEqual(argv)
    expect(omsAnswerOf(`${JSON.stringify(expected)}\n`)).toEqual({ ok: true, agent: jsonField(expected, 'agent'), conflict: Array.isArray(conflict) ? conflict : [], version: 2 })
  }
})

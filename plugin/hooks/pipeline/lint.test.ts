// Proves that the phrase rules of the brief check skip quoted text between `<data>` tags, and still read every line outside them.
import { expect, test } from 'claude-code/testing'

import { lintFindings } from './lint'

function brief(context: string): string {
  return ['Task: T1', 'Outcome: one file.', 'Scope: src/.', `Context: ${context}`, 'Acceptance: `cargo test` passes.', 'Limits: none.'].join('\n')
}

test('never_inside_data_passes', () => {
  const quoted = brief('The request: <data>\nNever run git stash. Think hard. This is IMPORTANT.\n</data>')

  expect(lintFindings(quoted, null)).toEqual([])
})

test('never_outside_data_fails', () => {
  const plain = brief('<data>a request</data> Never touch docs/.')

  expect(lintFindings(plain, null)).toEqual(['"never" stands outside Limits:'])
})

test('unpaired_data_is_read_as_own_words', () => {
  const open = brief('<data>never closed')

  expect(lintFindings(open, null)).toEqual(['"never" stands outside Limits:', '<data> tags do not pair: 1 opened, 0 closed'])
})

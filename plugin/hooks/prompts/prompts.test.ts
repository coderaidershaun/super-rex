// Proves the prompt pipeline: a job-start prompt gets a restatement beside it, and the person's words reach the model byte for byte.
import { expect, test } from 'claude-code/testing'
import type { PromptOrigin, PromptSubmitInput } from 'claude-code'

import { ROOT, world } from '../book/kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const TEXT = '/super-rex:orchestration-builder Fix the parser'
const COMMAND = { name: 'super-rex:orchestration-builder', args: 'Fix the parser' }
const ANSWER = 'Goal: fix the parser. Missing: merge answer.'
const USAGE = { input_tokens: 10, output_tokens: 12, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 }

test('restatement_in_context_prompt_text_unchanged', async ($, on) => {
  const session = world(on, { modelAnswer: () => ({ isAnswered: true, text: ANSWER, usage: USAGE }) })
  const received: PromptSubmitInput[] = []
  on('prompt.submit', (_$, e) => {
    received.push(e)
    return { text: e.text }
  })

  await $.session.start(START)
  await $.prompt.submit({ text: TEXT, command: COMMAND, origin: { kind: 'composer' }, wait: false })

  expect(session.completions).toHaveLength(1)
  expect(session.completions[0]).toMatchObject({ model: 'sonnet', effort: 'low', prompt: 'Fix the parser' })
  expect(received).toHaveLength(1)
  expect(received[0]?.text).toBe(TEXT)
  expect(received[0]?.context).toEqual([
    `rex restatement of the request above (the person's words stay the request; this is one reading of them):\n${ANSWER}`,
  ])
  expect(session.statuses).toEqual(['rex: added restatement (merge answer)'])
  expect(session.book().jobStartPrompt).toMatchObject({ origin: 'composer', text: 'Fix the parser', restatement: ANSWER, restatementSkipped: null })
  expect(session.book().jobs).toEqual([])
})

test('prompt_of_other_origin_is_not_touched', async ($, on) => {
  const session = world(on, { modelAnswer: () => ({ isAnswered: true, text: ANSWER, usage: USAGE }) })
  const received: PromptSubmitInput[] = []
  on('prompt.submit', (_$, e) => {
    received.push(e)
    return { text: e.text }
  })
  const origins: readonly PromptOrigin[] = [{ kind: 'task-notification' }, { kind: 'peer' }, { kind: 'plugin', name: 'other' }]

  await $.session.start(START)
  const before = session.book()
  for (const origin of origins) {
    await $.prompt.submit({ text: TEXT, command: COMMAND, origin, wait: false })
  }

  expect(received.map(e => [e.text, e.context, e.origin])).toEqual(origins.map(origin => [TEXT, undefined, origin]))
  expect(session.completions).toEqual([])
  expect(session.book()).toEqual(before)
  expect(session.statuses).toEqual([])
})

test('a failed restatement lets the prompt pass and marks the job that opens next', async ($, on) => {
  const session = world(on, { modelAnswer: () => ({ isAnswered: false, reason: 'api-error', status: 529, error: 'overloaded', usage: USAGE }) })
  const received: PromptSubmitInput[] = []
  on('prompt.submit', (_$, e) => {
    received.push(e)
    return { text: e.text }
  })

  await $.session.start(START)
  await $.prompt.submit({ text: TEXT, command: COMMAND, origin: { kind: 'composer' }, wait: false })

  expect(received[0]?.context).toBeUndefined()
  expect(session.statuses).toEqual(['rex: restatement skipped (api-error 529 overloaded)'])
  expect(session.book().jobStartPrompt?.restatementSkipped).toBe('api-error 529 overloaded')
})

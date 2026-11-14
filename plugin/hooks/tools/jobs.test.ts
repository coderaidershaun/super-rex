// Proves the job tools: who may open a job and when, that open answers the caller's own job, that a plan is recorded once, and that close is safe to repeat.
import { expect, test } from 'claude-code/testing'
import type { PromptOrigin } from 'claude-code'

import { CONFIG_OFF, CONFIG_ON, ROOT, world } from '../book/kit'
import type { ProcessCall } from '../book/kit'
import { BASE_CALL, files, promptsPass, scripted } from './act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const RESTATED = { isAnswered: true, text: 'Goal: fix the parser. Missing: merge answer.', usage: { input_tokens: 1, output_tokens: 1, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 } } as const

function jobStart(origin: PromptOrigin, text = 'Fix the parser') {
  return { text: `/super-rex:orchestration-builder ${text}`, command: { name: 'super-rex:orchestration-builder', args: text }, origin, wait: false } as const
}

function lineOf(result: unknown, key: string): string | undefined {
  const text = typeof result === 'object' && result !== null ? Reflect.get(result, 'result') : undefined
  return typeof text === 'string' ? text.split('\n').find(line => line.startsWith(`${key}=`))?.slice(key.length + 1) : undefined
}

test('job_open_only_after_person_job_prompt', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted([BASE_CALL]), modelAnswer: () => RESTATED })
  await $.session.start(START)

  const early = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', request: 'x' })
  expect(early.deny).toBe(
    'rex_job_open: only a builder opens a job: the main loop right after your /orchestration-builder prompt, or a builder that rex_build started; you are main',
  )

  await $.prompt.submit(jobStart({ kind: 'composer' }))
  const opened = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', request: 'Fix the parser' })
  expect(lineOf(opened, 'job')).toMatch(/^0g[0-9a-z]{8}$/)
  expect(lineOf(opened, 'existing')).toBe('no')
  expect(lineOf(opened, 'merge')).toBe('unknown')
  expect(lineOf(opened, 'storage')).toBe('off')
  expect(lineOf(opened, 'limit_usd')).toBe('32.00')
  expect(lineOf(opened, 'branch')).toBe(`rex/${lineOf(opened, 'job') ?? ''}`)
  expect(lineOf(opened, 'base')).toBe('main')
  expect(lineOf(opened, 'worktree')).toBe(`${ROOT}.worktrees/${lineOf(opened, 'job') ?? ''}`)

  const again = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', request: 'Something else' })
  expect(lineOf(again, 'job')).toBe(lineOf(opened, 'job'))
  expect(lineOf(again, 'existing')).toBe('yes')

  const agent = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', request: 'x', agentId: 'a9' })
  expect(agent.deny).toMatch(/; you are agent$/)

  expect(session.book().jobs.map(job => [job.owner.session, job.owner.builder])).toEqual([[session.book().session, 'main']])
})

test('a job opened with no stated limit gets the default of its size and budget from the limits table', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const calls: ProcessCall[] = []
  world(on, { config: CONFIG_ON, processAnswer: scripted([BASE_CALL], calls), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(jobStart({ kind: 'composer' }))
  const opened = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', request: 'Fix the parser', size: 'small' })

  expect(lineOf(opened, 'limit_usd')).toBe('28.00')
  expect(lineOf(opened, 'storage')).toBe('on')
  expect(calls.map(call => call.argv)).toEqual([BASE_CALL.argv])
})

test('job_from_sdk_allowed_by_config', async ($, on) => {
  files(on, {})
  promptsPass(on)
  world(on, { config: CONFIG_OFF, processAnswer: scripted([BASE_CALL]), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(jobStart({ kind: 'sdk' }, 'Fix the parser. Set the spend limit of this job to 3 dollars'))
  const opened = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', request: 'Fix the parser' })

  expect(lineOf(opened, 'existing')).toBe('no')
  expect(lineOf(opened, 'limit_usd')).toBe('3.00')
})

test('a job-start prompt from sdk opens no job when the config turns jobs from sdk off', async ($, on) => {
  files(on, {})
  promptsPass(on)
  world(on, { config: CONFIG_OFF.replace('"jobs_from_sdk": true', '"jobs_from_sdk": false'), processAnswer: scripted([BASE_CALL]), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(jobStart({ kind: 'sdk' }))
  const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', request: 'Fix the parser' })

  expect(refused.deny).toMatch(/; you are main$/)
})

test('a bad argument changes nothing', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted([BASE_CALL]), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(jobStart({ kind: 'composer' }))
  const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', request: 'Fix the parser', size: 'huge' })

  expect(refused.deny).toBe('rex_job_open: size must be one of small, medium, large, got huge; call it again with one of them')
  expect(session.book().jobs).toEqual([])
})

// Proves the /rex router: its sub-commands, the authority check on the person's own typing, and the calls to the binary against their shared files.
import { expect, test } from 'claude-code/testing'
import type { CommandRunInput, PromptOrigin } from 'claude-code'

import { CONFIG_ON, PLUGIN_VERSION, ROOT, world } from '../book/kit'
import { sharedJson } from '../book/shared'
import { createParts } from '../parts'
import { rexLine, unknownLine } from './index'
import { registerProbe } from './probe-subcommand.fixture'
import { OWN_SUBCOMMANDS } from './router'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const PRESENTATION = { isFullscreen: false, columns: 120 } as const

function rex(args: string, origin: PromptOrigin = { kind: 'composer' }): CommandRunInput {
  return { command: 'rex', args, origin, presentation: PRESENTATION }
}

function fieldOf(value: unknown, key: string): unknown {
  return typeof value === 'object' && value !== null ? Reflect.get(value, key) : undefined
}

test('the rex command is registered at session start and opens the pane', async ($, on) => {
  const session = world(on)

  await $.session.start(START)
  const watched = await $.command.run(rex('watch'))

  expect(session.commands).toEqual(['rex'])
  expect(watched).toEqual({})
  expect(session.opened).toEqual(['rex'])
})

test('an unknown sub-command names the ones there are', async ($, on) => {
  world(on)

  await $.session.start(START)
  const answered = await $.command.run(rex('nope now'))

  expect(answered.text).toBe('rex: no sub-command nope; one of watch, status, jobs, doctor, prompts, limit')
})

test('prompts sets the mode from the person and refuses any other origin', async ($, on) => {
  const session = world(on)

  await $.session.start(START)
  const refused = await $.command.run(rex('prompts off', { kind: 'sdk' }))
  const bad = await $.command.run(rex('prompts loud'))
  const set = await $.command.run(rex('prompts observe'))

  expect(refused.text).toBe('rex prompts: only your own typing may run this, got origin sdk')
  expect(bad.text).toBe('rex prompts: loud is not a prompt mode, got "loud"; use off, observe or improve')
  expect(set.text).toBe('prompts=observe\n')
  expect(session.book().prompts).toBe('observe')
})

test('jobs reads the store through the call of its shared file', async ($, on) => {
  const call = sharedJson('fixtures/shared/calls/jobs.json')
  const stdout = JSON.stringify(fieldOf(fieldOf(call, 'expect'), 'stdout_json_contains'))
  const session = world(on, {
    config: CONFIG_ON,
    processAnswer: one => (one.argv[1] === 'ingest' ? { exitCode: 0, stdout: 'last_sequence=0\n', stderr: '' } : { exitCode: 0, stdout, stderr: '' }),
  })

  await $.session.start(START)
  const answered = await $.command.run(rex('jobs'))

  expect(session.calls.map(one => one.argv)).toEqual([['rex', 'ingest'], fieldOf(call, 'argv')])
  expect(answered.text).toBe('0gaq820001 created composer main Fix the parser. Merge to main: yes\n')
})

test('doctor gives the binary lines and the mod version', async ($, on) => {
  const call = sharedJson('fixtures/shared/calls/doctor.json')
  const doctorOut = 'doctor=ok\nbinary=0.8.0\nstore=v2\nstorage=on\nclaude=2.1.323\nplugin=0.8.0\n'
  expect(doctorOut).toMatch(new RegExp(String(fieldOf(fieldOf(call, 'expect'), 'stdout_pattern'))))
  const session = world(on, { processAnswer: () => ({ exitCode: 0, stdout: doctorOut, stderr: '' }) })

  await $.session.start(START)
  const answered = await $.command.run(rex('doctor'))

  expect(session.calls.map(one => one.argv)).toEqual([fieldOf(call, 'argv')])
  expect(answered.text).toBe(`${doctorOut}mod=${PLUGIN_VERSION}\n`)
})

test('status shows the main job, and a failed binary read answers its own line', async ($, on) => {
  const session = world(on, { config: CONFIG_ON, processAnswer: one => (one.argv[1] === 'ingest' ? { exitCode: 0, stdout: 'last_sequence=0\n', stderr: '' } : { exitCode: 2, stdout: '', stderr: 'store is version 1; run rex migrate\n' }) })

  await $.session.start(START)
  const none = await $.command.run(rex('status'))
  const failed = await $.command.run(rex('jobs'))

  expect(none.text).toBe('job=none\n')
  expect(failed.text).toBe('store is version 1; run rex migrate')
  expect(session.book().ingest?.halted).toBeNull()
})

test('a sub-command from another module answers its own name, and the router lists it', async ($, on) => {
  const parts = createParts()
  for (const entry of OWN_SUBCOMMANDS) {
    parts.subcommands.add(entry)
  }
  registerProbe(on, parts, 'rex-probe')
  on('command.run', { command: 'rex-probe' }, (_$, e) => ({ text: unknownLine(rexLine(e).name, parts.subcommands) }))
  const line = (args: string, origin: PromptOrigin): CommandRunInput => ({ command: 'rex-probe', args, origin, presentation: PRESENTATION })

  const answered = await $.command.run(line('probe hi there', { kind: 'composer' }))
  const refused = await $.command.run(line('probe hi', { kind: 'sdk' }))
  const unknown = await $.command.run(line('nope', { kind: 'composer' }))

  expect(answered.text).toBe('probe says hi there')
  expect(refused.text).toBe('rex probe: only your own typing may run this, got origin sdk')
  expect(unknown.text).toBe('rex: no sub-command nope; one of watch, status, jobs, doctor, prompts, probe')
})

// Proves the /rex router: its sub-commands, the outline of /rex tree and the tree's lines of /rex status, the authority check on the person's own typing, and the calls to the binary against their shared files.
import { expect, test } from 'claude-code/testing'
import type { CommandRunInput, PromptOrigin } from 'claude-code'

import { CONFIG_ON, PLUGIN_VERSION, ROOT, SESSION, START_MS, world } from '../book/kit'
import { liveBook, WRITTEN_AT } from '../live/live-book.fixture'
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

  expect(answered.text).toBe('rex: no sub-command nope; one of watch, status, tree, jobs, doctor, prompts, limit, master, policy, answer')
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
  expect(unknown.text).toBe('rex: no sub-command nope; one of watch, status, tree, jobs, doctor, prompts, probe')
})

test('tree prints the outline of the session with tree guides, each node by its level, each line cut to the terminal', async ($, on) => {
  world(on, { seed: liveBook(SESSION, START_MS - WRITTEN_AT) })

  await $.session.start(START)
  const wide = await $.command.run({ ...rex('tree'), presentation: { isFullscreen: false, columns: 200 } })
  const narrow = await $.command.run(rex('tree'))

  expect(wide.text?.split('\n')).toEqual([
    '? session 00000000 · storage off · 1 question waits · 1 master tree · 2 jobs, 1 open · 2 agents open, 3 done · $5.45',
    '├─ ? master t0geh4lh3i · running · 1 question waits · 0/1 jobs delivered · 2 agents open, 2 done · $3.65 of $150.00 · 7m · Build a hello world app and a goodbye world app.',
    '│  └─ ? builder 0geh5fafkb · planned · 1 question waits · 2 agents open, 2 done · $3.25 of $28.00 · 6m · Build two small command-line apps, hello and goodbye, in th…',
    '│     ├─ ✓ lead decision-maker · done (answer) · $0.70 · 1m',
    '│     └─ ● lead engineer-lead · open · 1 agent open, 1 done · $0.60 · 3m',
    '│        └─ ● worker engineer T1 · open · 1 agent done · $0.45 · 3m',
    '│           └─ ✓ helper model-low-effort-low · done (answer) · $0.10 · 41s',
    '└─ ✓ builder 0gegkccde2 · delivered · 1 agent done · $1.80 of $28.00 · 6m · please build a hello world app',
    '   └─ ✓ lead engineer T1 · done (answer) · $0.20 · 44s',
    '',
  ])
  expect(narrow.text?.split('\n').every(line => line.length <= 120 - 16)).toBe(true)
})

test('status with no job says in plain lines what the live tree runs, where, and what waits on the person; a job named answers for that job', async ($, on) => {
  world(on, { seed: liveBook(SESSION, START_MS - WRITTEN_AT) })

  await $.session.start(START)
  const tree = await $.command.run(rex('status'))
  const job = await $.command.run(rex('status 0gegkccde2'))

  expect(tree.text?.split('\n')).toEqual([
    'rex master t0geh4lh3i running · 0/1 delivered · 2 agents open, deepest worker · $3.65/$150.00',
    '  builder 0geh5fafkb planned · leads engineer-lead · 1 worker open · $3.25/$28.00',
    '    in /home/ana/code/shop.worktrees/0geh5fafkb',
    '? q1-553d90ce69d1: Merge job 0geh5fafkb (rex/0geh5fafkb) to main of /home/ana/code/shop? · recommended Merge · /rex answer q1-553d90ce69d1 <1-2>',
    '',
  ])
  expect(job.text?.split('\n')[0]).toBe('job=0gegkccde2')
})

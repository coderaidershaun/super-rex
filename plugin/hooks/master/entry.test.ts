// Proves /rex master: only the person's own typing starts a master, each pre-flight refusal starts nothing, and the charter reaches the tree and the master unchanged.
import { expect, test } from 'claude-code/testing'
import type { PromptOrigin } from 'claude-code'

import { CONFIG_OFF } from '../book/kit'
import { sharedJson } from '../book/shared'
import { masterWorld, POLICY_KEY, rex, START, HEADLESS_START, confirmedPolicy } from './master-kit'

const OTHER_ORIGINS: readonly PromptOrigin[] = [
  { kind: 'task-notification' },
  { kind: 'scheduled-trigger' },
  { kind: 'peer' },
  { kind: 'peer-send-message' },
  { kind: 'projects-relay' },
  { kind: 'channel', server: 'slack' },
  { kind: 'coordinator' },
  { kind: 'observer' },
  { kind: 'observer-activity' },
  { kind: 'auto-continuation' },
  { kind: 'unclassified' },
  { kind: 'slack-ping' },
  { kind: 'plugin', name: 'other' },
]

for (const origin of OTHER_ORIGINS) {
  test(`master entry refused for origin ${origin.kind}, which is not the person's typing`, async ($, on) => {
    const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy({ allow_headless: true }) } })
    await $.session.start(START)

    const answered = await $.command.run(rex('master Ship the parser', origin))

    expect(answered.text).toBe(`rex master: only your own typing may run this, got origin ${origin.kind}`)
    expect(session.spawned).toEqual([])
    expect(session.book().trees).toEqual([])
  })
}

test('master_from_sdk_refused_by_default', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(HEADLESS_START)

  const answered = await $.command.run(rex('master Ship the parser', { kind: 'sdk' }))

  expect(answered.text).toBe('rex master: only your own typing may run this, got origin sdk')
  expect(session.spawned).toEqual([])
  expect(session.book().trees).toEqual([])
})

test('master from sdk passes the origin check when the policy allows headless masters', async ($, on) => {
  const session = masterWorld(on, {
    store: { [POLICY_KEY]: await confirmedPolicy({ allow_headless: true }) },
    env: { CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH: '4', CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS: '0' },
  })
  await $.session.start(HEADLESS_START)

  const answered = await $.command.run(rex('master Ship the parser', { kind: 'sdk' }))

  expect(answered.text).toBe('rex master: headless mode unknown asks no one, so each ask is a deny; run with bypassPermissions or list allow rules in policy required_allow, got required_allow is empty')
  expect(session.book().trees.map(tree => tree.state)).toEqual(['lost'])
})

test('preflight: storage off refuses and starts nothing', async ($, on) => {
  const session = masterWorld(on, { config: CONFIG_OFF, store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await $.command.run(rex('master Ship the parser'))

  expect(answered.text).toBe('rex master: needs storage; run rex init --with-storage')
  expect(session.spawned).toEqual([])
})

test('preflight: a binary of another version refuses', async ($, on) => {
  const session = masterWorld(on, { binaryVersion: 'rex 0.7.0\n', store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await $.command.run(rex('master Ship the parser'))

  expect(answered.text).toBe('rex master: binary 0.7.0 does not match plugin 0.8.0; run rex doctor')
  expect(session.spawned).toEqual([])
})

test('preflight: the version call is the shared rex --version call', async ($, on) => {
  const version = sharedJson('fixtures/shared/calls/version.json')
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)
  await $.command.run(rex('master Ship the parser'))

  const argv: unknown = typeof version === 'object' && version !== null ? Reflect.get(version, 'argv') : null
  expect(session.versionCalls.map(call => call.argv).at(-1)).toEqual(argv)
})

test('preflight: depth under 4 refuses', async ($, on) => {
  const session = masterWorld(on, { env: { CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH: '3' }, store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await $.command.run(rex('master Ship the parser'))

  expect(answered.text).toBe('rex master: depth is 3; run rex init (it writes 4)')
  expect(session.spawned).toEqual([])
})

test('preflight: depth unset refuses', async ($, on) => {
  const session = masterWorld(on, { env: {}, store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await $.command.run(rex('master Ship the parser'))

  expect(answered.text).toBe('rex master: depth is unset; run rex init (it writes 4)')
  expect(session.spawned).toEqual([])
})

test('headless_master_needs_wait_ceiling_zero', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy({ allow_headless: true }) } })
  await $.session.start(HEADLESS_START)

  const answered = await $.command.run(rex('master Ship the parser', { kind: 'sdk' }))

  expect(answered.text).toBe('rex master: headless runs need CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0, or background agents stop after 10 minutes')
  expect(session.spawned).toEqual([])
})

test('preflight: headless with no confirmed policy refuses', async ($, on) => {
  const session = masterWorld(on, { env: { CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH: '4', CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS: '0' } })
  await $.session.start(HEADLESS_START)

  const answered = await $.command.run(rex('master Ship the parser'))

  expect(answered.text).toBe('rex master: no confirmed policy for this repo; run /rex policy confirm in an interactive session')
  expect(session.spawned).toEqual([])
})

test('preflight: a policy changed since it was confirmed refuses', async ($, on) => {
  const changed = { ...(await confirmedPolicy()), tree_usd: 900 }
  const session = masterWorld(on, { store: { [POLICY_KEY]: changed } })
  await $.session.start(START)

  const answered = await $.command.run(rex('master Ship the parser'))

  expect(answered.text).toMatch(/^rex master: the policy of this repo changed since you confirmed it, got hash sha256:[0-9a-f]{64}; run \/rex policy confirm$/)
  expect(session.spawned).toEqual([])
})

test('preflight: headless default permission mode refuses at the master spawn, and the tree is lost', async ($, on) => {
  const session = masterWorld(on, {
    store: { [POLICY_KEY]: await confirmedPolicy({ required_allow: ['Bash(cargo test:*)'] }) },
    env: { CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH: '4', CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS: '0' },
    allow: ['Bash(git status:*)'],
  })
  await $.session.start(HEADLESS_START)

  const answered = await $.command.run(rex('master Ship the parser'))

  expect(answered.text).toBe('rex master: headless mode unknown asks no one, so each ask is a deny; run with bypassPermissions or list allow rules in policy required_allow, got Bash(cargo test:*)')
  expect(session.book().trees.map(tree => tree.state)).toEqual(['lost'])
  expect(session.book().runs).toEqual([])
})

test('preflight: headless passes the permission check when each required allow rule is in the settings', async ($, on) => {
  const session = masterWorld(on, {
    store: { [POLICY_KEY]: await confirmedPolicy({ required_allow: ['Bash(cargo test:*)'] }) },
    env: { CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH: '4', CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS: '0' },
    allow: ['Bash(cargo test:*)', 'Read'],
  })
  await $.session.start(HEADLESS_START)

  const answered = await $.command.run(rex('master Ship the parser'))

  expect(answered.text).toMatch(/^tree=t[0-9a-z]{9}\nmaster=m1\n$/)
  expect(session.book().trees.map(tree => tree.state)).toEqual(['running'])
})

test('master_starts_after_preflight_with_charter_unchanged', async ($, on) => {
  const charter = '# Charter\n\nShip the parser.\n\n## Constraints\n\n- Never touch infra/.\n'
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() }, files: { '/home/person/repo/charter.md': charter } })
  await $.session.start(START)

  const answered = await $.command.run(rex('master @charter.md'))

  const tree = session.book().trees[0]
  expect(answered.text).toBe(`tree=${tree?.id ?? ''}\nmaster=m1\n`)
  expect(tree?.charter).toBe(charter)
  expect(tree?.state).toBe('running')
  expect(tree?.master).toBe('m1')
  expect(session.spawned.map(spawn => [spawn.type, spawn.description])).toEqual([['super-rex:master', `rex master ${tree?.id ?? ''}`]])
  expect(session.spawned[0]?.prompt).toContain(charter)
  const run = session.book().runs.find(each => each.agentId === 'm1')
  expect([run?.parent, run?.parentSource, run?.job, run?.tree]).toEqual(['main', 'mod', null, tree?.id])
})

test('a charter file that cannot be read refuses and starts nothing', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await $.command.run(rex('master @missing.md'))

  expect(answered.text).toBe('rex master: charter file /home/person/repo/missing.md could not be read, got HooksError: super-rex: $.fs.read: ENOENT: no file at /home/person/repo/missing.md; name a file in the repo')
  expect(session.spawned).toEqual([])
})

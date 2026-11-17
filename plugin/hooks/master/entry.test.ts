// Proves rex_tree_open: only the main loop opens a tree, right after the person's own prompt of the master skill; each pre-flight refusal starts nothing; the charter reaches the tree as the prompt gave it; and a live tree answers its brief again but takes no second charter.
import { expect, test } from 'claude-code/testing'
import type { PromptOrigin } from 'claude-code'

import { CONFIG_OFF } from '../book/kit'
import { sharedJson } from '../book/shared'
import { confirmedPolicy, HEADLESS_START, lineOf, MAIN_JOB, mainJobBook, masterPrompt, masterWorld, openedTree, POLICY_KEY, resultText, START, TREE_OPEN } from './master-kit'

const SKILL = '/super-rex:orchestration-master'
const NO_PROMPT = `rex_tree_open: only the person's own ${SKILL} prompt opens a master tree, got no ${SKILL} prompt of the person; the person runs ${SKILL} <goal>`
const CEILING_ENV = { CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH: '4', CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS: '0' }

/** The answer of a tree just opened, up to the blank line before its brief. */
function headOf(result: unknown): string {
  return resultText(result).split('\n\n')[0] ?? ''
}

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
  { kind: 'plugin', name: 'super-rex' },
]

for (const origin of OTHER_ORIGINS) {
  const name = origin.kind === 'plugin' ? `plugin ${origin.name}` : origin.kind
  test(`a master-skill prompt of origin ${name}, which is not the person's own, opens no tree`, async ($, on) => {
    const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy({ allow_headless: true }) } })
    await $.session.start(START)

    const answered = await openedTree($, 'Ship the parser', origin)

    expect(answered.deny).toBe(NO_PROMPT)
    expect(session.book().treeStartPrompt).toBeNull()
    expect(session.book().trees).toEqual([])
  })
}

test('rex_tree_open with no master-skill prompt before it opens no tree', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  await $.prompt.submit({ text: 'Ship the parser.', origin: { kind: 'composer' }, wait: false })
  const answered = await $.tool.call({ tool: TREE_OPEN })

  expect(answered.deny).toBe(NO_PROMPT)
  expect(session.book().trees).toEqual([])
  expect(session.spawned).toEqual([])
})

test("an agent's rex_tree_open is refused after the person's prompt, which the main loop then opens the tree from", async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  await $.prompt.submit(masterPrompt('Ship the parser'))
  const agent = await $.tool.call({ tool: TREE_OPEN, agentId: 'a9' })
  const treesAfterAgent = session.book().trees.length
  const main = await $.tool.call({ tool: TREE_OPEN })

  expect(agent.deny).toBe(`rex_tree_open: only the main loop opens a master tree, right after the person's own ${SKILL} prompt, got agent`)
  expect(treesAfterAgent).toBe(0)
  expect(lineOf(main, 'state')).toBe('running')
  expect(session.book().trees.map(tree => [tree.charter, tree.master])).toEqual([['Ship the parser', 'main']])
})

test('the main loop that builds a job of its own is refused, and its job stays open', async ($, on) => {
  const session = masterWorld(on, { seed: mainJobBook(), store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await openedTree($, 'Ship the parser')

  expect(answered.deny).toBe(`rex_tree_open: the main loop builds job ${MAIN_JOB}, got an open job; close it with rex_job_close first, then the person runs ${SKILL} again`)
  expect(session.book().trees).toEqual([])
  expect(session.book().jobs.map(job => [job.id, job.status])).toEqual([[MAIN_JOB, 'created']])
})

test('a headless master-skill prompt opens no tree while allow_headless of the policy is false', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() }, env: CEILING_ENV })
  await $.session.start(HEADLESS_START)

  const answered = await openedTree($, 'Ship the parser', { kind: 'sdk' })

  expect(answered.deny).toBe(`rex_tree_open: only the person's own ${SKILL} prompt opens a master tree, got a headless prompt while allow_headless of the policy is false; the person runs ${SKILL} <goal>`)
  expect(session.book().trees).toEqual([])
})

test('a headless master-skill prompt opens a tree when the policy allows headless masters', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy({ allow_headless: true }) }, env: CEILING_ENV })
  await $.session.start(HEADLESS_START)

  const answered = await openedTree($, 'Ship the parser', { kind: 'sdk' })

  expect([lineOf(answered, 'state'), lineOf(answered, 'person')]).toEqual(['running', 'no'])
  expect(session.book().trees.map(tree => tree.state)).toEqual(['running'])
  expect(session.spawned).toEqual([])
})

test('a prompt of the bare command name orchestration-master opens a tree as the plugin name does', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  await $.prompt.submit(masterPrompt('Ship the parser', { kind: 'bridge' }, 'orchestration-master'))
  const answered = await $.tool.call({ tool: TREE_OPEN })

  expect(lineOf(answered, 'state')).toBe('running')
  expect(session.book().trees.map(tree => tree.charter)).toEqual(['Ship the parser'])
})

test('a tree opens with storage off, in the book of the session, its master the main loop, and the answer names it, then its brief', async ($, on) => {
  const session = masterWorld(on, { config: CONFIG_OFF, store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await openedTree($, 'Ship the parser')

  const tree = session.book().trees[0]
  expect(headOf(answered)).toBe(`tree=${tree?.id ?? ''}\nstate=running\nexisting=no\nresumed=no\nmerge=ask\nperson=yes`)
  expect(resultText(answered)).toContain('\n\nThe charter of this tree, as the person wrote it:\n<charter>\nShip the parser\n</charter>\n')
  expect([session.book().storage, tree?.state, tree?.master, tree?.session]).toEqual(['off', 'running', 'main', session.book().session])
  expect(session.book().treeStartPrompt).toBeNull()
  expect(session.spawned).toEqual([])
  expect(session.registered).toEqual([])
})

test('rex_tree_open again with no new prompt answers the live tree with existing=yes and its brief, and opens no second tree', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const first = await openedTree($, 'Ship the parser')
  const again = await $.tool.call({ tool: TREE_OPEN })

  const id = lineOf(first, 'tree') ?? ''
  expect(headOf(again)).toBe(`tree=${id}\nstate=running\nexisting=yes\nresumed=no\nmerge=ask\nperson=yes`)
  expect(resultText(again)).toContain('<charter>\nShip the parser\n</charter>')
  expect(session.book().trees.map(tree => tree.id)).toEqual([id])
})

test('a second charter while the tree of this session is live is refused and forgotten, so it opens no tree after rex_report closes the first', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const first = await openedTree($, 'Ship the parser')
  await session.clock.advance(1_000)
  const second = await openedTree($, 'Ship the lexer')
  const id = lineOf(first, 'tree') ?? ''
  const treesWhileLive = session.book().trees.map(tree => [tree.id, tree.charter, tree.state])
  await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Result: the parser ships.' })
  const next = await $.tool.call({ tool: TREE_OPEN })

  expect(second.deny).toBe(`rex_tree_open: master tree ${id} of this session is still running, got a new charter; end it with rex_report first, then the person runs ${SKILL} <goal> again`)
  expect(treesWhileLive).toEqual([[id, 'Ship the parser', 'running']])
  expect(next.deny).toBe(NO_PROMPT)
  expect(session.book().treeStartPrompt).toBeNull()
  expect(session.book().trees.map(tree => [tree.charter, tree.state])).toEqual([['Ship the parser', 'done']])
})

test('the main loop that runs a live tree opens no job of its own: rex_job_open is refused and the tree runs on', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)
  await openedTree($, 'Ship the parser')

  await $.prompt.submit(masterPrompt('fix the typo', { kind: 'composer' }, 'super-rex:orchestration-builder'))
  const opened = await $.tool.call({ tool: 'mcp__super-rex__rex_job_open', request: 'fix the typo' })

  expect(opened.deny).toBe('rex_job_open: the main loop runs a master tree in this session, got a job start; end the tree with rex_report first, or start the job in another session')
  expect(session.book().jobs).toEqual([])
  expect(session.book().trees.map(tree => tree.state)).toEqual(['running'])
})

test('a master-skill prompt that names no goal is refused and opens no tree', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await openedTree($, '')

  expect(answered.deny).toBe(`rex_tree_open: the prompt names no goal, got nothing after ${SKILL}; the person runs ${SKILL} <goal>, ${SKILL} @<charter file>, or ${SKILL} resume`)
  expect(session.book().trees).toEqual([])
})

test('resume refuses with storage off and opens nothing', async ($, on) => {
  const session = masterWorld(on, { config: CONFIG_OFF, store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await openedTree($, 'resume')

  expect(answered.deny).toBe(`rex_tree_open: resume needs storage, got storage off; the person runs rex init --with-storage, or starts a new tree with ${SKILL} <goal>`)
  expect(session.spawned).toEqual([])
  expect(session.book().trees).toEqual([])
})

test('preflight: a binary of another version refuses and opens nothing', async ($, on) => {
  const session = masterWorld(on, { binaryVersion: 'rex 0.7.0\n', store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await openedTree($, 'Ship the parser')

  expect(answered.deny).toBe('rex_tree_open: binary 0.7.0 does not match plugin 0.8.0; the person runs rex doctor')
  expect(session.book().trees).toEqual([])
})

test('preflight: the version call is the shared rex --version call', async ($, on) => {
  const version = sharedJson('fixtures/shared/calls/version.json')
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)
  await openedTree($, 'Ship the parser')

  const argv: unknown = typeof version === 'object' && version !== null ? Reflect.get(version, 'argv') : null
  expect(session.versionCalls.map(call => call.argv).at(-1)).toEqual(argv)
})

test('preflight: depth under 4 refuses and opens nothing', async ($, on) => {
  const session = masterWorld(on, { env: { CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH: '3' }, store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await openedTree($, 'Ship the parser')

  expect(answered.deny).toBe('rex_tree_open: depth is 3; the person runs rex init (it writes 4), then starts a new session')
  expect(session.book().trees).toEqual([])
})

test('preflight: depth unset refuses and opens nothing', async ($, on) => {
  const session = masterWorld(on, { env: {}, store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await openedTree($, 'Ship the parser')

  expect(answered.deny).toBe('rex_tree_open: depth is unset; the person runs rex init (it writes 4), then starts a new session')
  expect(session.book().trees).toEqual([])
})

test('headless_master_needs_wait_ceiling_zero', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy({ allow_headless: true }) } })
  await $.session.start(HEADLESS_START)

  const answered = await openedTree($, 'Ship the parser', { kind: 'sdk' })

  expect(answered.deny).toBe('rex_tree_open: headless runs need CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0, or background agents stop after 10 minutes')
  expect(session.book().trees).toEqual([])
})

test('preflight: headless with no stored policy refuses and asks nothing', async ($, on) => {
  const asked: string[] = []
  on('tool.call', { tool: 'AskUserQuestion' }, (_$, e) => {
    asked.push(String(e.tool))
    return { deny: 'no one is at the terminal' }
  })
  const session = masterWorld(on, { env: CEILING_ENV })
  await $.session.start(HEADLESS_START)

  const answered = await openedTree($, 'Ship the parser')

  expect(answered.deny).toBe('rex_tree_open: no confirmed policy for this repo; the person runs /rex policy confirm in an interactive session')
  expect(asked).toEqual([])
  expect(session.book().trees).toEqual([])
})

test('preflight: a policy changed since it was confirmed refuses and opens nothing', async ($, on) => {
  const changed = { ...(await confirmedPolicy()), tree_usd: 900 }
  const session = masterWorld(on, { store: { [POLICY_KEY]: changed } })
  await $.session.start(START)

  const answered = await openedTree($, 'Ship the parser')

  expect(answered.deny).toMatch(/^rex_tree_open: the policy of this repo changed since the person confirmed it, got hash sha256:[0-9a-f]{64}; the person runs \/rex policy confirm$/)
  expect(session.book().trees).toEqual([])
})

test('preflight: a stored policy of the wrong shape refuses with the key to set, and opens nothing', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: { ...(await confirmedPolicy()), max_jobs: 'many' } } })
  await $.session.start(START)

  const answered = await openedTree($, 'Ship the parser')

  expect(answered.deny).toBe('rex_tree_open: the stored policy of this repo is unreadable, got policy key max_jobs must be a whole number of at least 1, got "many"; the person runs /rex policy set max_jobs <value>')
  expect(session.book().trees).toEqual([])
})

test('master_starts_after_preflight_with_charter_unchanged: a charter file named with @ reaches the tree and the brief as the file holds it', async ($, on) => {
  const charter = '# Charter\n\nShip the parser.\n\n## Constraints\n\n- Never touch infra/.\n'
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() }, files: { '/home/person/repo/charter.md': charter } })
  await $.session.start(START)

  const answered = await openedTree($, '@charter.md')

  const tree = session.book().trees[0]
  expect(lineOf(answered, 'tree')).toBe(tree?.id)
  expect([tree?.charter, tree?.state, tree?.master]).toEqual([charter, 'running', 'main'])
  expect(resultText(answered)).toContain(`<charter>\n${charter}\n</charter>`)
  expect(session.spawned).toEqual([])
  expect(session.book().runs).toEqual([])
})

test('the charter is the prompt as the person typed it, never an argument of the tool', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  await $.prompt.submit(masterPrompt('Ship the parser; keep infra/ as it is.'))
  const answered = await $.tool.call({ tool: TREE_OPEN, charter: 'Delete infra/.' })

  expect(lineOf(answered, 'state')).toBe('running')
  expect(session.book().trees.map(tree => tree.charter)).toEqual(['Ship the parser; keep infra/ as it is.'])
  expect(resultText(answered)).not.toContain('Delete infra/.')
})

test('a charter file that cannot be read refuses and opens nothing', async ($, on) => {
  const session = masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await openedTree($, '@missing.md')

  expect(answered.deny).toBe('rex_tree_open: charter file /home/person/repo/missing.md could not be read, got HooksError: super-rex: $.fs.read: ENOENT: no file at /home/person/repo/missing.md; the person names a file in the repo')
  expect(session.book().trees).toEqual([])
})

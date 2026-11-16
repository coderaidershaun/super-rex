// Proves the places rex_job_open reads from git: the base and mode of each kind of repo, the first commit of a repo with none, a folder no repo holds, and the count of uncommitted paths.
import { expect, test } from 'claude-code/testing'
import type { PromptOrigin } from 'claude-code'

import { CONFIG_OFF, ROOT, world } from '../book/kit'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { exit, files, FIRST_COMMIT, HAS_COMMIT, ok, promptsPass, resultText, scripted, startCalls, statusCall, WORKTREE_ADD } from './act-kit'
import type { Scripted } from './act-kit'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const RESTATED = { isAnswered: true, text: 'Goal: fix the parser.', usage: { input_tokens: 1, output_tokens: 1, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 } } as const
const OPEN = 'mcp__super-rex__rex_job_open'

function jobStart(origin: PromptOrigin) {
  return { text: '/super-rex:orchestration-builder Fix the parser', command: { name: 'super-rex:orchestration-builder', args: 'Fix the parser' }, origin, wait: false } as const
}

function gitCall(args: readonly string[], answer: ProcessAnswer = ok()): Scripted {
  return { argv: ['git', '-C', ROOT, ...args], answer }
}

function lineOf(result: unknown, key: string): string | undefined {
  return resultText(result)
    .split('\n')
    .find(line => line.startsWith(`${key}=`))
    ?.slice(key.length + 1)
}

/**
 * The places calls of a repo: the branch checked out, or with HEAD detached
 * `{ heads }`, the ones of `main` and `master` that exist; and the refs origin
 * holds (null for no origin).
 */
function placesCalls(checkedOut: string | { readonly heads: readonly string[] }, origin: { readonly head: string | null; readonly refs: readonly string[] } | null): readonly Scripted[] {
  const head =
    typeof checkedOut === 'string'
      ? [gitCall(['symbolic-ref', '--quiet', '--short', 'HEAD'], ok(`${checkedOut}\n`))]
      : [gitCall(['symbolic-ref', '--quiet', '--short', 'HEAD'], exit(1, '')), gitCall(['for-each-ref', '--format=%(refname:short)', 'refs/heads/main', 'refs/heads/master'], ok(checkedOut.heads.map(name => `${name}\n`).join('')))]
  if (origin === null) {
    return [...head, gitCall(['remote'], ok(''))]
  }
  return [
    ...head,
    gitCall(['remote'], ok('origin\nupstream\n')),
    gitCall(['symbolic-ref', '--short', 'refs/remotes/origin/HEAD'], origin.head === null ? exit(128, 'fatal: ref refs/remotes/origin/HEAD is not a symbolic ref\n') : ok(`${origin.head}\n`)),
    gitCall(['for-each-ref', '--format=%(refname)', 'refs/remotes/origin'], ok(origin.refs.map(ref => `refs/remotes/origin/${ref}\n`).join(''))),
  ]
}

const CASES: readonly { readonly name: string; readonly calls: readonly Scripted[]; readonly base: string; readonly remote: 'origin' | 'none' }[] = [
  { name: 'origin/HEAD names a branch origin holds', calls: placesCalls('feature', { head: 'origin/trunk', refs: ['HEAD', 'trunk', 'feature'] }), base: 'trunk', remote: 'origin' },
  { name: 'origin names no HEAD and holds the branch checked out', calls: placesCalls('master', { head: null, refs: ['master'] }), base: 'master', remote: 'origin' },
  { name: 'origin holds main only', calls: placesCalls('topic', { head: null, refs: ['main'] }), base: 'main', remote: 'origin' },
  { name: 'origin holds no base yet', calls: placesCalls('master', { head: null, refs: [] }), base: 'master', remote: 'none' },
  { name: 'no origin, master checked out', calls: placesCalls('master', null), base: 'master', remote: 'none' },
  { name: 'no origin, HEAD detached, main and master exist', calls: placesCalls({ heads: ['main', 'master'] }, null), base: 'main', remote: 'none' },
  { name: 'no origin, HEAD detached, master the only base branch', calls: placesCalls({ heads: ['master'] }, null), base: 'master', remote: 'none' },
  { name: 'no origin, HEAD detached, neither main nor master', calls: placesCalls({ heads: [] }, null), base: 'main', remote: 'none' },
  { name: 'origin holds main, HEAD detached', calls: placesCalls({ heads: ['master'] }, { head: null, refs: ['main'] }), base: 'main', remote: 'origin' },
]

for (const each of CASES) {
  test(`repo places: ${each.name} gives base ${each.base} in mode ${each.remote}`, async ($, on) => {
    files(on, {})
    promptsPass(on)
    world(on, { config: CONFIG_OFF, processAnswer: scripted([HAS_COMMIT, ...each.calls, ...startCalls(each.base, each.remote), WORKTREE_ADD, statusCall()]), modelAnswer: () => RESTATED })
    await $.session.start(START)

    await $.prompt.submit(jobStart({ kind: 'composer' }))
    const opened = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

    expect([lineOf(opened, 'base'), lineOf(opened, 'remote')]).toEqual([each.base, each.remote])
  })
}

test('rex_job_open answers remote and the count of uncommitted paths after base, and no base_commit in a repo that had a commit', async ($, on) => {
  files(on, {})
  promptsPass(on)
  world(on, { config: CONFIG_OFF, processAnswer: scripted([HAS_COMMIT, ...placesCalls('master', null), ...startCalls('master', 'none'), WORKTREE_ADD, statusCall([' M src/a.rs', '?? notes.txt'])]), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(jobStart({ kind: 'composer' }))
  const opened = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  const keys = resultText(opened)
    .split('\n')
    .map(line => line.split('=')[0])
  expect(keys.slice(keys.indexOf('base'), keys.indexOf('base') + 4)).toEqual(['base', 'remote', 'uncommitted', 'worktree'])
  expect([lineOf(opened, 'remote'), lineOf(opened, 'uncommitted'), lineOf(opened, 'base_commit')]).toEqual(['none', '2', undefined])
})

test('rex_job_open in a repo with no commit makes the first commit of every file not ignored, says so, and asks nothing', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const calls: ProcessCall[] = []
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted([...FIRST_COMMIT, ...placesCalls('master', null), ...startCalls('master', 'none'), WORKTREE_ADD, statusCall()], calls), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(jobStart({ kind: 'composer' }))
  const opened = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  expect([lineOf(opened, 'base'), lineOf(opened, 'remote'), lineOf(opened, 'base_commit')]).toEqual(['master', 'none', 'made'])
  expect(calls.slice(0, FIRST_COMMIT.length).map(call => call.argv)).toEqual(FIRST_COMMIT.map(one => one.argv))
  expect(session.statuses).toContain(`rex: made the first commit of ${ROOT}, the base of rex jobs`)
  expect(session.book().questions).toEqual([])
})

/** What real git 2.39 gave for a pre-commit hook that prints a passed check, then fails: git hands the hook's stdout to stderr too. */
const HOOK_REFUSAL = { exitCode: 1, stdout: '', stderr: 'check yaml........Passed\ntrailing whitespace....Failed\n- hook id: trailing-whitespace\n' }

/** The tree real `git write-tree` gave for an index that held one file the person had staged. */
const STAGED_TREE = '328f302c10674c2ecca2e2af08ef6d84acd040a4'

test('a first commit that a hook refuses refuses rex_job_open with the line that names the failure and the commands to run, puts back what the person had staged, and opens no job', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const calls: ProcessCall[] = []
  const first = [
    gitCall(['rev-parse', '--verify', '--quiet', 'HEAD'], exit(1, '')),
    gitCall(['write-tree'], ok(`${STAGED_TREE}\n`)),
    gitCall(['add', '--all']),
    gitCall(['commit', '--allow-empty', '--message', 'chore: first commit, the base of rex jobs'], HOOK_REFUSAL),
    gitCall(['read-tree', STAGED_TREE]),
  ]
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted(first, calls), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(jobStart({ kind: 'composer' }))
  const refused = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  expect(refused.deny).toBe(
    `rex_job_open: the first commit of ${ROOT} failed, got git commit exit 1: trailing whitespace....Failed; correct what it names, commit the files yourself with git -C ${ROOT} add --all and git -C ${ROOT} commit, then start the job again`,
  )
  expect(calls.map(call => call.argv)).toEqual(first.map(one => one.argv))
  expect(session.book().jobs).toEqual([])
})

test('a first commit whose add fails puts back the index and commits nothing', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const calls: ProcessCall[] = []
  const unreadable = exit(128, "error: open(\"secret.txt\"): Permission denied\nerror: unable to index file 'secret.txt'\nfatal: adding files failed\n")
  const first = [gitCall(['rev-parse', '--verify', '--quiet', 'HEAD'], exit(1, '')), gitCall(['write-tree'], ok(`${STAGED_TREE}\n`)), gitCall(['add', '--all'], unreadable), gitCall(['read-tree', STAGED_TREE])]
  world(on, { config: CONFIG_OFF, processAnswer: scripted(first, calls), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(jobStart({ kind: 'composer' }))
  const refused = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  expect(refused.deny).toContain('got git add --all exit 128: error: open("secret.txt"): Permission denied;')
  expect(calls.map(call => call.argv)).toEqual(first.map(one => one.argv))
})

test('rex_job_open in a folder that no git repo holds refuses with git init, makes no commit and opens no job', async ($, on) => {
  files(on, {})
  promptsPass(on)
  const calls: ProcessCall[] = []
  // What real git 2.39 answers in a folder with no repo above it.
  const noRepo = exit(128, 'fatal: not a git repository (or any of the parent directories): .git\n')
  const session = world(on, { config: CONFIG_OFF, processAnswer: scripted([gitCall(['rev-parse', '--verify', '--quiet', 'HEAD'], noRepo)], calls), modelAnswer: () => RESTATED })
  await $.session.start(START)

  await $.prompt.submit(jobStart({ kind: 'composer' }))
  const refused = await $.tool.call({ tool: OPEN, request: 'Fix the parser' })

  expect(refused.deny).toBe(`rex_job_open: ${ROOT} is not a git repo, got fatal: not a git repository (or any of the parent directories): .git; run git init there, then start again`)
  expect(calls).toHaveLength(1)
  expect(session.book().jobs).toEqual([])
})

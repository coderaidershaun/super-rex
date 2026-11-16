// Proves the close of a merged tree job end to end: its builder's rex_job_close removes the worktree its own shell runs in and every branch of the job, the mod runs each git call in the main checkout, and the done check that follows passes.
import { expect, test } from 'claude-code/testing'

import type { RexBook } from '../../types'
import { mergeDoneIn, openJobIn } from '../book/jobs'
import { ROOT, START_MS } from '../book/kit'
import type { ProcessAnswer, ProcessCall } from '../book/kit'
import { linkIn } from '../book/trees'
import { byRule, confirmedPolicy, lineOf, MASTER, masterWorld, placesRules, POLICY_KEY, ran, resultText, settleUntil, START, TREE, treeBook } from './master-kit'
import type { Policy } from './policy'

const JOB = '0gaq820002'
const BRANCH = `rex/${JOB}`
const JOB_DIR = `${ROOT}.worktrees/${JOB}`
const MERGE = '3f2a9c1e8b7d6a5f4e3d2c1b0a9f8e7d6c5b4a39'
const WORK_TIP = '77aa88bb99cc00dd11ee22ff33aa44bb55cc66dd'
const HAND_OFF = `${ROOT}/.rex/plans/${JOB}/builder.md`
const ENDED = { agentId: 'b1', reason: 'answer', answer: 'Merged as 3f2a9c1.', durationMs: 1, isAborted: false, turnId: 'turn-b1' } as const
const NO = { exitCode: 1, stdout: '', stderr: '' } as const

/** Tree t1 with job JOB, whose builder b1 runs in the job worktree, merged by rex_merge and not closed yet. */
async function mergedTreeBook(policy: Policy): Promise<RexBook> {
  const input = {
    builder: 'b1',
    tree: TREE,
    origin: 'master',
    request: 'Fix the parser',
    repo: ROOT,
    base: 'main',
    type: 'fix',
    size: 'small',
    consequence: 'low',
    langs: ['rust'],
    prefs: { duration: null, budget: null, autonomy: 'high' },
    limit: { nano: 10_000_000_000, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  const opened = linkIn(openJobIn(await treeBook(policy), input, JOB, START_MS).book, TREE, JOB, 'b1', START_MS)
  return mergeDoneIn(opened, JOB, { repo: ROOT, base: 'main', branch: BRANCH, mergeCommit: MERGE }, START_MS)
}

/**
 * A repo in mode origin that changes as the mod acts on it: the job worktree
 * until rex worktree removes it, each branch of the job until it is deleted
 * here or on origin.
 */
function liveRepo(): (call: ProcessCall) => ProcessAnswer {
  let isWorktreeThere = true
  let isRemoteThere = true
  const heads = new Set([BRANCH, `${BRANCH}--work`])
  const onBase = new Set([BRANCH, `refs/remotes/origin/${BRANCH}`])
  const rest = byRule([...placesRules('origin'), { starts: ['rex', 'job', 'show'], answer: ran('{}\n') }])
  return call => {
    const words = call.argv.slice(3).join(' ')
    if (call.argv.slice(0, 3).join(' ') === 'rex worktree remove') {
      isWorktreeThere = false
      return ran(`worktree=${JOB_DIR}\nsettings=removed\n`)
    }
    if (words === 'worktree list --porcelain') {
      return ran(`worktree ${ROOT}\nHEAD ${MERGE}\nbranch refs/heads/main\n${isWorktreeThere ? `\nworktree ${JOB_DIR}\nHEAD ${MERGE}\nbranch refs/heads/${BRANCH}\n` : ''}`)
    }
    if (words.startsWith('for-each-ref --format=%(refname:short) ')) {
      return ran([...heads].map(head => `${head}\n`).join(''))
    }
    if (words.startsWith('for-each-ref --format=%(refname:lstrip=3) ')) {
      return ran(isRemoteThere ? `${BRANCH}\n` : '')
    }
    if (words.startsWith('branch -D ')) {
      heads.delete(call.argv[5] ?? '')
      return ran()
    }
    if (words === `push origin --delete ${BRANCH}`) {
      isRemoteThere = false
      return ran()
    }
    if (words.startsWith('branch -a --list')) {
      return ran([...heads, ...(isRemoteThere ? [`remotes/origin/${BRANCH}`] : [])].map(head => `  ${head}\n`).join(''))
    }
    if (words === `rev-parse refs/heads/${BRANCH}--work refs/heads/${BRANCH}--work^{tree} ${MERGE}^{tree}`) {
      return ran(`${WORK_TIP}\ntree1\ntree1\n`)
    }
    if (words.startsWith('merge-base --is-ancestor ')) {
      const [commit = '', of = ''] = call.argv.slice(5)
      return commit === MERGE || of === WORK_TIP || onBase.has(commit) ? ran() : NO
    }
    if (words === 'fetch --prune origin' || words === 'fetch origin') {
      return ran()
    }
    if (words === `branch -r --contains ${MERGE}`) {
      return ran('  origin/main\n')
    }
    if (words === 'rev-parse main origin/main') {
      return ran(`${MERGE}\n${MERGE}\n`)
    }
    return rest(call)
  }
}

test('a tree builder that closes its merged job loses its own worktree, the close and its last message still go through, and the done check passes', async ($, on) => {
  const policy = await confirmedPolicy({ merge_checks: [] })
  const calls: ProcessCall[] = []
  const repo = liveRepo()
  const session = masterWorld(on, {
    seed: await mergedTreeBook(policy),
    store: { [POLICY_KEY]: policy },
    processAnswer: call => {
      calls.push(call)
      return repo(call)
    },
    files: { [HAND_OFF]: '# Report\n' },
  })
  await $.session.start(START)

  const closed = await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', agentId: 'b1', status: 'delivered', outcome: 'parser fixed', deliveries: [{ repo: ROOT, outcome: 'merged' }] })
  await $.turn.complete(ENDED)
  await settleUntil(session.clock, () => session.sent.some(sent => sent.to === MASTER))
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: MASTER, job: JOB })

  expect(resultText(closed).split('\n').filter(line => line.startsWith('removed.') || line.startsWith('kept.'))).toEqual([
    `removed.worktree=${JOB_DIR}`,
    `removed.branch=${BRANCH}`,
    `removed.branch=${BRANCH}--work`,
    `removed.remote_branch=${BRANCH}`,
  ])
  // The builder's shell sits in the job worktree; the mod never runs a call there, so its removal breaks no call that follows.
  expect(calls.filter(call => call.argv[1] !== 'ingest').map(call => call.cwd).filter(cwd => cwd !== ROOT)).toEqual([])
  expect(session.sent.find(sent => sent.to === MASTER)?.text).toContain(`job ${JOB} was delivered and its done check ran`)
  expect(['merged', 'branches_gone', 'worktrees_gone'].map(check => lineOf(status, `done.${check}`))).toEqual(['pass', 'pass', 'pass'])
})

// Proves a delivered tree job from git, not from what its builder said: the merge on origin, the local base, no branch or worktree left, the report written, and the checks green on the base.
import type { EngineInterface } from 'claude-code'

import type { RexDoneCheck, RexJob } from '../../types'
import { recordDoneCheck } from '../book/index'
import { handOffFile } from './brief'
import type { Policy } from './policy'
import { git } from './worktree'

const CHECK_TIMEOUT_MS = 600_000

type Result = RexDoneCheck['result']

function passIf(isTrue: boolean): Result {
  return isTrue ? 'pass' : 'fail'
}

async function checksOnBase($: EngineInterface, repo: string, policy: Policy): Promise<Result> {
  if (policy.merge_checks.length === 0) {
    return 'not-set'
  }
  for (const argv of policy.merge_checks) {
    const ran = await $.process.run(argv, { cwd: repo, timeoutMs: CHECK_TIMEOUT_MS })
    if (ran.exitCode !== 0) {
      return 'fail'
    }
  }
  return 'pass'
}

/** Runs each check in order and records its result as it comes, so a check that hangs leaves the earlier results in the book. */
export async function runDoneCheck($: EngineInterface, job: RexJob, policy: Policy): Promise<void> {
  const record = (check: RexDoneCheck['check'], result: Result): Promise<void> => recordDoneCheck($, job.id, { check, result })
  const { repo, base } = job
  const fetched = await git($, repo, ['fetch', 'origin'])
  await record('fetched', passIf(fetched.exitCode === 0))
  const commit = job.deliveries.find(delivery => delivery.outcome === 'merged')?.mergeCommit ?? null
  const holding = commit === null ? null : await git($, repo, ['branch', '-r', '--contains', commit])
  await record('merged', passIf(holding !== null && holding.exitCode === 0 && holding.stdout.split('\n').some(line => line.trim() === `origin/${base}`)))
  const heads = await git($, repo, ['rev-parse', base, `origin/${base}`])
  const [local, remote] = heads.stdout.split('\n')
  await record('base_current', passIf(heads.exitCode === 0 && local !== undefined && local !== '' && local === remote))
  const branches = await git($, repo, ['branch', '-a', '--list', `*${job.id}*`])
  await record('branches_gone', passIf(branches.exitCode === 0 && branches.stdout === ''))
  const worktrees = await git($, repo, ['worktree', 'list', '--porcelain'])
  await record('worktrees_gone', passIf(worktrees.exitCode === 0 && !worktrees.stdout.includes(job.id)))
  await record('output_exists', passIf(await $.fs.exists(handOffFile(job))))
  await record('checks_on_base', await checksOnBase($, repo, policy))
}

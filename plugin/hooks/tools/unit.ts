// rex_worktree: the builder of a job or any agent of it gets the worktree of one unit of parallel work, which the mod makes from the tip of the job branch, so no agent runs a worktree command.
import type { EngineInterface, ToolCallResult } from 'claude-code'

import { fail, isOpen, jobWorktree, readBook } from '../book/index'
import type { ToolEntry } from '../parts'
import { argsOf, field, refused } from './args'
import { callerIn, callerRefusal } from './caller'
import { git } from './git'
import { answer, deny } from './text'
import { addWorktree, hasWorktree } from './worktree'

/** One to three lower-case words of letters and digits, joined by single hyphens, as the parallel writers' rules name a unit. */
const UNIT = /^[a-z0-9]+(?:-[a-z0-9]+){0,2}$/

/** `<job branch>--work` keeps the work before the squash, and `<job id>--review` is the reviewer's. */
const RESERVED = ['work', 'review']

/** The commit a branch was made from: the oldest entry of its reflog. */
async function startOf($: EngineInterface, repo: string, branch: string): Promise<string> {
  const reflog = await git($, repo, ['reflog', 'show', '--format=%H', `refs/heads/${branch}`, '--'])
  return reflog.stdout.split('\n').filter(line => line !== '').at(-1) ?? ''
}

export async function serveUnitWorktree($: EngineInterface, event: { readonly agentId?: string }, entry: ToolEntry): Promise<ToolCallResult> {
  const tool = entry.name
  const caller = callerIn(await readBook($), event.agentId)
  const job = caller.job
  const refusal = callerRefusal(entry, caller)
  if (refusal !== null || job === null || !isOpen(job)) {
    await fail($, { kind: 'caller-not-allowed', tool, caller: caller.kind })
    return deny(refusal ?? `${tool}: only the builder or an agent of an open job may call it, got ${caller.kind} of ${job === null ? 'no job' : `job ${job.id}, ${job.status}`}`)
  }
  const unit = field(argsOf(event), 'unit')
  if (typeof unit !== 'string' || !UNIT.test(unit) || RESERVED.includes(unit)) {
    const bad = refused(tool, 'unit must be one to three lower-case words of a-z and 0-9 joined by single hyphens, and not work or review', unit, 'call it again with a unit name such as parser or api-client')
    await fail($, { kind: 'bad-argument', tool, argument: 'unit', value: bad.refused })
    return deny(bad.refused)
  }
  const place = { repo: job.repo, branch: `${job.branch}--${unit}`, worktree: jobWorktree(job.repo, `${job.id}--${unit}`) }
  if (await hasWorktree($, place.repo, place.worktree)) {
    return answer([`worktree=${place.worktree}`, `branch=${place.branch}`, `start=${await startOf($, place.repo, place.branch)}`, 'existing=yes'])
  }
  const tip = await git($, job.repo, ['rev-parse', `refs/heads/${job.branch}`])
  if (tip.exitCode !== 0 || tip.stdout === '') {
    return deny(`${tool}: the job branch ${job.branch} has no tip, got exit ${String(tip.exitCode)}: ${tip.firstError}; call rex_status, and report the job blocked if its branch is gone`)
  }
  const added = await addWorktree($, place, tip.stdout)
  if (added !== null) {
    return deny(`${tool}: ${added.command} failed for unit ${unit}, got exit ${String(added.exitCode)}: ${added.firstError}; correct what it names, then call it again`)
  }
  return answer([`worktree=${place.worktree}`, `branch=${place.branch}`, `start=${tip.stdout}`, 'existing=no'])
}

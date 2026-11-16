// The sub-commands of /rex that read and show: the pane, a job's or a tree's status, the outline of the run, the jobs, the doctor's check, and the prompt mode.
import type { CommandRunInput, CommandRunResult, EngineInterface } from 'claude-code'

import { fail, mainJob, newestOpenJob, pluginVersion, setPromptMode } from '../book/index'
import type { RexBook, RexJob } from '../../types'
import { answerCommand, treeJobLine, treeLine } from '../pane/band'
import { isLiveNode, liveTrees, outlineOf, outlineText } from '../pane/outline'
import type { OutlineNode } from '../pane/outline'

const PANE_ID = 'rex'
/** The cells the transcript takes before a command's answer: `  ⎿  super-rex: `. */
const RESULT_INDENT = 16
const MODES: readonly RexBook['prompts'][] = ['off', 'observe', 'improve']

/** One `/rex` line: the event, what follows the sub-command's name, and the book as it stood. */
export type Request = { readonly event: CommandRunInput; readonly args: string; readonly book: RexBook }

function usd(nano: number): string {
  return (nano / 1e9).toFixed(2)
}

function lines(rows: readonly string[]): CommandRunResult {
  return { text: rows.map(row => `${row}\n`).join('') }
}

/** The job named, else the main loop's open job; with neither, a live tree speaks before the session's newest open job. */
function jobFor(book: RexBook, id: string): RexJob | undefined {
  if (id !== '') {
    return book.jobs.find(job => job.id === id)
  }
  return mainJob(book)
}

/** A live tree in plain lines: the tree, each open job with where it builds and what its builder does, then what waits on the person. */
function treeStatusRows(book: RexBook, node: OutlineNode): string[] {
  const jobs = node.children.filter(isLiveNode).flatMap(child => {
    const job = book.jobs.find(each => each.id === child.ref)
    return [treeJobLine(child), `    in ${job?.worktree ?? job?.repo ?? 'no folder yet'}`]
  })
  const waits = node.questions.map(question => `? ${question.id}: ${question.question} · recommended ${question.recommendation} · ${answerCommand(question)}`)
  return [treeLine(node), ...jobs, ...(waits.length === 0 ? ['nothing waits on you'] : waits)]
}

export async function watch($: EngineInterface): Promise<CommandRunResult> {
  await $.ui.open({ id: PANE_ID, title: 'SuperRex' })
  return {}
}

export function statusRows(book: RexBook, job: RexJob): string[] {
  const runs = book.runs.filter(run => run.job === job.id)
  return [
    `job=${job.id}`,
    `status=${job.status}`,
    `origin=${job.origin}`,
    `agents_open=${String(runs.filter(run => run.state === 'open').length)}`,
    `agents_done=${String(runs.filter(run => run.state !== 'open').length)}`,
    `spent_usd=${usd(job.spentNano)}`,
    `limit_usd=${usd(job.limit.nano)}`,
    `degraded=${job.degraded.length === 0 ? 'none' : job.degraded.join(',')}`,
  ]
}

export function status({ book, args }: Request, now: number): CommandRunResult {
  const job = jobFor(book, args)
  if (job !== undefined) {
    return lines(statusRows(book, job))
  }
  if (args !== '') {
    return lines([`rex status: no job ${args} in this session, got ${args}; run /rex jobs`])
  }
  const tree = liveTrees(book, outlineOf(book, now))[0]
  if (tree !== undefined) {
    return lines(treeStatusRows(book, tree))
  }
  const newest = newestOpenJob(book)
  return lines(newest === undefined ? ['job=none'] : statusRows(book, newest))
}

/** The outline of the session as text with tree guides: each master tree, its jobs and their agents, by level. */
export function tree({ book, event }: Request, now: number): CommandRunResult {
  return lines(outlineText(outlineOf(book, now), event.presentation.columns - RESULT_INDENT))
}

/** One line per job of the store's answer: id, status, origin, builder, request. */
export function jobLines(stdout: string): string[] {
  const parsed: unknown = JSON.parse(stdout)
  const jobs: unknown = typeof parsed === 'object' && parsed !== null ? Reflect.get(parsed, 'jobs') : undefined
  if (!Array.isArray(jobs)) {
    return [`rex jobs: the store's answer holds no jobs list, got ${stdout.slice(0, 80)}`]
  }
  return jobs.map((job: unknown) => {
    const field = (key: string): string => (typeof job === 'object' && job !== null ? String(Reflect.get(job, key)) : '')
    return `${field('id')} ${field('status')} ${field('origin')} ${field('builder_agent_id')} ${field('request')}`
  })
}

async function readFromBinary($: EngineInterface, argv: readonly string[]): Promise<{ readonly stdout: string } | { readonly refused: CommandRunResult }> {
  const result = await $.process.run(argv, { cwd: await $.session.root() })
  if (result.exitCode === 0) {
    return { stdout: result.stdout }
  }
  await fail($, { kind: 'binary-read-failed', argv, exitCode: result.exitCode, stderr: result.stderr })
  return { refused: { text: result.stderr.split('\n')[0] ?? `${argv.join(' ')} exited ${String(result.exitCode)}` } }
}

export async function jobs($: EngineInterface, { book }: Request): Promise<CommandRunResult> {
  if (book.storage === 'off') {
    return lines(book.jobs.map(job => `${job.id} ${job.status} ${job.origin} ${job.owner.builder} ${job.request}`))
  }
  const read = await readFromBinary($, ['rex', 'jobs', '--json'])
  return 'refused' in read ? read.refused : lines(jobLines(read.stdout))
}

export async function doctor($: EngineInterface): Promise<CommandRunResult> {
  const read = await readFromBinary($, ['rex', 'doctor'])
  if ('refused' in read) {
    return read.refused
  }
  return { text: `${read.stdout}mod=${await pluginVersion($)}\n` }
}

export async function prompts($: EngineInterface, { args }: Request): Promise<CommandRunResult> {
  const mode = MODES.find(each => each === args)
  if (mode === undefined) {
    return { text: `rex prompts: ${args === '' ? 'no mode' : args} is not a prompt mode, got ${JSON.stringify(args)}; use off, observe or improve` }
  }
  await setPromptMode($, mode)
  return lines([`prompts=${mode}`])
}

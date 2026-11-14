// The sub-commands of /rex that read and show: the pane, a job's status, the jobs, the doctor's check, and the prompt mode.
import type { CommandRunInput, CommandRunResult, EngineInterface } from 'claude-code'

import { fail, isOpen, pluginVersion, setPromptMode } from '../book/index'
import type { RexBook, RexJob } from '../../types'

const PANE_ID = 'rex'
const MODES: readonly RexBook['prompts'][] = ['off', 'observe', 'improve']

/** One `/rex` line: the event, what follows the sub-command's name, and the book as it stood. */
export type Request = { readonly event: CommandRunInput; readonly args: string; readonly book: RexBook }

function usd(nano: number): string {
  return (nano / 1e9).toFixed(2)
}

function lines(rows: readonly string[]): CommandRunResult {
  return { text: rows.map(row => `${row}\n`).join('') }
}

/** The job a person most likely means: the one named, else the main loop's open job, else the newest open job. */
function jobFor(book: RexBook, id: string): RexJob | undefined {
  if (id !== '') {
    return book.jobs.find(job => job.id === id)
  }
  const open = book.jobs.filter(job => isOpen(job) && job.owner.session === book.session)
  return open.find(job => job.owner.builder === 'main') ?? [...open].sort((a, b) => b.createdAt - a.createdAt)[0]
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

export function status({ book, args }: Request): CommandRunResult {
  const job = jobFor(book, args)
  if (job === undefined) {
    return lines([args === '' ? 'job=none' : `rex status: no job ${args} in this session, got ${args}; run /rex jobs`])
  }
  return lines(statusRows(book, job))
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

// The band above the prompt as the short form of the outline: a line for each live tree and its open jobs, the main loop's job, and the oldest waiting question.
// Pure, and within the rows and columns the band has: job lines go first when rows are short.
import type { RexBook, RexJob, RexQuestion } from '../../types'
import { bandJob } from '../book/index'
import { cut, deepestLevel, isLiveNode, liveTrees, plural, usd } from './outline'
import type { OutlineNode } from './outline'

export type BandRow = { readonly kind: 'head' | 'job' | 'ask'; readonly text: string }

/** The band's short form of spend against a limit, so a tree's line fits a 100-column terminal. */
function spendOf(spentNano: number, capNano: number | null): string {
  return `${usd(spentNano)}/${usd(capNano ?? 0)}`
}

/** A tree in one line: its state, the jobs it delivered of all its jobs, its open agents and the deepest level at work, its spend of its cap. */
export function treeLine(node: OutlineNode): string {
  const delivered = node.children.filter(job => job.status === 'delivered').length
  const deepest = deepestLevel(node, each => each.kind !== 'tree' && isLiveNode(each))
  const working = deepest === null ? '' : `, deepest ${deepest}`
  return `rex master ${node.ref} ${node.status} · ${String(delivered)}/${String(node.children.length)} delivered · ${plural(node.openBelow, 'agent', 'agents')} open${working} · ${spendOf(node.spentNano, node.capNano)}`
}

/** A job of a tree in one line: what its builder is doing, by the roles of its open leads and the count of their open workers. */
export function treeJobLine(node: OutlineNode): string {
  const leads = node.children.filter(isLiveNode).map(lead => (lead.name === '' ? lead.level : lead.name))
  const workers = node.children.flatMap(lead => lead.children).filter(isLiveNode).length
  const doing = leads.length === 0 ? 'no lead open' : `leads ${leads.join(', ')}`
  return `  builder ${node.ref} ${node.status} · ${doing} · ${plural(workers, 'worker', 'workers')} open · ${spendOf(node.spentNano, node.capNano)}`
}

function jobLine(book: RexBook, job: RexJob, node: OutlineNode): string {
  const deepest = deepestLevel(node, each => each.kind === 'agent') ?? 'builder'
  const asks = book.permissionAsks === 0 ? [] : [plural(book.permissionAsks, 'permission prompt', 'permission prompts')]
  return [`rex builder ${job.id} ${job.status}`, `${String(node.doneBelow)}/${plural(node.openBelow + node.doneBelow, 'agent', 'agents')} done`, `deepest ${deepest}`, spendOf(job.spentNano, job.limit.nano), ...asks].join(' · ')
}

export function oldestWaiting(book: RexBook): readonly RexQuestion[] {
  return [...book.questions.filter(question => question.answer === null)].sort((a, b) => a.askedAt - b.askedAt)
}

/** The command that answers a question by its option number. */
export function answerCommand(question: RexQuestion): string {
  return `/rex answer ${question.id} <1-${String(question.options.length)}>`
}

function askLine(book: RexBook, columns: number): string | null {
  const waiting = oldestWaiting(book)
  const oldest = waiting[0]
  if (oldest === undefined) {
    return null
  }
  const head = `? ${plural(waiting.length, 'question waits', 'questions wait')} on you · `
  const command = ` · ${answerCommand(oldest)}`
  return `${head}${cut(oldest.question, Math.max(10, columns - head.length - command.length))}${command}`
}

function fit(rows: readonly BandRow[], maxRows: number): readonly BandRow[] {
  const kept = [...rows]
  for (let index = kept.length - 1; kept.length > maxRows && index >= 0; index -= 1) {
    if (kept[index]?.kind === 'job') {
      kept.splice(index, 1)
    }
  }
  if (kept.length <= maxRows) {
    return kept
  }
  const ask = kept.filter(row => row.kind === 'ask')
  const heads = kept.filter(row => row.kind === 'head')
  return maxRows <= ask.length ? heads.slice(0, maxRows) : [...heads.slice(0, maxRows - ask.length), ...ask]
}

/** No rows when no tree is live and no job is open. */
export function bandRows(book: RexBook, outline: OutlineNode, maxRows: number, columns: number): readonly BandRow[] {
  const rows: BandRow[] = []
  for (const tree of liveTrees(book, outline)) {
    rows.push({ kind: 'head', text: treeLine(tree) })
    for (const job of tree.children.filter(isLiveNode)) {
      rows.push({ kind: 'job', text: treeJobLine(job) })
    }
  }
  const job = bandJob(book)
  const jobNode = outline.children.find(node => node.kind === 'job' && node.ref === job?.id)
  if (job !== undefined && jobNode !== undefined) {
    rows.push({ kind: 'head', text: jobLine(book, job, jobNode) })
  }
  if (rows.length === 0) {
    return []
  }
  const ask = askLine(book, columns)
  if (ask !== null) {
    rows.push({ kind: 'ask', text: ask })
  }
  return fit(rows, maxRows).map(row => ({ kind: row.kind, text: cut(row.text, columns) }))
}

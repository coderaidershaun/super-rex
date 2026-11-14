// What each view of the watch pane shows, drawn from the book alone, so the same book always draws the same rows.
import type { RexBook, RexJob, RexRun } from '../../types'
import type { PaneElements, PaneView, PaneViewContext } from '../parts'

type Drawn = ReturnType<PaneElements['Box']>

export function usd(nano: number): string {
  return `$${(nano / 1e9).toFixed(2)}`
}

function tokenCount(run: RexRun): number {
  return run.tokens.input + run.tokens.output + run.tokens.cacheRead + run.tokens.cacheCreation
}

export function maxDepth(book: RexBook): number {
  return book.runs.reduce((deepest, run) => Math.max(deepest, run.depth), 0)
}

export function jobRuns(book: RexBook, job: RexJob): readonly RexRun[] {
  return book.runs.filter(run => run.job === job.id)
}

export function waitingQuestions(book: RexBook): number {
  return book.questions.filter(question => question.answer === null).length
}

function header({ elements, book }: PaneViewContext): Drawn {
  const { Box, Text } = elements
  const state = book.inert === null ? `storage ${book.storage}` : `off: ${book.inert}`
  const { mod, binary, claudeCode } = book.versions
  return (
    <Box flexDirection="column">
      <Text bold>SuperRex</Text>
      <Text dimColor>
        {state} · prompts {book.prompts} · depth {maxDepth(book)} · {book.jobs.length} jobs · {waitingQuestions(book)} questions
      </Text>
      <Text dimColor>
        mod {mod} · binary {binary ?? 'unknown'} · Claude Code {claudeCode ?? 'unknown'}
      </Text>
    </Box>
  )
}

function jobLine(book: RexBook, job: RexJob): string {
  const runs = jobRuns(book, job)
  const open = runs.filter(run => run.state === 'open').length
  const orchestrator = job.plan?.orchestrator ?? 'no plan'
  const asked = book.questions.filter(question => question.answer === null && (question.job === job.id || question.mergeFor === job.id)).length
  const degraded = job.degraded.length === 0 ? '' : ` · degraded ${job.degraded.join(',')}`
  return `${job.id} ${job.status} · ${orchestrator} · agents ${String(open)}/${String(runs.length)} · ${usd(job.spentNano)}/${usd(job.limit.nano)} · ${String(asked)} questions${degraded}`
}

function jobsView({ elements, book }: PaneViewContext): Drawn {
  const { Box, Text } = elements
  return (
    <Box flexDirection="column">
      {book.jobs.length === 0 && <Text dimColor>No jobs yet.</Text>}
      {book.jobs.map(job => (
        <Text>{jobLine(book, job)}</Text>
      ))}
    </Box>
  )
}

function runLine(run: RexRun): string {
  const ran = run.ran === null ? 'not run' : `${run.ran.model}/${run.ran.effort ?? 'none'}`
  const marks = run.marks.length === 0 ? '' : ` · ${run.marks.join(',')}`
  return `${'  '.repeat(run.depth - 1)}${run.agentType} ${run.agentId} · ${run.role ?? 'no role'} · planned ${run.planned ?? 'none'} · ran ${ran} · ${String(tokenCount(run))} tokens · ${usd(run.spentNano)} · ${run.state}${marks}`
}

/** The runs in tree order: each run below its parent, the main loop's children first. */
export function treeOrder(runs: readonly RexRun[]): readonly RexRun[] {
  const below = (parent: string): readonly RexRun[] =>
    runs.filter(run => run.parent === parent).flatMap(run => [run, ...below(run.agentId)])
  return below('main')
}

function treeView({ elements, book }: PaneViewContext): Drawn {
  const { Box, Text } = elements
  const ordered = treeOrder(book.runs)
  return (
    <Box flexDirection="column">
      {book.trees.map(tree => (
        <Text bold>
          master tree {tree.id} {tree.state} · ledger {usd(tree.ledgerNano)}
        </Text>
      ))}
      {ordered.length === 0 && <Text dimColor>No agents yet.</Text>}
      {ordered.map(run => (
        <Text>{runLine(run)}</Text>
      ))}
    </Box>
  )
}

function questionsView({ elements, book, actions }: PaneViewContext): Drawn {
  const { Box, Text, Button } = elements
  const waiting = book.questions.filter(question => question.answer === null)
  const answered = book.questions.filter(question => question.answer !== null)
  return (
    <Box flexDirection="column">
      {book.questions.length === 0 && <Text dimColor>No questions.</Text>}
      {waiting.map(question => (
        <Box flexDirection="column">
          <Text>
            {question.id}: {question.question}
          </Text>
          <Text dimColor>recommended: {question.recommendation}</Text>
          <Box flexDirection="row" gap={1}>
            {question.options.map((option, index) => (
              <Button key={`answer-${question.id}-${String(index)}`} label={option} onPress={() => { actions.answer(question.id, option) }} />
            ))}
          </Box>
        </Box>
      ))}
      {answered.map(question => (
        <Text dimColor>
          {question.id}: {question.question} answered {question.answer?.option ?? ''} ({question.answer?.source ?? ''})
        </Text>
      ))}
    </Box>
  )
}

/** The pane's own views; `header` stands above every other view, the rest are tabs in order. */
export const HEADER_VIEW: PaneView = { name: 'header', title: 'SuperRex', order: 10, draw: header }
export const JOBS_VIEW: PaneView = { name: 'jobs', title: 'Jobs', order: 20, draw: jobsView }
export const TREE_VIEW: PaneView = { name: 'tree', title: 'Tree', order: 30, draw: treeView }
export const QUESTIONS_VIEW: PaneView = { name: 'questions', title: 'Questions', order: 50, draw: questionsView }

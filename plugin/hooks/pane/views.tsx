// The header and the questions view of the watch pane, drawn from the book alone, so the same book always draws the same rows.
// The questions view gives the digits to the options of the oldest waiting question, so the tab's key and one digit answer it.
import type { PaneElements, PaneView, PaneViewContext } from '../parts'
import { deepestLevel, plural } from './outline'

type Drawn = ReturnType<PaneElements['Box']>

const DIGITS = 9

function header({ elements, book, outline }: PaneViewContext): Drawn {
  const { Box, Text } = elements
  const state = book.inert === null ? `storage ${book.storage}` : `off: ${book.inert}`
  const { mod, binary, claudeCode } = book.versions
  const deepest = deepestLevel(outline, node => node.kind !== 'session') ?? 'none'
  const waiting = book.questions.filter(question => question.answer === null).length
  return (
    <Box flexDirection="column">
      <Text bold>SuperRex</Text>
      <Text dimColor>
        {state} · prompts {book.prompts} · {plural(book.jobs.length, 'job', 'jobs')} · deepest level {deepest} · {plural(waiting, 'question waits', 'questions wait')}
      </Text>
      <Text dimColor>
        mod {mod} · binary {binary ?? 'unknown'} · Claude Code {claudeCode ?? 'unknown'}
      </Text>
    </Box>
  )
}

function questionsView({ elements, book, actions }: PaneViewContext): Drawn {
  const { Box, Text, Button } = elements
  const waiting = [...book.questions.filter(question => question.answer === null)].sort((a, b) => a.askedAt - b.askedAt)
  const answered = book.questions.filter(question => question.answer !== null)
  return (
    <Box flexDirection="column">
      {book.questions.length === 0 && <Text dimColor>No questions.</Text>}
      {waiting.map((question, order) => (
        <Box flexDirection="column">
          <Text color="warning">
            ? {question.id}: {question.question}
          </Text>
          <Text dimColor>
            for {question.job === null ? `tree ${question.tree ?? 'none'}` : `job ${question.job}`} · recommended: {question.recommendation} · {order === 0 ? 'press a digit, or ' : ''}type /rex answer {question.id} &lt;1-{String(question.options.length)}&gt;
          </Text>
          <Box flexDirection="row" gap={2}>
            {question.options.map((option, index) => (
              <Button
                key={`answer-${question.id}-${String(index)}`}
                plain
                {...(order === 0 && index < DIGITS ? { hotkey: String(index + 1), label: option } : { label: `${String(index + 1)}. ${option}` })}
                onPress={() => { actions.answer(question.id, option) }}
              />
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

/** The pane's own views besides the tree; `header` stands above every other view, the rest are tabs in order. */
export const HEADER_VIEW: PaneView = { name: 'header', title: 'SuperRex', order: 10, draw: header }
export const QUESTIONS_VIEW: PaneView = { name: 'questions', title: 'Questions', order: 50, draw: questionsView }

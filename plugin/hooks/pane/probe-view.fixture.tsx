// A pane view that only a test adds, as another module would add one: it draws from the pane's context and answers through its actions.
import type { PaneElements, PaneView, PaneViewContext } from '../parts'

function probe({ elements, book, columns, actions }: PaneViewContext): ReturnType<PaneElements['Box']> {
  const { Box, Text, Button } = elements
  return (
    <Box flexDirection="column">
      <Text>
        probe sees {book.jobs.length} jobs at {columns} columns
      </Text>
      <Button key="probe-answer" label="Answer q1" onPress={() => { actions.answer('q1', 'Merge') }} />
    </Box>
  )
}

export const PROBE_VIEW: PaneView = { name: 'probe', title: 'Probe', order: 45, draw: probe }

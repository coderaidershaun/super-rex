// The decisions view of the watch pane: every recorded skill, answer and grid agent of each job, with its source and its one-line reason.
import type { PaneElements, PaneView, PaneViewContext } from '../parts'

function decisionsView({ elements, book }: PaneViewContext): ReturnType<PaneElements['Box']> {
  const { Box, Text } = elements
  const rows = book.jobs.flatMap(job => job.decisions.map(decision => ({ job: job.id, decision })))
  return (
    <Box flexDirection="column">
      {rows.length === 0 && <Text dimColor>No decisions yet.</Text>}
      {rows.map(({ job, decision }) => (
        <Text>
          {job} {decision.subject} {decision.item}={decision.value} · {decision.source}: {decision.reason}
        </Text>
      ))}
    </Box>
  )
}

export const DECISIONS_VIEW: PaneView = { name: 'decisions', title: 'Decisions', order: 40, draw: decisionsView }

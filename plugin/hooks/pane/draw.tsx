// Draws the whole watch pane from the pane-view table: the header view on top, a tab for each other view, and the chosen view below.
import type { RexBook } from '../../types'
import type { PaneActions, PaneElements, PaneView } from '../parts'

export const HEADER = 'header'

export type PaneDraw = {
  readonly elements: PaneElements
  readonly book: RexBook
  readonly columns: number
  readonly actions: PaneActions
  readonly views: readonly PaneView[]
  /** The name the person chose; a name no view has falls back to the first tab. */
  readonly chosen: string
  readonly choose: (name: string) => void
}

export function tabsOf(views: readonly PaneView[]): readonly PaneView[] {
  return [...views].filter(view => view.name !== HEADER).sort((a, b) => a.order - b.order)
}

export function drawPane(pane: PaneDraw): ReturnType<PaneElements['Box']> {
  const { Box, Button } = pane.elements
  const ctx = { elements: pane.elements, book: pane.book, columns: pane.columns, actions: pane.actions }
  const tabs = tabsOf(pane.views)
  const header = pane.views.find(view => view.name === HEADER)
  const shown = tabs.find(view => view.name === pane.chosen) ?? tabs[0]
  return (
    <Box flexDirection="column">
      {header?.draw(ctx)}
      <Box flexDirection="row" gap={1}>
        {tabs.map(view => (
          <Button key={`view-${view.name}`} label={view.name} variant={view === shown ? 'primary' : 'secondary'} onPress={() => { pane.choose(view.name) }} />
        ))}
      </Box>
      {shown?.draw(ctx)}
    </Box>
  )
}

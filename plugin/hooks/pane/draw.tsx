// Draws the whole watch pane from the pane-view table: the header view on top, a tab for each other view with a letter key, and the chosen view below.
import type { RexBook } from '../../types'
import type { PaneActions, PaneElements, PaneView } from '../parts'
import type { OutlineNode } from './outline'

export const HEADER = 'header'
/** The letters the tree view keeps for itself: `u` goes up, `n` turns the page of rows. */
const TREE_KEYS: readonly string[] = ['u', 'n']

export type PaneDraw = {
  readonly elements: PaneElements
  readonly book: RexBook
  readonly columns: number
  readonly isFocused: boolean
  readonly actions: PaneActions
  readonly at: string
  readonly page: number
  readonly outline: OutlineNode
  readonly views: readonly PaneView[]
  /** The name the person chose; a name no view has falls back to the first tab. */
  readonly chosen: string
  readonly choose: (name: string) => void
}

export function tabsOf(views: readonly PaneView[]): readonly PaneView[] {
  return [...views].filter(view => view.name !== HEADER).sort((a, b) => a.order - b.order)
}

/** Each tab's key: the first letter of its name that no earlier tab and no key of the tree view took; none when every letter is taken. */
export function tabKeys(tabs: readonly PaneView[]): ReadonlyMap<string, string> {
  const taken = new Set(TREE_KEYS)
  const keys = new Map<string, string>()
  for (const tab of tabs) {
    const key = [...tab.name.toLowerCase()].find(letter => /^[a-z]$/.test(letter) && !taken.has(letter))
    if (key !== undefined) {
      taken.add(key)
      keys.set(tab.name, key)
    }
  }
  return keys
}

export function drawPane(pane: PaneDraw): ReturnType<PaneElements['Box']> {
  const { Box, Text, Button } = pane.elements
  const ctx = { elements: pane.elements, book: pane.book, columns: pane.columns, actions: pane.actions, at: pane.at, page: pane.page, outline: pane.outline }
  const tabs = tabsOf(pane.views)
  const keys = tabKeys(tabs)
  const header = pane.views.find(view => view.name === HEADER)
  const shown = tabs.find(view => view.name === pane.chosen) ?? tabs[0]
  return (
    <Box flexDirection="column">
      {header?.draw(ctx)}
      <Box flexDirection="row" gap={2}>
        {tabs.map(view => {
          const key = keys.get(view.name)
          return <Button key={`view-${view.name}`} plain {...(key === undefined ? {} : { hotkey: key })} {...(view === shown ? {} : { dimColor: true })} label={view === shown ? `[${view.name}]` : view.name} onPress={() => { pane.choose(view.name) }} />
        })}
      </Box>
      {!pane.isFocused && <Text dimColor>ctrl+x tab: give this pane the keys</Text>}
      {shown?.draw(ctx)}
    </Box>
  )
}

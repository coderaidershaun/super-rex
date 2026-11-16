// Draws the band above the prompt and the watch pane from the outline of the live record, and keeps the pane's tab, the node in view and the page of its rows.
import { atom, read, update } from 'claude-code'
import type { EngineInterface, On } from 'claude-code'

import { answerQuestion, errorText, fail, readBook, setVersions } from '../book/index'
import { wakeOnAnswer } from '../master/ask'
import type { Parts } from '../parts'
import { bandRows } from './band'
import { DECISIONS_VIEW } from './decisions'
import { drawPane } from './draw'
import { TREE_VIEW } from './drill'
import { outlineOf, ROOT_ID } from './outline'
import { HEADER_VIEW, QUESTIONS_VIEW } from './views'

export const PANE_ID = 'rex'

const viewAtom = atom({ plugin: 'super-rex', key: 'view' } as const, 'tree')
const atAtom = atom({ plugin: 'super-rex', key: 'at' } as const, ROOT_ID)
const pageAtom = atom({ plugin: 'super-rex', key: 'page' } as const, 0)

/** A drawing cannot write the book, so a failed drawing is recorded from a timer once the drawing is over. */
function recordDrawFailure($: EngineInterface, site: string, error: string): void {
  $.clock.after(0, () => {
    void fail($, { kind: 'observer-failed', event: `ui.render:${site}`, module: 'pane', agentId: null, error }).catch((again: unknown) => {
      $.ui.status(`rex: the pane could not record its own failure: ${errorText(again)}`)
    })
  })
}

/** The binary's version for the header, read once per session; a binary that does not answer leaves it unknown. */
async function readBinaryVersion($: EngineInterface): Promise<void> {
  const argv = ['rex', '--version']
  try {
    const result = await $.process.run(argv)
    const version = /^rex (\S+)\n?$/.exec(result.stdout)?.[1]
    if (result.exitCode === 0 && version !== undefined) {
      await setVersions($, { binary: version })
      return
    }
    await fail($, { kind: 'binary-read-failed', argv, exitCode: result.exitCode, stderr: result.stderr })
  } catch (error) {
    await fail($, { kind: 'binary-read-failed', argv, exitCode: -1, stderr: errorText(error) })
  }
}

/** A press records the answer, then wakes whoever waits on it, as the dialog and /rex answer do; a question answered before wakes nobody again. */
async function answerFromPane($: EngineInterface, id: string, option: string): Promise<void> {
  const question = (await readBook($)).questions.find(each => each.id === id)
  if (question === undefined || question.answer !== null) {
    return
  }
  await answerQuestion($, id, { option, source: 'pane' })
  await wakeOnAnswer($, question, option)
}

export function register(on: On, parts: Parts): void {
  for (const view of [HEADER_VIEW, TREE_VIEW, DECISIONS_VIEW, QUESTIONS_VIEW]) {
    parts.paneViews.add(view)
  }

  on('session.start', { cwd: /(?:)/ }, async ($, e, next) => {
    const book = await readBook($)
    if (book.inert === null) {
      await readBinaryVersion($)
    }
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'session.start', module: 'pane', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('ui.render', { component: 'AbovePrompt' }, async ($, e, next) => {
    const book = await readBook($)
    if (book.inert !== null || e.props.hasSurvey) {
      return next(e)
    }
    const rows = bandRows(book, outlineOf(book, await $.clock.now()), e.props.maxRows, e.props.bodyColumns)
    if (rows.length === 0) {
      return next(e)
    }
    const { Box, Text } = $.ui.resolve(e)
    return (
      <Box flexDirection="column">
        {rows.map(row => (row.kind === 'ask' ? <Text color="warning">{row.text}</Text> : <Text dimColor={row.kind === 'job'}>{row.text}</Text>))}
      </Box>
    )
  }).catch(($, e, next) => {
    recordDrawFailure($, 'AbovePrompt', next.error.message ?? next.error.kind)
    return next(e)
  })

  on('ui.render', { component: 'Pane', requestId: PANE_ID }, async ($, e) => {
    const book = await readBook($)
    const chosen = await read($, viewAtom)
    const at = await read($, atAtom)
    const page = await read($, pageAtom)
    const { Box, Text, Button } = $.ui.resolve(e)
    const answer = (question: string, option: string): void => {
      void answerFromPane($, question, option).catch((error: unknown) => {
        $.ui.status(`rex: the answer to ${question} was not recorded: ${errorText(error)}`)
      })
    }
    const choose = (name: string): void => {
      void update($, viewAtom, () => name).catch((error: unknown) => {
        $.ui.status(`rex: the pane could not change its view: ${errorText(error)}`)
      })
    }
    const go = (node: string): void => {
      void Promise.all([update($, atAtom, () => node), update($, pageAtom, () => 0)]).catch((error: unknown) => {
        $.ui.status(`rex: the pane could not go to ${node}: ${errorText(error)}`)
      })
    }
    const turn = (next: number): void => {
      void update($, pageAtom, () => next).catch((error: unknown) => {
        $.ui.status(`rex: the pane could not show page ${String(next + 1)} of its rows: ${errorText(error)}`)
      })
    }
    const elements = { Box, Text, Button }
    const outline = outlineOf(book, await $.clock.now())
    return drawPane({ elements, book, columns: e.props.bodyColumns, isFocused: e.props.isFocused, actions: { answer, go, turn }, at, page, outline, views: parts.paneViews.entries(), chosen, choose })
  }).catch(($, e, next) => {
    recordDrawFailure($, 'Pane', next.error.message ?? next.error.kind)
    const { Text } = $.ui.resolve(e)
    return <Text dimColor>rex: the pane could not be drawn ({next.error.message ?? next.error.kind})</Text>
  })
}

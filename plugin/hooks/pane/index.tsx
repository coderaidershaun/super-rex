// Draws the band above the prompt and the watch pane from the live record.
import { atom, read, update } from 'claude-code'
import type { EngineInterface, On } from 'claude-code'

import { answerQuestion, errorText, fail, isOpen, readBook, setVersions } from '../book/index'
import type { RexBook, RexJob } from '../../types'
import type { Parts } from '../parts'
import { DECISIONS_VIEW } from './decisions'
import { drawPane } from './draw'
import { HEADER_VIEW, JOBS_VIEW, jobRuns, maxDepth, QUESTIONS_VIEW, TREE_VIEW, usd, waitingQuestions } from './views'

export const PANE_ID = 'rex'

const viewAtom = atom({ plugin: 'super-rex', key: 'view' } as const, 'jobs')

/** The job the band speaks of: the main loop's open job, else the newest open job of the session. */
function bandJob(book: RexBook): RexJob | undefined {
  const open = book.jobs.filter(job => isOpen(job) && job.owner.session === book.session)
  return open.find(job => job.owner.builder === 'main') ?? [...open].sort((a, b) => b.createdAt - a.createdAt)[0]
}

export function bandLine(book: RexBook, job: RexJob): string {
  const runs = jobRuns(book, job)
  const done = runs.filter(run => run.state !== 'open').length
  const questions = waitingQuestions(book)
  const asks = book.permissionAsks
  return `rex ${job.id} ${job.status} · ${String(done)}/${String(runs.length)} agents · ${usd(job.spentNano)}/${usd(job.limit.nano)} · depth ${String(maxDepth(book))} · ${String(questions)} ${questions === 1 ? 'question' : 'questions'} · ${String(asks)} ${asks === 1 ? 'permission prompt' : 'permission prompts'}`
}

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

export function register(on: On, parts: Parts): void {
  for (const view of [HEADER_VIEW, JOBS_VIEW, TREE_VIEW, DECISIONS_VIEW, QUESTIONS_VIEW]) {
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
    const job = bandJob(book)
    if (book.inert !== null || job === undefined || e.props.hasSurvey) {
      return next(e)
    }
    const { Box, Text } = $.ui.resolve(e)
    return (
      <Box>
        <Text dimColor>{bandLine(book, job)}</Text>
      </Box>
    )
  }).catch(($, e, next) => {
    recordDrawFailure($, 'AbovePrompt', next.error.message ?? next.error.kind)
    return next(e)
  })

  on('ui.render', { component: 'Pane', requestId: PANE_ID }, async ($, e) => {
    const book = await readBook($)
    const chosen = await read($, viewAtom)
    const { Box, Text, Button } = $.ui.resolve(e)
    const answer = (question: string, option: string): void => {
      void answerQuestion($, question, { option, source: 'pane' }).catch((error: unknown) => {
        $.ui.status(`rex: the answer to ${question} was not recorded: ${errorText(error)}`)
      })
    }
    const choose = (name: string): void => {
      void update($, viewAtom, () => name).catch((error: unknown) => {
        $.ui.status(`rex: the pane could not change its view: ${errorText(error)}`)
      })
    }
    const elements = { Box, Text, Button }
    return drawPane({ elements, book, columns: e.props.bodyColumns, actions: { answer }, views: parts.paneViews.entries(), chosen, choose })
  }).catch(($, _e, next) => {
    recordDrawFailure($, 'Pane', next.error.message ?? next.error.kind)
    const { Text } = $.ui.resolve(_e)
    return <Text dimColor>rex: the pane could not be drawn ({next.error.message ?? next.error.kind})</Text>
  })
}

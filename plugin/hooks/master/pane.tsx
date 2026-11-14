// The master view of the watch pane: each tree with its state, pinned policy and spend, the jobs its master built, and the decisions it noted.
import type { RexBook, RexJob, RexTree } from '../../types'
import type { PaneElements, PaneView, PaneViewContext } from '../parts'
import { treeJobs, treeSpentNano } from './gates'

type Drawn = ReturnType<PaneElements['Box']>

/** Enough of `sha256:<hex>` to tell two policies apart at a glance. */
const HASH_SHOWN = 15

function dollars(nano: number): string {
  return `$${(nano / 1e9).toFixed(2)}`
}

function treeLine(book: RexBook, tree: RexTree): string {
  return `master tree ${tree.id} ${tree.state} · master ${tree.master ?? 'none'} · policy ${tree.policyHash.slice(0, HASH_SHOWN)} · ${dollars(treeSpentNano(book, tree))} of ${dollars(tree.capNano)}`
}

function charterLine(tree: RexTree): string {
  const first = tree.charter.split('\n').find(line => line.trim() !== '') ?? ''
  return `charter: ${first.trim()}`
}

function jobLine(tree: RexTree, job: RexJob): string {
  const builder = tree.builders.find(each => each.job === job.id)?.agentId ?? 'none'
  const merge = job.merge === null ? 'not answered' : `${job.merge.answer} (${job.merge.source})`
  const after = job.mergeAfter.length === 0 ? '' : ` · after ${job.mergeAfter.join(',')}`
  const failed = job.doneChecks.filter(check => check.result === 'fail').map(check => check.check)
  const done = job.doneChecks.length === 0 ? '' : ` · done check ${failed.length === 0 ? 'passed' : `failed ${failed.join(',')}`}`
  return `${job.id} ${job.status} · builder ${builder} · ${dollars(job.spentNano)}/${dollars(job.limit.nano)} · merge ${merge}${after}${done}`
}

function masterView({ elements, book }: PaneViewContext): Drawn {
  const { Box, Text } = elements
  return (
    <Box flexDirection="column">
      {book.trees.length === 0 && <Text dimColor>No master tree. Start one with /rex master &lt;goal&gt;.</Text>}
      {book.trees.map(tree => (
        <Box flexDirection="column">
          <Text bold>{treeLine(book, tree)}</Text>
          <Text dimColor>{charterLine(tree)}</Text>
          {treeJobs(book, tree.id).map(job => (
            <Text>{jobLine(tree, job)}</Text>
          ))}
          {tree.notes.map(note => (
            <Text dimColor>
              note: {note.job === null ? '' : `${note.job}: `}
              {note.text}
            </Text>
          ))}
        </Box>
      ))}
    </Box>
  )
}

export const MASTER_VIEW: PaneView = { name: 'master', title: 'Master', order: 35, draw: masterView }

// The lines of rex_status: each status section of the table in its order, so other modules add lines without editing this one.
import type { RexBook, RexJob } from '../../types'
import type { Caller, StatusContext, StatusSection, Table } from '../parts'
import { usd } from './text'

export function statusLines(book: RexBook, job: RexJob | null, caller: Caller, sections: Table<StatusSection>): readonly string[] {
  const context: StatusContext = { book, job, caller }
  const ordered = [...sections.entries()].sort((a, b) => a.order - b.order)
  return ordered.flatMap(section => section.lines(context))
}

/** Where a job is in its life: before its plan, while it runs, or closed. */
function phaseOf(job: RexJob): 'plan' | 'run' | 'closed' {
  switch (job.status) {
    case 'created':
      return 'plan'
    case 'planned':
      return 'run'
    default:
      return 'closed'
  }
}

export const OWN_SECTIONS: readonly StatusSection[] = [
  {
    name: 'job',
    order: 10,
    lines: ({ job }) => (job === null ? ['job=none'] : [`job=${job.id}`, `status=${job.status}`, `phase=${phaseOf(job)}`, `origin=${job.origin}`]),
  },
  {
    name: 'agents',
    order: 20,
    lines: ({ book, job }) => {
      if (job === null) {
        return []
      }
      const runs = book.runs.filter(run => run.job === job.id)
      const open = runs.filter(run => run.state === 'open').length
      return [`agents_open=${String(open)}`, `agents_done=${String(runs.length - open)}`]
    },
  },
  {
    name: 'spend',
    order: 30,
    lines: ({ job }) =>
      job === null
        ? []
        : [`spent_usd=${usd(job.spentNano)}`, `limit_usd=${usd(job.limit.nano)}`, `degraded=${job.degraded.length === 0 ? 'none' : job.degraded.join(',')}`],
  },
]

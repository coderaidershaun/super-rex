// Proves rex_status: the sections in their order, and a section another module adds called with the status context.
import { expect, test } from 'claude-code/testing'

import { createParts } from '../parts'
import { JOB, seededBook } from './act-kit'
import { callerIn } from './caller'
import { PROBE_SECTION } from './probe-section.fixture'
import { OWN_SECTIONS, statusLines } from './status'

test('a status section another module adds stands between the others by its order and reads the status context', () => {
  const parts = createParts()
  parts.statusSections.add(PROBE_SECTION)
  for (const section of OWN_SECTIONS) {
    parts.statusSections.add(section)
  }
  const book = seededBook()
  const job = book.jobs[0] ?? null

  const lines = statusLines(book, job, callerIn(book, undefined), parts.statusSections)

  expect(lines).toEqual([
    `job=${JOB}`,
    'status=created',
    'phase=plan',
    'origin=composer',
    'agents_open=0',
    'agents_done=0',
    'probe.caller=builder',
    `probe.job=${JOB}`,
    'probe.runs=0',
    'spent_usd=0.00',
    'limit_usd=10.00',
    'degraded=none',
  ])
})

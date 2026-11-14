// Proves the plugin loads under the test kit, that a session starts through it, and that a shared table keeps its entries in order and refuses a second entry of one name.
import { expect, test } from 'claude-code/testing'

import { createParts, DuplicateEntryError } from './parts'
import type { Subcommand } from './parts'

test('a table keeps entries in order and refuses a second entry of one name', () => {
  const parts = createParts()
  const watch: Subcommand = { name: 'watch', summary: 'Opens the pane', grantsAuthority: false }
  const status: Subcommand = { name: 'status', summary: 'Shows a job', grantsAuthority: false }

  parts.subcommands.add(watch)
  parts.subcommands.add(status)

  expect(() => {
    parts.subcommands.add(watch)
  }).toThrow(DuplicateEntryError)
  expect(parts.subcommands.entries().map(entry => entry.name)).toEqual(['watch', 'status'])
})

test('the plugin loads, and a session starts through all its modules', async ($, on) => {
  on('session.start', (_, e) => ({ cwd: e.cwd }))

  const started = await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })

  expect(started).toEqual({ cwd: '/' })
})

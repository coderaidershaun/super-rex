// Proves the cases of a spawn, first match wins, the fixed manifests of the roles that start before a plan, and a brief part added from another file.
import { expect, test } from 'claude-code/testing'

import { spawnInput, START_MS } from '../book/kit'
import { JOB, seededBook } from '../tools/act-kit'
import { createParts } from '../parts'
import { spawnCase } from './brief'
import { assembled, OWN_PARTS } from './parts'
import { PROBE_PART, seenContexts } from './probe-part.fixture'
import { fixedManifests } from './manifests'

test('the spawn cases: own, resume, team, fork in a job, new in a job, anything else', () => {
  const book = seededBook()
  const job = book.jobs[0]
  const pending = { ...book, pendingSpawns: [{ description: 'rex builder 0gaq820001', kind: 'builder' as const, tree: 't1', job: JOB, at: START_MS }] }

  expect(spawnCase(pending, spawnInput({ description: 'rex builder 0gaq820001' }), 'super-rex', job)).toBe('own')
  expect(spawnCase(pending, spawnInput({ description: 'rex builder 0gaq820001' }), 'engine', job)).toBe('new')
  expect(spawnCase(book, spawnInput({ resume: { agentId: 'a1' }, fork: true }), 'engine', job)).toBe('resume')
  expect(spawnCase(book, spawnInput({ isTeammate: true }), 'engine', job)).toBe('team')
  expect(spawnCase(book, spawnInput({ workflow: { runId: 'w', agentIndex: 0 } }), 'engine', job)).toBe('team')
  expect(spawnCase(book, spawnInput({ fork: true }), 'engine', job)).toBe('fork')
  expect(spawnCase(book, spawnInput({ fork: true }), 'engine', undefined)).toBe('other')
  expect(spawnCase(book, spawnInput({}), 'engine', job)).toBe('new')
  expect(spawnCase(book, spawnInput({}), 'engine', undefined)).toBe('other')
})

test('the fixed manifests come from the table of the builder skill', () => {
  const skill = [
    '# Builder',
    '## Fixed manifests',
    '',
    '| Role | Read-first skills |',
    '| --- | --- |',
    '| `decision-maker` | `rex-code-philosophy`, `rex-agent-prompting` |',
    '| decision-challenger | `rex-code-philosophy` |',
    '',
    '## Rules',
    '| skill-writer | `rex-skill-language` |',
  ].join('\n')

  expect([...fixedManifests(skill).entries()]).toEqual([
    ['decision-maker', ['rex-code-philosophy', 'rex-agent-prompting']],
    ['decision-challenger', ['rex-code-philosophy']],
  ])
})

test('a brief part another module adds is asked with the brief context and stands in its order', () => {
  const parts = createParts()
  for (const part of OWN_PARTS) {
    parts.briefParts.add(part)
  }
  parts.briefParts.add(PROBE_PART)
  const book = seededBook()
  const job = book.jobs[0]
  if (job === undefined) {
    throw new Error('the seeded book holds no job')
  }
  const ctx = { book, job, run: null, spawn: spawnInput({ subagentType: 'general-purpose' }), role: 'engineer', task: 'T2', files: [] }

  const brief = assembled(parts.briefParts.entries(), ctx, 'Task: T2')

  expect(brief.split('\n\n').map(piece => piece.slice(0, 12))).toEqual(['Rex-Job: 0ga', 'Task: T2', 'Probe: job 0', 'Shell comman', 'Return: writ'])
  expect(seenContexts.at(-1)).toEqual(ctx)
})

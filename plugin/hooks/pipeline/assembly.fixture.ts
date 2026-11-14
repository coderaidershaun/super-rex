// Test support for the spawn pipeline: a planned job, the files its briefs read, a brief written to the template, and an engine that records each prompt it starts.
import type { AgentSpawnInput, On } from 'claude-code'

import type { RexBook } from '../../types'
import { planJobIn } from '../book/jobs'
import { START_MS } from '../book/kit'
import { JOB, seededBook } from '../tools/act-kit'
import { TIER_FILE_COPIES } from './tier-files.fixture'

/** A read-first skill whose hash the shared hash vectors give. */
export const TDD_TEXT = 'Body.\n'
export const TDD_HASH = 'sha256:44261ce242e1b99d52c7d2a4cb6dbcb5a4ab507bed9b9b303062a969fafe1d1e'

export const SKILL_FILES = {
  'plugin:library/rex-code-tdd/SKILL.md': TDD_TEXT,
  'plugin:library/rex-agent-prompting/SKILL.md': '# Agent Prompting\n',
  'plugin:library/rex-agent-prompting/haiku.md': TIER_FILE_COPIES['haiku.md'] ?? '',
  'plugin:library/rex-agent-prompting/sonnet.md': TIER_FILE_COPIES['sonnet.md'] ?? '',
  'plugin:library/rex-agent-prompting/opus.md': TIER_FILE_COPIES['opus.md'] ?? '',
  'plugin:library/rex-agent-prompting/fable.md': TIER_FILE_COPIES['fable.md'] ?? '',
} as const

export const AUTHOR = [
  'Task: T2',
  'Outcome: the parser reads empty files.',
  'Scope: src/parser.rs and its tests.',
  'Context: the parser is in src/parser.rs.',
  'Acceptance: `cargo test parser` passes.',
  'Limits: never push to main.',
].join('\n')

/** Job 0gaq820001 planned with task T2 of role engineer, which runs on `gridAgent` and reads rex-code-tdd first. */
export function plannedBook(gridAgent: string): RexBook {
  const plan = {
    orchestrator: 'orchestration-builder 0.8.0',
    breakdown: 'T2 engineer',
    tasks: [{ id: 'T2', role: 'engineer', skills: ['rex-code-tdd'], openWhen: [{ skill: 'rex-code-tdd', file: 'mocking.md', when: 'a test crosses a process' }] }],
    roles: [{ role: 'engineer', ratings: { intelligence: 'high', reliability: 'high', consequence: 'high', verification: 'low' }, gridAgent, conflict: null, omsVersion: 2 }],
    manifest: [],
  } as const
  return planJobIn(seededBook(), JOB, plan, START_MS)
}

/** The engine beneath the plugin: each spawn starts an agent `a<n>` and its prompt is kept. */
export function spawnsKept(on: On): AgentSpawnInput[] {
  const started: AgentSpawnInput[] = []
  on('agent.spawn', (_$, e) => {
    started.push(e)
    return { model: 'claude-opus-5-5', agentId: `a${String(started.length)}` }
  })
  return started
}

// The book behind fixtures/shared/live/snapshot.json, for the tests of the snapshot, the outline, the pane and the band: a master tree four levels deep and a delivered job with no tree.
// `shift` moves every time by the same amount, so a kit session whose clock starts elsewhere sees the same ages.
import type { RexBook, RexJob, RexRun, RexTokens } from '../../types'
import { emptyBook } from '../book/events'

export const FIXTURE_SESSION = '53f1c255-4567-4e00-ab35-553d90ce69d1'
/** The fixture's `written_at`. */
export const WRITTEN_AT = Date.parse('2026-11-15T20:41:10.000Z')

const REPO = '/home/ana/code/shop'
const PLANS = `${REPO}/.rex/plans`

function tokens(input: number, output: number, cacheRead: number, cacheCreation: number): RexTokens {
  return { input, output, cacheRead, cacheCreation }
}

type RunFields = Pick<RexRun, 'agentId' | 'job' | 'tree' | 'parent' | 'depth' | 'role' | 'task' | 'agentType' | 'planned' | 'state' | 'endReason' | 'tokens' | 'spentNano' | 'marks' | 'lint' | 'skillsAttached' | 'skillsOpened' | 'outputFile'> & {
  readonly model: string
  readonly effort: string
  readonly started: string
  readonly ended: string | null
}

function run(fields: RunFields, at: (iso: string) => number): RexRun {
  const { model, effort, started, ended, ...rest } = fields
  return {
    ...rest,
    toolUseId: null,
    parentSource: 'engine',
    ran: { model, effort },
    startedAt: at(started),
    endedAt: ended === null ? null : at(ended),
    briefHash: null,
    assembled: 'yes',
    lintFindings: [],
    answerChars: null,
  }
}

type JobFields = Pick<RexJob, 'id' | 'tree' | 'origin' | 'status' | 'request' | 'merge' | 'spentNano' | 'tokens' | 'deliveries' | 'outcome' | 'createdAt' | 'updatedAt'> & {
  readonly builder: string
  readonly orchestrator: string
}

function job(fields: JobFields, session: string): RexJob {
  const { builder, orchestrator, ...rest } = fields
  return {
    ...rest,
    owner: { session, builder },
    restatement: null,
    repo: REPO,
    branch: `rex/${fields.id}`,
    base: 'main',
    worktree: `/home/ana/code/shop.worktrees/${fields.id}`,
    type: 'feature',
    size: 'small',
    consequence: 'low',
    langs: ['rust'],
    prefs: { duration: null, budget: null, autonomy: null },
    mergeQuestion: null,
    limit: { nano: 28_000_000_000, source: 'default' },
    plan: { orchestrator, breakdown: '', tasks: [], roles: [], manifest: [] },
    decisions: [],
    degraded: [],
    dependsOn: [],
    mergeAfter: [],
    outputFile: null,
    retries: 0,
    stuckWakes: 0,
    doneChecks: [],
  }
}

export function liveBook(session: string = FIXTURE_SESSION, shift = 0): RexBook {
  const at = (iso: string): number => Date.parse(iso) + shift
  const runs: readonly RunFields[] = [
    { agentId: 'amaster01', job: null, tree: 't0geh4lh3i', parent: 'main', depth: 1, role: null, task: null, agentType: 'super-rex:master', planned: null, model: 'claude-opus-5-5', effort: 'high', state: 'done', endReason: 'answer', started: '2026-11-15T20:33:59.000Z', ended: '2026-11-15T20:34:45.000Z', tokens: tokens(300, 2500, 60000, 21000), spentNano: 400_000_000, marks: [], lint: null, skillsAttached: [], skillsOpened: [], outputFile: null },
    { agentId: 'abuilder01', job: '0geh5fafkb', tree: null, parent: 'main', depth: 1, role: null, task: null, agentType: 'super-rex:model-high-effort-high', planned: null, model: 'claude-opus-5-5', effort: 'high', state: 'open', endReason: null, started: '2026-11-15T20:34:28.000Z', ended: null, tokens: tokens(500, 9000, 800000, 70000), spentNano: 1_400_000_000, marks: [], lint: null, skillsAttached: [], skillsOpened: ['orchestration-builder'], outputFile: `${PLANS}/0geh5fafkb/builder.md` },
    { agentId: 'adecision01', job: '0geh5fafkb', tree: null, parent: 'abuilder01', depth: 2, role: 'decision-maker', task: 'decision-maker', agentType: 'super-rex:model-high-effort-medium', planned: null, model: 'claude-opus-5-5', effort: 'medium', state: 'done', endReason: 'answer', started: '2026-11-15T20:35:02.000Z', ended: '2026-11-15T20:36:40.000Z', tokens: tokens(200, 8000, 400000, 40000), spentNano: 700_000_000, marks: [], lint: 'pass', skillsAttached: ['orchestration-builder/stages/1-skills/stage.md'], skillsOpened: [], outputFile: `${PLANS}/0geh5fafkb/decision-maker.md` },
    { agentId: 'alead01', job: '0geh5fafkb', tree: null, parent: 'abuilder01', depth: 2, role: 'engineer-lead', task: 'engineer-lead', agentType: 'super-rex:model-high-effort-medium', planned: 'model-high-effort-medium', model: 'claude-opus-5-5', effort: 'medium', state: 'open', endReason: null, started: '2026-11-15T20:37:30.000Z', ended: null, tokens: tokens(100, 6000, 500000, 30000), spentNano: 600_000_000, marks: [], lint: 'pass', skillsAttached: ['rex-code-philosophy'], skillsOpened: [], outputFile: `${PLANS}/0geh5fafkb/engineer-lead.md` },
    { agentId: 'aworker01', job: '0geh5fafkb', tree: null, parent: 'alead01', depth: 3, role: 'engineer', task: 'T1', agentType: 'super-rex:model-medium-effort-medium', planned: 'model-medium-effort-medium', model: 'claude-sonnet-5-5', effort: 'medium', state: 'open', endReason: null, started: '2026-11-15T20:38:10.000Z', ended: null, tokens: tokens(80, 4000, 300000, 15000), spentNano: 450_000_000, marks: ['lint-passed-marked'], lint: 'passed-marked', skillsAttached: ['rex-code-philosophy', 'rex-code-ergonomics'], skillsOpened: ['rex-code-tdd'], outputFile: `${PLANS}/0geh5fafkb/engineer.md` },
    { agentId: 'ahelper01', job: '0geh5fafkb', tree: null, parent: 'aworker01', depth: 4, role: null, task: null, agentType: 'super-rex:model-low-effort-low', planned: null, model: 'claude-haiku-5-5', effort: 'low', state: 'done', endReason: 'answer', started: '2026-11-15T20:39:00.000Z', ended: '2026-11-15T20:39:41.000Z', tokens: tokens(20, 1500, 100000, 5000), spentNano: 100_000_000, marks: [], lint: null, skillsAttached: [], skillsOpened: [], outputFile: null },
    { agentId: 'asolo01', job: '0gegkccde2', tree: null, parent: 'main', depth: 1, role: 'engineer', task: 'T1', agentType: 'super-rex:model-low-effort-low', planned: 'model-low-effort-low', model: 'claude-haiku-5-5', effort: 'low', state: 'done', endReason: 'answer', started: '2026-11-15T20:27:18.000Z', ended: '2026-11-15T20:28:02.000Z', tokens: tokens(60, 3000, 200000, 12000), spentNano: 200_000_000, marks: [], lint: 'pass', skillsAttached: ['rex-code-philosophy'], skillsOpened: ['rex-code-tests-unit-testing'], outputFile: `${PLANS}/0gegkccde2/engineer.md` },
  ]
  return {
    ...emptyBook(),
    session,
    versions: { mod: '0.9.0', binary: '0.9.0', claudeCode: '2.1.325' },
    storage: 'off',
    trees: [
      {
        id: 't0geh4lh3i',
        session,
        master: 'amaster01',
        charter: 'Build a hello world app and a goodbye world app.\nErgonomics and commenting are important.',
        charterHash: 'sha256:charter',
        policyHash: 'sha256:0123456789abcdef0123',
        state: 'running',
        ledgerNano: 400_000_000,
        capNano: 150_000_000_000,
        builders: [{ job: '0geh5fafkb', agentId: 'abuilder01' }],
        ticks: [],
        notes: [{ at: at('2026-11-15T20:34:17.000Z'), job: null, text: 'Step list: one step. Both apps touch the same files, so one builder does both.' }],
        startedAt: at('2026-11-15T20:33:59.000Z'),
        closedAt: null,
        reportFile: null,
      },
    ],
    jobs: [
      job(
        {
          id: '0geh5fafkb',
          tree: 't0geh4lh3i',
          builder: 'abuilder01',
          origin: 'master',
          status: 'planned',
          request: 'Build two small command-line apps, hello and goodbye, in the Cargo package at the repository root.',
          orchestrator: 'lead-and-workers',
          merge: { answer: 'yes', source: 'policy', question: null, policyHash: 'sha256:0123456789abcdef0123', at: at('2026-11-15T20:34:27.000Z') },
          spentNano: 3_250_000_000,
          tokens: tokens(1200, 30500, 2100000, 160000),
          deliveries: [],
          outcome: null,
          createdAt: at('2026-11-15T20:34:27.000Z'),
          updatedAt: at('2026-11-15T20:41:05.000Z'),
        },
        session,
      ),
      job(
        {
          id: '0gegkccde2',
          tree: null,
          builder: 'main',
          origin: 'composer',
          status: 'delivered',
          request: 'please build a hello world app',
          orchestrator: 'single-engineer',
          merge: { answer: 'yes', source: 'dialog', question: null, policyHash: null, at: at('2026-11-15T20:21:48.000Z') },
          spentNano: 1_800_000_000,
          tokens: tokens(900, 14000, 900000, 80000),
          deliveries: [{ repo: REPO, branch: 'rex/0gegkccde2', pr: null, outcome: 'merged', mergeCommit: 'bb03a46f0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f', note: null }],
          outcome: 'Hello world binary built, verified and merged to main.',
          createdAt: at('2026-11-15T20:21:48.000Z'),
          updatedAt: at('2026-11-15T20:28:23.000Z'),
        },
        session,
      ),
    ],
    runs: runs.map(fields => run(fields, at)),
    questions: [
      { id: 'q1-553d90ce69d1', tree: 't0geh4lh3i', job: '0geh5fafkb', question: 'Merge job 0geh5fafkb (rex/0geh5fafkb) to main of /home/ana/code/shop?', options: ['Merge', 'Keep the branch'], recommendation: 'Merge', category: 'merge', mergeFor: '0geh5fafkb', askedAt: at('2026-11-15T20:40:50.000Z'), answer: null },
      {
        id: 'q2-553d90ce69d1',
        tree: 't0geh4lh3i',
        job: null,
        question: 'The charter names no test command. Which one proves each step?',
        options: ['cargo test', 'cargo test and cargo clippy'],
        recommendation: 'cargo test and cargo clippy',
        category: 'scope',
        mergeFor: null,
        askedAt: at('2026-11-15T20:34:30.000Z'),
        answer: { option: 'cargo test and cargo clippy', source: 'command', at: at('2026-11-15T20:35:00.000Z') },
      },
    ],
    ingest: { stream: 'aaaaaaaaaaaaaaaa', secret: '9f'.repeat(32), nextSequence: 1, acked: 0, failures: 0, resets: 0, halted: null, dropped: 0, queue: [] },
  }
}

// Proves the assignment rules as pure functions: the table as data gives the skills of 0.7's table, a compound row fits on any one criterion, a rule-required skill stays whatever the answers, the cap and placement, and an overlay that changes one row.
import { expect, test } from 'claude-code/testing'

import { ASSIGN_07 } from './assign-07.fixture'
import { assignTasks } from './rules'
import type { Answer, AnswerOf, Placed, TaskFacts } from './rules'
import { applyOverlay, parseTable } from './table'
import type { AssignTable } from './table'
import { TABLE_TEXT } from './table.fixture'

function table(): AssignTable {
  const read = parseTable(TABLE_TEXT, 'sha256:table')
  if (!read.ok) {
    throw new Error(read.problem)
  }
  return read.value
}

/** The tier files 0.7 gave each lead as read-first; the mod now adds a grid agent's tier text to its brief at the spawn, so the table leaves them out. */
const TIER_FILES = ['rex-agent-prompting/fable.md', 'rex-agent-prompting/haiku.md', 'rex-agent-prompting/opus.md', 'rex-agent-prompting/sonnet.md']

/** Each skill or sibling file one 0.7 cell names in backticks. */
function namesIn(cell: string): readonly string[] {
  return [...cell.matchAll(/`(rex-[a-z0-9<>-]+(?:\/[A-Za-z0-9._/-]+\.md)?)`/g)].map(match => (match[1] ?? '').replace('<language>', '{language}'))
}

/** 0.7's rows by their kind of work: the code rulebooks, the gap skill line, and each row of `## By kind of work`. */
function rows07(): ReadonlyMap<string, readonly string[]> {
  const rows = new Map<string, readonly string[]>()
  const rulebooks = ASSIGN_07.split('## Each task that writes code')[1]?.split('## By kind of work')[0] ?? ''
  rows.set('Each task that writes code', rulebooks.split('\n').filter(line => line.startsWith('| `')).flatMap(namesIn))
  const gap = rulebooks.split('\n').find(line => line.includes('gap skill')) ?? ''
  rows.set('Gap skill of each language that is not rust', namesIn(gap))
  const byKind = ASSIGN_07.split('## By kind of work')[1]?.split('## Not in table')[0] ?? ''
  for (const line of byKind.split('\n').filter(each => each.startsWith('| ') && !each.startsWith('| Kind') && !each.startsWith('| ---'))) {
    const [work = '', skills = ''] = line.split(' | ').map(cell => cell.replace(/^\| /, '').replace(/ \|$/, ''))
    rows.set(work, namesIn(skills))
  }
  return rows
}

test('each row of the 0.7 assignment table gives the same skills as table.json', () => {
  const rows = table().rows
  const old = rows07()

  expect(old.size).toBe(22)
  for (const [work, skills] of old) {
    const now = rows.filter(row => row.work === work).flatMap(row => [...row.skills, ...row.files.map(file => `${file.skill}/${file.file}`)])
    const kept = skills.filter(skill => !TIER_FILES.includes(skill))
    expect([work, [...new Set(now)].sort()]).toEqual([work, [...new Set(kept)].sort()])
  }
  expect(rows.filter(row => !old.has(row.work)).map(row => row.id)).toEqual([])
})

const CODE_TASK: TaskFacts = { id: 'T2', role: 'engineer', writesCode: true, langs: ['rust'] }
const NO_ANSWERS: AnswerOf = () => undefined

function answersBy(pick: (task: string, row: string, criterion: string) => Answer | undefined): AnswerOf {
  return key => pick(key.task, key.row, key.criterion)
}

const yes = (reason: string): Answer => ({ answer: 'yes', reason })
const no = (reason: string): Answer => ({ answer: 'no', reason })

function placedOf(result: ReturnType<typeof assignTasks>, task: string): readonly Placed[] {
  if (result.kind !== 'placed') {
    throw new Error(`questions remain: ${result.questions.map(question => `${question.row}/${question.criterion}`).join(', ')}`)
  }
  return result.tasks.find(each => each.id === task)?.placements ?? []
}

function readFirst(placements: readonly Placed[]): readonly string[] {
  return placements.filter(one => one.place === 'read-first').map(one => one.skill)
}

function openWhen(placements: readonly Placed[]): readonly string[] {
  return placements.filter(one => one.place === 'open-when').map(one => (one.file === null ? one.skill : `${one.skill}/${one.file}`))
}

test('a first call asks only what no rule decides, and a task that writes no code is asked no code row', () => {
  const docs: TaskFacts = { id: 'T9', role: 'author', writesCode: false, langs: [] }
  const result = assignTasks(table(), [CODE_TASK, docs], NO_ANSWERS)

  expect(result.kind).toBe('questions')
  const questions = result.kind === 'questions' ? result.questions : []
  const asked = (task: string) => questions.filter(question => question.task === task).map(question => `${question.row}/${question.criterion}`)
  expect(asked('T2')).not.toContain('code-rulebooks/writes-code')
  expect(asked('T2')).toContain('tdd/builds-feature')
  expect(asked('T9')).toEqual(['design/design-first', 'spike/unproven', 'priorities/ranked-decision', 'pr-review/reviews-others', 'diagrams/draws-diagram', 'skill-text/writes-instructions', 'briefs/starts-agent', 'briefs/writes-brief', 'plan-writing/writes-code-plan'])
  expect(questions.find(question => question.task === 'T2' && question.criterion === 'retry-or-reconnect')?.text).toBe('Is it true that task T2 retries or reconnects? Row failure-policy: Outside input, long-running state, retry, reconnect, main loop')
})

test('a compound row fits when any one of its criteria holds, and its case is the criterion that fit', () => {
  const only = (criterion: string) =>
    answersBy((_task, row, asked) => {
      if (row === 'failure-policy' && asked === 'each-step') return no('one module only')
      return row === 'failure-policy' && asked === criterion ? yes('it does') : no('not this task')
    })

  for (const criterion of ['outside-input', 'long-running-state', 'retry-or-reconnect', 'main-loop']) {
    const placements = placedOf(assignTasks(table(), [CODE_TASK], only(criterion)), 'T2')
    const policy = placements.find(one => one.skill === 'rex-code-failure-policy' && one.file === null)
    const text = table().rows.find(row => row.id === 'failure-policy')?.criteria.find(each => each.id === criterion)?.text
    expect([criterion, policy?.place, policy?.when, policy?.source]).toEqual([criterion, 'open-when', text, 'decision-agent'])
  }
  const none = placedOf(assignTasks(table(), [CODE_TASK], answersBy(() => no('no'))), 'T2')
  expect(none.some(one => one.skill === 'rex-code-failure-policy')).toBe(false)
})

test('a rule-required skill stays under any answers', () => {
  const ts: TaskFacts = { id: 'T5', role: 'engineer', writesCode: true, langs: ['ts'] }
  const required = ['rex-code-philosophy', 'rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts']
  const answerSets: readonly AnswerOf[] = [
    answersBy(() => no('no')),
    answersBy(() => yes('yes')),
    answersBy((_task, _row, criterion) => (criterion === 'each-step' ? yes('each step') : yes('yes'))),
    answersBy((_task, row, criterion) => ((row.length + criterion.length) % 2 === 0 ? yes('even') : no('odd'))),
  ]
  for (const answers of answerSets) {
    const placements = placedOf(assignTasks(table(), [ts], answers), 'T5')
    expect(required.map(skill => [skill, placements.find(one => one.skill === skill && one.file === null)?.place, placements.find(one => one.skill === skill)?.source])).toEqual(
      required.map(skill => [skill, 'read-first', 'rule']),
    )
  }
})

test('the cap keeps six read-first skills by priority and moves the rest to open-when, and sibling files are always open-when', () => {
  const ts: TaskFacts = { id: 'T5', role: 'engineer', writesCode: true, langs: ['ts'] }
  const eager = answersBy((_task, row, criterion) => (row === 'latency' ? no('no hot path') : criterion === 'each-step' ? yes('each step') : yes('yes')))
  const placements = placedOf(assignTasks(table(), [ts], eager), 'T5')

  expect(readFirst(placements)).toEqual(['rex-code-philosophy', 'rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts', 'rex-code-tdd'])
  const design = placements.find(one => one.skill === 'rex-code-design-sketch')
  expect([design?.place, design?.when, design?.reason]).toEqual(['open-when', 'designs a new system, feature or module before its code', 'design/design-first: yes; each step: each step; over the cap of 6 read-first skills'])
  expect(openWhen(placements)).toContain('rex-code-tdd/mocking.md')
  expect(openWhen(placements)).toContain('rex-code-folder-management/move.md')
  expect(placements.find(one => one.file === 'move.md')?.when).toBe('it moves a file, module or folder')
  expect(placements.find(one => one.file === 'mocking.md')?.when).toBe('writes a test that crosses to an outside API or database')

  const fewer = answersBy((_task, row, criterion) => (row === 'folders' && criterion === 'each-step' ? no('only when it adds a module') : row === 'folders' && criterion === 'creates-or-splits' ? yes('adds one module') : no('no')))
  const folders = placedOf(assignTasks(table(), [ts], fewer), 'T5').find(one => one.skill === 'rex-code-folder-management' && one.file === null)
  expect([folders?.place, folders?.when, folders?.reason]).toEqual(['open-when', 'creates or splits a file, module or folder', 'folders/creates-or-splits: adds one module; one case: only when it adds a module'])
})

test('a placement question comes only after its row fits', () => {
  const first = assignTasks(table(), [CODE_TASK], answersBy((_task, row, criterion) => (criterion === 'each-step' ? undefined : row === 'spike' ? yes('new crate') : no('no'))))

  expect(first.kind === 'questions' ? first.questions.map(question => `${question.row}/${question.criterion}`) : []).toEqual(['spike/each-step'])
})

test('an overlay changes one row and leaves the others as they are', () => {
  const overlay = JSON.stringify({ format: 1, rows: [{ ...JSON.parse(JSON.stringify(table().rows.find(row => row.id === 'diagrams'))), skills: ['rex-house-diagrams'] }] })
  const read = applyOverlay(table(), overlay, 'sha256:overlay')

  expect(read.ok).toBe(true)
  const changed = read.ok ? read.value : table()
  expect(changed.overlayVersion).toBe('sha256:overlay')
  expect(changed.rows.map(row => row.id)).toEqual(table().rows.map(row => row.id))
  expect(changed.rows.filter((row, index) => JSON.stringify(row) !== JSON.stringify(table().rows[index])).map(row => [row.id, row.skills])).toEqual([['diagrams', ['rex-house-diagrams']]])

  const answers = answersBy((_task, row, criterion) => (row === 'diagrams' ? (criterion === 'each-step' ? yes('each step') : yes('draws one')) : no('no')))
  expect(readFirst(placedOf(assignTasks(changed, [CODE_TASK], answers), 'T2'))).toContain('rex-house-diagrams')
  expect(applyOverlay(table(), JSON.stringify({ format: 1, remove: ['nope'] }), 'v')).toEqual({ ok: false, problem: 'overlay.remove names no row of the table, got "nope"' })
  expect(applyOverlay(table(), '{"format": 1, "rows": [{"id": "x"}]}', 'v')).toEqual({ ok: false, problem: 'overlay.rows[0].work must be text, got nothing' })
})

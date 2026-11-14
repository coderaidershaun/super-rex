// Proves the assignment function against a real decision: answered as the decision agent of this mod's own job judged each task, it gives that block's Skills and Open when, except the differences listed here.
// Each listed difference is a rule of the table as data, not a judgement: a rule-required skill, a whole row, a sibling file in Open when, the cap, or a file 0.7's table never named.
import { expect, test } from 'claude-code/testing'

import { BLOCK_TASKS } from './block.fixture'
import type { BlockTask } from './block.fixture'
import { assignTasks } from './rules'
import type { AnswerOf, Placed } from './rules'
import { EACH_STEP, parseTable } from './table'
import type { AssignTable, Row } from './table'
import { TABLE_TEXT } from './table.fixture'

function table(): AssignTable {
  const read = parseTable(TABLE_TEXT, 'sha256:table')
  if (!read.ok) {
    throw new Error(read.problem)
  }
  return read.value
}

function itemsOf(row: Row, task: BlockTask): readonly string[] {
  const skills = row.skills.flatMap(skill => (skill.includes('{language}') ? task.langs.filter(lang => lang !== 'rust').map(lang => skill.replace('{language}', lang)) : [skill]))
  return [...skills, ...row.files.map(file => `${file.skill}/${file.file}`)]
}

/**
 * The answers the block's own Skills and Open when imply: a row fits on its
 * first criterion when it gives the task an item no earlier row gave, and it
 * is needed in each step when one of its skills is in Skills.
 */
function blockAnswers(rows: readonly Row[]): AnswerOf {
  return key => {
    const task = BLOCK_TASKS.find(each => each.id === key.task)
    const rowIndex = rows.findIndex(each => each.id === key.row)
    const row = rows[rowIndex]
    if (task === undefined || row === undefined) {
      return undefined
    }
    const given = new Set([...task.skills, ...task.openWhen])
    const earlier = new Set(rows.slice(0, rowIndex).flatMap(each => itemsOf(each, task)))
    if (key.criterion === EACH_STEP) {
      const isEachStep = row.skills.some(skill => task.skills.includes(skill))
      return { answer: isEachStep ? 'yes' : 'no', reason: isEachStep ? 'in Skills of the block' : 'in Open when of the block' }
    }
    const fits = key.criterion === row.criteria[0]?.id && itemsOf(row, task).some(item => given.has(item) && !earlier.has(item))
    return { answer: fits ? 'yes' : 'no', reason: fits ? 'the block gives its skills' : 'the block gives none of its skills' }
  }
}

function nameOf(placed: Placed): string {
  return placed.file === null ? placed.skill : `${placed.skill}/${placed.file}`
}

type Difference = readonly [task: string, item: string, block: 'skills' | 'open when' | 'none', function_: 'skills' | 'open when' | 'none']

function differences(): readonly Difference[] {
  const rows = table().rows
  const facts = BLOCK_TASKS.map(task => ({ id: task.id, role: task.role, writesCode: task.writesCode, langs: task.langs }))
  const result = assignTasks(table(), facts, blockAnswers(rows))
  if (result.kind !== 'placed') {
    throw new Error(`questions remain: ${result.questions.map(question => `${question.task} ${question.row}/${question.criterion}`).join(', ')}`)
  }
  const found: Difference[] = []
  for (const task of BLOCK_TASKS) {
    const placements = result.tasks.find(each => each.id === task.id)?.placements ?? []
    const placeIn = (item: string): Difference[2] => (task.skills.includes(item) ? 'skills' : task.openWhen.includes(item) ? 'open when' : 'none')
    const placeOut = (item: string): Difference[3] => {
      const placed = placements.find(one => nameOf(one) === item)
      return placed === undefined ? 'none' : placed.place === 'read-first' ? 'skills' : 'open when'
    }
    const items = [...new Set([...task.skills, ...task.openWhen, ...placements.map(nameOf)])]
    found.push(...items.filter(item => placeIn(item) !== placeOut(item)).map((item): Difference => [task.id, item, placeIn(item), placeOut(item)]))
  }
  return found
}

const RULEBOOKS = ['rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts']

/** The four code rulebooks and the gap skill are read first by rule for every code task; the block kept some in Open when for T1 (spike code) and T8. */
const RULE_REQUIRED: readonly Difference[] = ['T1', 'T8'].flatMap(task => RULEBOOKS.map((skill): Difference => [task, skill, 'open when', 'skills']))

/** With five read-first skills by rule, the cap of six moves the block's other read-first skills of T1 and T8 to Open when. */
const OVER_THE_CAP: readonly Difference[] = [
  ['T1', 'rex-code-spike', 'skills', 'open when'],
  ['T1', 'rex-code-validation-plan', 'skills', 'open when'],
  ['T1', 'rex-code-failure-policy', 'skills', 'open when'],
  ['T8', 'rex-code-validation-plan', 'skills', 'open when'],
  ['T8', 'rex-code-tests-integration-testing', 'skills', 'open when'],
]

/** A row gives all its skills: T8 has rex-code-validation-plan, so the design row also gives rex-code-design-sketch. */
const WHOLE_ROW: readonly Difference[] = [['T8', 'rex-code-design-sketch', 'none', 'skills']]

/** 0.7's own rule: a sibling file after a case word is in Open when with that case when the task is not that case; the block left these out. */
const SIBLING_CASES: readonly Difference[] = [
  ...['T2', 'T4', 'T5', 'T6', 'T7', 'T10'].map((task): Difference => [task, 'rex-code-failure-policy/failure-table.md', 'none', 'open when']),
  ...['T1', 'T4', 'T5', 'T6', 'T7', 'T10'].map((task): Difference => [task, 'rex-code-folder-management/move.md', 'none', 'open when']),
  ...['T2', 'T10'].flatMap((task): Difference[] => [
    [task, 'rex-code-folder-management/audit.md', 'none', 'open when'],
    [task, 'rex-code-failure-policy/audit.md', 'none', 'open when'],
  ]),
  ...['T2', 'T5', 'T7', 'T8'].map((task): Difference => [task, 'rex-code-tests-integration-testing/real-funds.md', 'none', 'open when']),
]

/** Files 0.7's table never named, which the block added by hand; in 0.8 they are overrides of rex_job_plan with their reason. */
const NOT_IN_TABLE: readonly Difference[] = [
  ['T1', 'rex-code-priorities/write.md', 'open when', 'none'],
  ['T1', 'rex-gap-ts/setup.md', 'open when', 'none'],
  ['T4', 'rex-agent-prompting/sonnet.md', 'open when', 'none'],
]

function sorted(list: readonly Difference[]): readonly Difference[] {
  return [...list].sort((a, b) => a.join(' ').localeCompare(b.join(' ')))
}

test('the tasks of this job decision block get the same Skills and Open when from the function, except the listed differences', () => {
  expect(sorted(differences())).toEqual(sorted([...RULE_REQUIRED, ...OVER_THE_CAP, ...WHOLE_ROW, ...SIBLING_CASES, ...NOT_IN_TABLE]))
})

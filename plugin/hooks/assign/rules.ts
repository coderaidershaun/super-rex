// Applies the assignment table to a plan's tasks: rules decide what code can, the decision agent's answers decide the rest, and placement and the cap follow from both.
// Pure, so the same table, facts and answers always give the same skills.
import { EACH_STEP, LANGUAGE } from './table'
import type { AssignTable, Criterion, CriterionRule, Row } from './table'

/** What the caller says about a task; nothing here is judged. */
export type TaskFacts = {
  readonly id: string
  readonly role: string
  readonly writesCode: boolean
  readonly langs: readonly string[]
}

export type Answer = { readonly answer: 'yes' | 'no'; readonly reason: string }

/** One question of the decision agent: does `task` meet `criterion` of `row`; criterion `each-step` asks where a fitting row goes. */
export type QuestionKey = { readonly task: string; readonly row: string; readonly criterion: string }

export type Question = QuestionKey & { readonly text: string }

export type AnswerOf = (key: QuestionKey) => Answer | undefined

/** One skill, or one sibling file of a skill, placed on a task with where it came from. */
export type Placed = {
  readonly skill: string
  readonly file: string | null
  readonly place: 'read-first' | 'open-when'
  readonly when: string | null
  readonly source: 'rule' | 'decision-agent'
  readonly reason: string
  readonly row: string
}

export type Assignment =
  | { readonly kind: 'questions'; readonly questions: readonly Question[] }
  | { readonly kind: 'placed'; readonly tasks: readonly { readonly id: string; readonly placements: readonly Placed[] }[] }

type Fit = { readonly criterion: Criterion; readonly source: Placed['source']; readonly reason: string }

type Ranked = Placed & { readonly priority: number; readonly order: number; readonly fitCase: string }

function otherLanguages(task: TaskFacts): readonly string[] {
  return task.langs.filter(lang => lang !== 'rust')
}

function ruleHolds(rule: CriterionRule, task: TaskFacts): boolean {
  switch (rule.kind) {
    case 'writes-code':
      return task.writesCode
    case 'language-not-rust':
      return task.writesCode && otherLanguages(task).length > 0
    case 'role':
      return task.role === rule.role
    default:
      return assertNever(rule)
  }
}

function assertNever(value: never): never {
  throw new Error(`unhandled rule: ${JSON.stringify(value)}`)
}

function fitText(task: TaskFacts, row: Row, criterion: Criterion): string {
  return `Is it true that task ${task.id} ${criterion.text}? Row ${row.id}: ${row.work}`
}

function eachStepText(task: TaskFacts, row: Row, fit: Criterion): string {
  return `Does task ${task.id} need row ${row.id} in each of its steps, not only in one case? It fits because it ${fit.text}. Row ${row.id}: ${row.work}`
}

/** The criterion that makes `row` fit `task`, the questions that must be answered first, or neither when the row does not fit. */
function fitOf(task: TaskFacts, row: Row, answerOf: AnswerOf): { readonly fit: Fit | null; readonly questions: readonly Question[] } {
  const questions: Question[] = []
  for (const criterion of row.criteria) {
    if (criterion.rule !== null) {
      if (ruleHolds(criterion.rule, task)) {
        return { fit: { criterion, source: 'rule', reason: criterion.text }, questions: [] }
      }
      continue
    }
    const key = { task: task.id, row: row.id, criterion: criterion.id }
    const answer = answerOf(key)
    if (answer === undefined) {
      questions.push({ ...key, text: fitText(task, row, criterion) })
    } else if (answer.answer === 'yes') {
      return { fit: { criterion, source: 'decision-agent', reason: answer.reason }, questions: [] }
    }
  }
  return { fit: null, questions }
}

function skillsOf(row: Row, task: TaskFacts): readonly string[] {
  return row.skills.flatMap(skill => (skill.includes(LANGUAGE) ? otherLanguages(task).map(lang => skill.replace(LANGUAGE, lang)) : [skill]))
}

/** Every skill and file of a fitting row, before duplicates and the cap. */
function placementsOf(row: Row, rowIndex: number, task: TaskFacts, fit: Fit, place: Placed['place'], why: string): readonly Ranked[] {
  const reason = `${row.id}/${fit.criterion.id}: ${fit.reason}${why}`
  const base = { source: fit.source, reason, row: row.id, priority: row.priority, fitCase: fit.criterion.text }
  const skills = skillsOf(row, task).map((skill, index): Ranked => ({ ...base, skill, file: null, place, when: place === 'open-when' ? fit.criterion.text : null, order: rowIndex * 100 + index }))
  const files = row.files.map((file, index): Ranked => ({ ...base, skill: file.skill, file: file.file, place: 'open-when', when: file.case ?? fit.criterion.text, order: rowIndex * 100 + 50 + index }))
  return [...skills, ...files]
}

function byRank(a: Ranked, b: Ranked): number {
  return a.priority - b.priority || a.order - b.order
}

/** One item for each skill or file: read-first wins over open-when, then the higher priority; read-first items past the cap move to open-when. */
function settle(ranked: readonly Ranked[], cap: number): readonly Placed[] {
  const sorted = [...ranked].sort((a, b) => (a.place === b.place ? byRank(a, b) : a.place === 'read-first' ? -1 : 1))
  const unique = sorted.filter((one, index) => sorted.findIndex(other => other.skill === one.skill && other.file === one.file) === index)
  const first = unique.filter(one => one.place === 'read-first').sort(byRank)
  const over = first.slice(cap).map(one => ({ ...one, place: 'open-when' as const, when: one.fitCase, reason: `${one.reason}; over the cap of ${String(cap)} read-first skills` }))
  const opens = [...unique.filter(one => one.place === 'open-when'), ...over].sort(byRank)
  return [...first.slice(0, cap), ...opens].map(({ priority: _priority, order: _order, fitCase: _fitCase, ...placed }) => placed)
}

export function assignTasks(table: AssignTable, tasks: readonly TaskFacts[], answerOf: AnswerOf): Assignment {
  const questions: Question[] = []
  const placed: { readonly id: string; readonly placements: readonly Placed[] }[] = []
  for (const task of tasks) {
    const ranked: Ranked[] = []
    for (const [rowIndex, row] of table.rows.entries()) {
      if (row.scope === 'code' && !task.writesCode) {
        continue
      }
      const { fit, questions: asked } = fitOf(task, row, answerOf)
      questions.push(...asked)
      if (fit === null) {
        continue
      }
      if (row.placement !== 'ask') {
        ranked.push(...placementsOf(row, rowIndex, task, fit, row.placement, ''))
        continue
      }
      const key = { task: task.id, row: row.id, criterion: EACH_STEP }
      const eachStep = answerOf(key)
      if (eachStep === undefined) {
        questions.push({ ...key, text: eachStepText(task, row, fit.criterion) })
        continue
      }
      const place = eachStep.answer === 'yes' ? 'read-first' : 'open-when'
      ranked.push(...placementsOf(row, rowIndex, task, fit, place, `; ${place === 'read-first' ? 'each step' : 'one case'}: ${eachStep.reason}`))
    }
    placed.push({ id: task.id, placements: settle(ranked, table.cap) })
  }
  return questions.length > 0 ? { kind: 'questions', questions } : { kind: 'placed', tasks: placed }
}

// The assignment table as data: its rows, their atomic criteria and placement, read from the plugin's table.json and changed by a repo's .rex/assign.json.
// Both files come from outside the code, so each is checked here and a bad one is a refusal with the problem seen, never a guess.

/** A criterion that code decides from the facts the caller gave about a task, so nobody is asked. */
export type CriterionRule = { readonly kind: 'writes-code' } | { readonly kind: 'language-not-rust' } | { readonly kind: 'role'; readonly role: string }

export type Criterion = {
  readonly id: string
  readonly text: string
  readonly rule: CriterionRule | null
}

/** A file inside a skill other than its SKILL.md; it is always an open-when item, with its own case or the criterion that fit. */
export type SiblingFile = {
  readonly skill: string
  readonly file: string
  readonly case: string | null
}

export type Placement = 'read-first' | 'open-when' | 'ask'

export type Row = {
  readonly id: string
  readonly work: string
  readonly priority: number
  readonly scope: 'code' | 'any'
  readonly placement: Placement
  readonly criteria: readonly Criterion[]
  readonly skills: readonly string[]
  readonly files: readonly SiblingFile[]
}

/** The table in force for one job: the plugin's rows with the repo's overlay applied, and the hash of each file. */
export type AssignTable = {
  readonly cap: number
  readonly rows: readonly Row[]
  readonly tableVersion: string
  readonly overlayVersion: string | null
}

export type Read<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly problem: string }

/** The criterion id of the second question of a row whose placement is `ask`: does the task need the row in each step. */
export const EACH_STEP = 'each-step'

/** The text that stands for each language in a skill name, as in `rex-gap-{language}`. */
export const LANGUAGE = '{language}'

const ID = /^[a-z0-9][a-z0-9-]*$/
const SKILL = /^[a-z0-9][a-z0-9{}-]*$/
const FILE = /^[A-Za-z0-9._-]+(?:\/[A-Za-z0-9._-]+)*\.md$/

type Fields = { readonly [key: string]: unknown }

function fieldsOf(value: unknown): Fields | null {
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? Object.fromEntries(Object.entries(value)) : null
}

function seen(value: unknown): string {
  return JSON.stringify(value) ?? 'nothing'
}

function fail<T>(problem: string): Read<T> {
  return { ok: false, problem }
}

function textOf(fields: Fields, key: string, at: string): Read<string> {
  const value = fields[key]
  return typeof value === 'string' && value.trim() !== '' ? { ok: true, value } : fail(`${at}.${key} must be text, got ${seen(value)}`)
}

function listOf(fields: Fields, key: string, at: string): Read<readonly unknown[]> {
  const value = fields[key]
  return Array.isArray(value) ? { ok: true, value } : fail(`${at}.${key} must be a list, got ${seen(value)}`)
}

function ruleOf(value: unknown, at: string): Read<CriterionRule | null> {
  if (value === undefined || value === null) {
    return { ok: true, value: null }
  }
  if (value === 'writes-code' || value === 'language-not-rust') {
    return { ok: true, value: { kind: value } }
  }
  if (typeof value === 'string' && value.startsWith('role:') && ID.test(value.slice('role:'.length))) {
    return { ok: true, value: { kind: 'role', role: value.slice('role:'.length) } }
  }
  return fail(`${at}.rule must be writes-code, language-not-rust or role:<name>, got ${seen(value)}`)
}

function criterionOf(value: unknown, at: string): Read<Criterion> {
  const fields = fieldsOf(value)
  if (fields === null) {
    return fail(`${at} must be an object, got ${seen(value)}`)
  }
  const id = textOf(fields, 'id', at)
  const text = textOf(fields, 'text', at)
  const rule = ruleOf(fields['rule'], at)
  if (!id.ok) return id
  if (!ID.test(id.value) || id.value === EACH_STEP) {
    return fail(`${at}.id must be lower-case words with hyphens and not ${EACH_STEP}, got ${seen(id.value)}`)
  }
  if (!text.ok) return text
  if (!rule.ok) return rule
  return { ok: true, value: { id: id.value, text: text.value, rule: rule.value } }
}

function fileOf(value: unknown, at: string): Read<SiblingFile> {
  const fields = fieldsOf(value)
  if (fields === null) {
    return fail(`${at} must be an object, got ${seen(value)}`)
  }
  const skill = textOf(fields, 'skill', at)
  const file = textOf(fields, 'file', at)
  const named = fields['case']
  if (!skill.ok) return skill
  if (!file.ok) return file
  if (!SKILL.test(skill.value) || !FILE.test(file.value) || file.value === 'SKILL.md') {
    return fail(`${at} must name a skill and a .md file inside it other than SKILL.md, got ${seen(value)}`)
  }
  if (named !== undefined && named !== null && (typeof named !== 'string' || named.trim() === '')) {
    return fail(`${at}.case must be text or null, got ${seen(named)}`)
  }
  return { ok: true, value: { skill: skill.value, file: file.value, case: typeof named === 'string' ? named : null } }
}

function each<T>(items: readonly unknown[], at: string, read: (item: unknown, at: string) => Read<T>): Read<readonly T[]> {
  const values: T[] = []
  for (const [index, item] of items.entries()) {
    const one = read(item, `${at}[${String(index)}]`)
    if (!one.ok) return one
    values.push(one.value)
  }
  return { ok: true, value: values }
}

function rowOf(value: unknown, at: string): Read<Row> {
  const fields = fieldsOf(value)
  if (fields === null) {
    return fail(`${at} must be an object, got ${seen(value)}`)
  }
  const id = textOf(fields, 'id', at)
  const work = textOf(fields, 'work', at)
  const { priority, scope, placement } = fields
  const criteria = listOf(fields, 'criteria', at)
  const skills = listOf(fields, 'skills', at)
  const files = listOf(fields, 'files', at)
  if (!id.ok) return id
  if (!ID.test(id.value)) return fail(`${at}.id must be lower-case words with hyphens, got ${seen(id.value)}`)
  if (!work.ok) return work
  if (typeof priority !== 'number' || !Number.isSafeInteger(priority) || priority < 0) return fail(`${at}.priority must be a whole number from 0, got ${seen(priority)}`)
  if (scope !== 'code' && scope !== 'any') return fail(`${at}.scope must be code or any, got ${seen(scope)}`)
  if (placement !== 'read-first' && placement !== 'open-when' && placement !== 'ask') return fail(`${at}.placement must be read-first, open-when or ask, got ${seen(placement)}`)
  if (!criteria.ok) return criteria
  if (!skills.ok) return skills
  if (!files.ok) return files
  const readCriteria = each(criteria.value, `${at}.criteria`, criterionOf)
  if (!readCriteria.ok) return readCriteria
  if (readCriteria.value.length === 0) return fail(`${at}.criteria must hold one criterion at least, got none`)
  const ids = readCriteria.value.map(criterion => criterion.id)
  const twice = ids.find((one, index) => ids.indexOf(one) !== index)
  if (twice !== undefined) return fail(`${at}.criteria holds ${twice} twice`)
  const badSkill = skills.value.find(skill => typeof skill !== 'string' || !SKILL.test(skill))
  if (badSkill !== undefined) return fail(`${at}.skills must be skill names, got ${seen(badSkill)}`)
  const readFiles = each(files.value, `${at}.files`, fileOf)
  if (!readFiles.ok) return readFiles
  if (skills.value.length === 0 && readFiles.value.length === 0) return fail(`${at} gives no skill and no file`)
  return {
    ok: true,
    value: { id: id.value, work: work.value, priority, scope, placement, criteria: readCriteria.value, skills: skills.value.map(String), files: readFiles.value },
  }
}

function rowsOf(fields: Fields, at: string): Read<readonly Row[]> {
  const rows = listOf(fields, 'rows', at)
  if (!rows.ok) return rows
  const read = each(rows.value, `${at}.rows`, rowOf)
  if (!read.ok) return read
  const ids = read.value.map(row => row.id)
  const twice = ids.find((one, index) => ids.indexOf(one) !== index)
  return twice === undefined ? read : fail(`${at}.rows holds row ${twice} twice`)
}

function jsonOf(text: string, at: string): Read<Fields> {
  let parsed: unknown
  try {
    parsed = JSON.parse(text)
  } catch (error) {
    if (error instanceof SyntaxError) {
      return fail(`${at} is not JSON: ${error.message}`)
    }
    throw error
  }
  const fields = fieldsOf(parsed)
  if (fields === null) return fail(`${at} must be a JSON object, got ${seen(parsed)}`)
  return fields['format'] === 1 ? { ok: true, value: fields } : fail(`${at}.format must be 1, got ${seen(fields['format'])}`)
}

/** The plugin's table, before any overlay. */
export function parseTable(text: string, version: string): Read<AssignTable> {
  const fields = jsonOf(text, 'table')
  if (!fields.ok) return fields
  const { cap } = fields.value
  if (typeof cap !== 'number' || !Number.isSafeInteger(cap) || cap < 1) return fail(`table.cap must be a whole number from 1, got ${seen(cap)}`)
  const rows = rowsOf(fields.value, 'table')
  if (!rows.ok) return rows
  return { ok: true, value: { cap, rows: rows.value, tableVersion: version, overlayVersion: null } }
}

/**
 * The table with a repo's overlay applied: a row of an id the table has
 * replaces that row where it stands, a new id is added at the end, and each
 * id under `remove` is dropped.
 */
export function applyOverlay(table: AssignTable, text: string, version: string): Read<AssignTable> {
  const fields = jsonOf(text, 'overlay')
  if (!fields.ok) return fields
  const rows = fields.value['rows'] === undefined ? ({ ok: true, value: [] } as const) : rowsOf(fields.value, 'overlay')
  const remove = fields.value['remove'] ?? []
  if (!rows.ok) return rows
  if (!Array.isArray(remove) || !remove.every((id: unknown) => typeof id === 'string')) return fail(`overlay.remove must be a list of row ids, got ${seen(remove)}`)
  const unknown = remove.find((id: unknown) => !table.rows.some(row => row.id === id))
  if (unknown !== undefined) return fail(`overlay.remove names no row of the table, got ${seen(unknown)}`)
  const byId = new Map(rows.value.map(row => [row.id, row]))
  const kept = table.rows.filter(row => !remove.includes(row.id)).map(row => byId.get(row.id) ?? row)
  const added = rows.value.filter(row => !table.rows.some(old => old.id === row.id))
  return { ok: true, value: { ...table, rows: [...kept, ...added], overlayVersion: version } }
}

// The read-first skill files of the roles that start before a job has a plan, from the table `## Fixed manifests` of the builder skill.

export const PRE_PLAN_ROLES = ['decision-maker', 'decision-challenger', 'skill-writer'] as const

/** One read-first file: a skill's `SKILL.md`, or another file of that skill. */
export type ManifestEntry = { readonly skill: string; readonly file: string }

/** A backticked `<skill>` names its SKILL.md; `<skill>/<file>.md` names that file of the skill. */
const ENTRY = /`([a-z0-9-]+)(?:\/([A-Za-z0-9._/-]+\.md))?`/g

/** Each role row of the table, with the skill files its second column names in backticks. */
export function fixedManifests(skillText: string): ReadonlyMap<string, readonly ManifestEntry[]> {
  const rows = new Map<string, readonly ManifestEntry[]>()
  const lines = skillText.split('\n')
  const start = lines.findIndex(line => line.trim() === '## Fixed manifests')
  for (const line of start === -1 ? [] : lines.slice(start + 1)) {
    if (line.startsWith('## ')) {
      break
    }
    const cells = line.split('|').map(cell => cell.trim())
    const role = cells[1]?.replace(/`/g, '') ?? ''
    if (cells.length < 4 || role === '' || role === 'Role' || /^-+$/.test(role)) {
      continue
    }
    const entries = [...(cells[2] ?? '').matchAll(ENTRY)].map(found => ({ skill: found[1] ?? '', file: found[2] ?? 'SKILL.md' }))
    rows.set(
      role,
      entries.filter(entry => entry.skill !== '' && !entry.file.split('/').includes('..')),
    )
  }
  return rows
}

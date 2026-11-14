// The read-first skills of the roles that start before a job has a plan, from the table `## Fixed manifests` of the builder skill.

export const PRE_PLAN_ROLES = ['decision-maker', 'decision-challenger', 'skill-writer'] as const

/** Each role row of the table, with the skills its second column names in backticks. */
export function fixedManifests(skillText: string): ReadonlyMap<string, readonly string[]> {
  const rows = new Map<string, readonly string[]>()
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
    const skills = [...(cells[2] ?? '').matchAll(/`([a-z0-9-]+)`/g)].map(found => found[1] ?? '')
    rows.set(role, skills.filter(skill => skill !== ''))
  }
  return rows
}

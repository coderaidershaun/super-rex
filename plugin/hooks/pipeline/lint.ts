// The deterministic check of a brief's author text before an agent starts: the five sections, evidence in Acceptance, the phrases the prompting rules forbid, `never` outside Limits, and closed `<data>` tags.

export const SECTIONS = ['Outcome', 'Scope', 'Context', 'Acceptance', 'Limits'] as const
type Section = (typeof SECTIONS)[number]

const HEADER = /^(?:## )?(Outcome|Scope|Context|Acceptance|Limits):/

/** Lines that the effort level controls, not the brief; matched in any case. */
const FORBIDDEN = [
  'think carefully',
  'think step by step',
  'think hard',
  'take your time',
  'show your reasoning',
  'explain your thinking',
  'answer directly',
  'do not think',
  'minimize tool calls',
  'do not be lazy',
] as const
const SHOUTED = /\b(CRITICAL|IMPORTANT)\b/
const EVIDENCE = /`[^`]+`|\b(tests?|passes|prints|exit|output|file|log)\b/i

/** The task the first line names: `Task: <id>`. */
export function taskLine(text: string): string | null {
  const first = text.split('\n').find(line => line.trim() !== '')?.trim() ?? ''
  const found = /^Task:\s*(\S+)/.exec(first)
  return found?.[1] ?? null
}

/** The text of each section, from its header line to the next header. */
export function sectionsOf(text: string): ReadonlyMap<Section, string> {
  const sections = new Map<Section, string>()
  let current: Section | null = null
  for (const line of text.split('\n')) {
    const header = HEADER.exec(line)?.[1]
    const section = SECTIONS.find(each => each === header)
    if (section !== undefined) {
      current = section
      sections.set(section, `${line}\n`)
      continue
    }
    if (current !== null) {
      sections.set(current, `${sections.get(current) ?? ''}${line}\n`)
    }
  }
  return sections
}

/** Every line that is not in the Limits section. */
function outsideLimits(text: string): string {
  let current: Section | null = null
  const kept: string[] = []
  for (const line of text.split('\n')) {
    current = SECTIONS.find(each => each === HEADER.exec(line)?.[1]) ?? current
    if (current !== 'Limits') {
      kept.push(line)
    }
  }
  return kept.join('\n')
}

/** `tasks` is null before a plan, when every task name passes. */
export function lintFindings(text: string, tasks: readonly string[] | null): readonly string[] {
  const findings: string[] = []
  const task = taskLine(text)
  if (task === null) {
    findings.push('Task: is missing on the first line')
  } else if (tasks !== null && !tasks.includes(task)) {
    findings.push(`Task: ${task} names no task of the plan`)
  }
  const sections = sectionsOf(text)
  for (const section of SECTIONS) {
    if (!sections.has(section)) {
      findings.push(`${section}: is missing`)
    }
  }
  const acceptance = sections.get('Acceptance')
  if (acceptance !== undefined && !EVIDENCE.test(acceptance.replace(HEADER, ''))) {
    findings.push('Acceptance: names no evidence (a command in backticks, a test, output, an exit code, a file or a log)')
  }
  const lower = text.toLowerCase()
  findings.push(...FORBIDDEN.filter(phrase => lower.includes(phrase)).map(phrase => `"${phrase}" is a line the effort level controls`))
  const shouted = SHOUTED.exec(text)?.[1]
  if (shouted !== undefined) {
    findings.push(`${shouted} in capitals adds force, not meaning`)
  }
  if (/\bnever\b/i.test(outsideLimits(text))) {
    findings.push('"never" stands outside Limits:')
  }
  const opened = text.split('<data>').length - 1
  const closed = text.split('</data>').length - 1
  if (opened !== closed) {
    findings.push(`<data> tags do not pair: ${String(opened)} opened, ${String(closed)} closed`)
  }
  return findings
}

export function lintRefusal(findings: readonly string[]): string {
  return `rex brief check (1 of 1): ${findings.join('; ')}. Fix the brief and start the agent again.`
}

// The brief parts this module adds around the author's text: tags, the skills block, the tier text, the sandbox text and the Return block.
import type { RexBook, RexJob, RexTask } from '../../types'
import type { BriefContext, BriefFile, BriefPart } from '../parts'
import { SANDBOX_TEXT } from './sandbox'
import { gridOf, TIER_FILE, tierText } from './tiers'

export const PROMPTING_SKILL = 'rex-agent-prompting'

/** The author's own text stands here among the parts' orders. */
export const AUTHOR_ORDER = 30

export function taskOf(job: RexJob, task: string | null): RexTask | undefined {
  return job.plan?.tasks.find(each => each.id === task)
}

function isTierFile(file: BriefFile): boolean {
  return file.skill === PROMPTING_SKILL && Object.values(TIER_FILE).includes(file.file)
}

/** The output file of a role in a job: `<role>.md`, else the lowest free `-<n>` that no other run of the session holds. */
export function outputFileOf(book: RexBook, job: RexJob, role: string | null): string {
  const stem = `${job.repo}/.rex/plans/${job.id}/${role ?? 'agent'}`
  const taken = new Set(book.runs.map(run => run.outputFile))
  if (!taken.has(`${stem}.md`)) {
    return `${stem}.md`
  }
  let number = 2
  while (taken.has(`${stem}-${String(number)}.md`)) {
    number += 1
  }
  return `${stem}-${String(number)}.md`
}

function agentTypeOf(ctx: BriefContext): string {
  return ctx.spawn?.subagentType ?? ctx.run?.planned ?? ctx.run?.agentType ?? ''
}

export const OWN_PARTS: readonly BriefPart[] = [
  { name: 'tags', order: 10, text: ctx => `Rex-Job: ${ctx.job.id}\nRex-Role: ${ctx.role ?? 'none'}` },
  {
    name: 'skills',
    order: 20,
    text: ctx => {
      const attached = ctx.files.filter(file => file.file === 'SKILL.md' && !isTierFile(file))
      const opens = taskOf(ctx.job, ctx.task)?.openWhen ?? []
      if (attached.length === 0 && opens.length === 0) {
        return null
      }
      const skills = attached.map(file => `<skill name="${file.skill}" source="${file.source}" hash="${file.hash}">\n${file.text}\n</skill>`)
      const openLines = opens.map(open => `- ${open.skill}${open.file === null ? '' : `/${open.file}`}: ${open.when}`)
      const openBlock = openLines.length === 0 ? [] : [`Open when (call rex_skill with the name and file):\n${openLines.join('\n')}`]
      return ['<skills>', ...skills, ...openBlock, '</skills>'].join('\n')
    },
  },
  {
    name: 'tier',
    order: 40,
    text: ctx => {
      const grid = gridOf(agentTypeOf(ctx))
      const file = grid === null ? undefined : ctx.files.find(each => each.skill === PROMPTING_SKILL && each.file === TIER_FILE[grid.tier])
      return grid === null || file === undefined ? null : tierText(grid.tier, grid.level, file.text)
    },
  },
  { name: 'sandbox', order: 60, text: () => SANDBOX_TEXT },
  {
    name: 'return',
    order: 70,
    text: ctx =>
      `Return: write your full report to ${ctx.run?.outputFile ?? outputFileOf(ctx.book, ctx.job, ctx.role)}. Your final message has at most 2,000 characters: the path of that file, a summary of at most 5 lines, and one state: done, partial, awaiting decision, or blocked.`,
  },
]

/** The brief: each part's text in order, the author's text unchanged at its own order, joined by a blank line. */
export function assembled(parts: readonly BriefPart[], ctx: BriefContext, author: string | null): string {
  const ordered = [...parts].sort((a, b) => a.order - b.order)
  const pieces: string[] = []
  let isAuthorPlaced = author === null
  for (const part of ordered) {
    if (!isAuthorPlaced && part.order > AUTHOR_ORDER && author !== null) {
      pieces.push(author)
      isAuthorPlaced = true
    }
    const text = part.text(ctx)
    if (text !== null) {
      pieces.push(text)
    }
  }
  if (!isAuthorPlaced && author !== null) {
    pieces.push(author)
  }
  return pieces.join('\n\n')
}

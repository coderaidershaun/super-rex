// The decisions section of rex_status: each skill, answer and grid agent the job's record holds, with its source and reason, so the challenger checks recorded answers.
import type { RexDecision } from '../../types'
import type { StatusContext, StatusSection } from '../parts'

/** The part of a decision's key after `decision.<task or role>.`: a skill, a skill's other file, `<row>.<criterion>` of an answer, or the item itself. */
function itemKey(decision: RexDecision): string {
  if (decision.item === 'skill') return decision.value
  if (decision.item.startsWith('open_when.')) return decision.item.slice('open_when.'.length)
  if (decision.item.startsWith('answer.')) return decision.item.slice('answer.'.length)
  if (decision.item.startsWith('file.')) return decision.item.slice('file.'.length)
  return decision.item
}

function placementOf(decision: RexDecision): string {
  if (decision.item === 'skill') return 'read first'
  if (decision.item.startsWith('open_when.')) return `open when ${decision.value}`
  if (decision.item.startsWith('file.')) return `open when ${decision.value}`
  return decision.value
}

export function decisionLine(decision: RexDecision): string {
  const subject = decision.subject.slice(decision.subject.indexOf(':') + 1)
  return `decision.${subject}.${itemKey(decision)}=${decision.source}: ${placementOf(decision)}; ${decision.reason}`
}

function decisionLines(ctx: StatusContext): readonly string[] {
  return ctx.job?.decisions.map(decisionLine) ?? []
}

export const DECISIONS_SECTION: StatusSection = { name: 'decisions', order: 40, lines: decisionLines }

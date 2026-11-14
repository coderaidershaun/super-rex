// Serves skill files by name through rex_skill, so no agent reads a skill by a path that changes with each plugin version.
import type { EngineInterface, On, ToolCallResult } from 'claude-code'

import type { RexBook, RexJob } from '../../types'
import { fail, isOpen, readBook, recordSkillOpen } from '../book/index'
import type { Parts, ToolEntry } from '../parts'
import { findSkill, nameProblem } from './resolve'
import type { SkillPlaces } from './resolve'

export { findSkill, hashText, nameProblem, resolveSkill, sha256Of, skillFolders } from './resolve'
export type { ResolvedSkill, SkillLookup, SkillPlaces } from './resolve'

export const SKILL_TOOL: ToolEntry = {
  name: 'rex_skill',
  description:
    'Reads one file of a skill by its name: SKILL.md, or another .md file inside the skill. Answers skill, file, source and hash, then a blank line and the text. With no name: the skills block, tier text and Return block your brief is missing.',
  inputSchema: {
    type: 'object',
    properties: {
      name: { type: 'string', description: 'The skill name, such as rex-code-tdd.' },
      file: { type: 'string', description: 'A .md file inside the skill; default SKILL.md.' },
    },
  },
  isDeferred: false,
  callers: ['main', 'builder', 'master', 'agent'],
}

/** The job whose worktree a caller works in: its run's job, or the job it owns. */
function jobOfCaller(book: RexBook, agentId: string | undefined): RexJob | undefined {
  const builder = agentId ?? 'main'
  const owned = book.jobs.find(job => isOpen(job) && job.owner.session === book.session && job.owner.builder === builder)
  const runJob = agentId === undefined ? undefined : book.runs.find(run => run.agentId === agentId)?.job
  return owned ?? book.jobs.find(job => job.id === runJob)
}

/** The person's home folder; a session whose environment cannot be read has no personal skills folder, and the other folders still serve. */
async function homeOf($: EngineInterface): Promise<string | null> {
  try {
    return (await $.env.get('HOME')) ?? null
  } catch (error) {
    if (error instanceof Error) {
      return null
    }
    throw error
  }
}

/** The folders a caller's skills come from: the session root, its job's worktree, the plugin, and the person's home. */
export async function skillPlaces($: EngineInterface, job: RexJob | null | undefined): Promise<SkillPlaces> {
  const root = await $.session.root()
  return { repo: job?.repo ?? root, worktree: job?.worktree ?? null, pluginRoot: $.plugin.root, home: await homeOf($) }
}

function textOf(value: unknown, fallback: string): string {
  return typeof value === 'string' ? value : fallback
}

async function serveSkill($: EngineInterface, event: { readonly agentId?: string; readonly [key: string]: unknown }): Promise<ToolCallResult> {
  const name = textOf(event['name'], '')
  const file = textOf(event['file'] ?? 'SKILL.md', String(event['file']))
  const problem = nameProblem(name, file)
  if (problem !== null) {
    await fail($, { kind: 'bad-argument', tool: 'rex_skill', argument: 'name', value: `${name}/${file}` })
    return { deny: `rex_skill: ${problem}, got ${name}/${file}; call it with a skill name such as rex-code-tdd and a file such as SKILL.md` }
  }
  const book = await readBook($)
  const places = await skillPlaces($, jobOfCaller(book, event.agentId))
  const lookup = await findSkill($, places, name, file)
  if (lookup.kind === 'missing') {
    return { deny: `rex_skill: no skill ${name} holds ${file}, got ${name}/${file}; name a skill of plugin/skills, plugin/library or .rex/skills` }
  }
  if (lookup.kind === 'outside') {
    await fail($, { kind: 'bad-argument', tool: 'rex_skill', argument: 'name', value: `${name}/${file}` })
    return { deny: `rex_skill: ${name}/${file} leads outside its skill folder, got ${lookup.landed ?? 'a path that leads nowhere'}; name a skill of plugin/skills, plugin/library or .rex/skills` }
  }
  const found = lookup.skill
  await recordSkillOpen($, event.agentId ?? 'main', { skill: found.skill, file: found.file, hash: found.hash, how: 'rex_skill' })
  return { result: `skill=${found.skill}\nfile=${found.file}\nsource=${found.source}\nhash=${found.hash}\n\n${found.text}` }
}

export function register(on: On, parts: Parts): void {
  parts.tools.add(SKILL_TOOL)
  on('tool.call', { tool: 'mcp__super-rex__rex_skill' }, ($, e) => serveSkill($, e)).catch(async ($, _e, next) => {
    await fail($, { kind: 'authority-check-failed', check: 'rex_skill', error: next.error.message ?? next.error.kind })
    return { deny: `rex: rex_skill failed (${next.error.kind}); refused` }
  })
}

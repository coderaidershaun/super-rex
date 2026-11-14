// What the pipeline knows about one new brief before it asks the parts: its case, task, role, the skill files it attaches, and its lint verdict.
import type { AgentSpawnInput, EngineInterface } from 'claude-code'

import type { RexBook, RexJob, RexRun } from '../../types'
import { isOwnSpawn } from '../gate/checks'
import type { BriefFile } from '../parts'
import { resolveSkill, skillPlaces } from '../skills/index'
import { fixedManifests, PRE_PLAN_ROLES } from './manifests'
import { PROMPTING_SKILL, taskOf } from './parts'
import { gridOf, TIER_FILE } from './tiers'

export type SpawnCase = 'own' | 'resume' | 'team' | 'fork' | 'new' | 'other'

/** The cases of a spawn, first match wins; `job` is the job of the spawn's parent, or of the main loop. */
export function spawnCase(book: RexBook, event: AgentSpawnInput, origin: string | undefined, job: RexJob | undefined): SpawnCase {
  if (isOwnSpawn(book, event, origin)) return 'own'
  if (event.resume !== undefined) return 'resume'
  if (event.isTeammate === true || event.workflow !== undefined) return 'team'
  if (event.fork && job !== undefined) return 'fork'
  if (job !== undefined) return 'new'
  return 'other'
}

const PLUGIN_PREFIX = 'super-rex:'

function bare(agentType: string): string {
  return agentType.startsWith(PLUGIN_PREFIX) ? agentType.slice(PLUGIN_PREFIX.length) : agentType
}

/** The plan role of a new agent: its task's role, else the role whose grid agent it is (the parent's own role first), else a pre-plan role its Task line names. */
export function roleFor(book: RexBook, job: RexJob, task: string | null, agentType: string, parentAgentId: string | undefined): string | null {
  const planTask = taskOf(job, task)
  if (planTask !== undefined) {
    return planTask.role
  }
  if (job.plan === null) {
    return PRE_PLAN_ROLES.find(role => role === task) ?? null
  }
  const parentRole = parentAgentId === undefined ? null : (book.runs.find(run => run.agentId === parentAgentId)?.role ?? null)
  const matching = job.plan.roles.filter(role => role.gridAgent === bare(agentType))
  return (matching.find(role => role.role === parentRole) ?? matching[0])?.role ?? null
}

/** The read-first skills of a brief: its plan task's, else the fixed manifest of its pre-plan role. */
async function readFirst($: EngineInterface, job: RexJob, task: string | null, role: string | null): Promise<readonly string[]> {
  const planTask = taskOf(job, task)
  if (planTask !== undefined) {
    return planTask.skills
  }
  if (job.plan !== null || role === null) {
    return []
  }
  const builder = await resolveSkill($, await skillPlaces($, job), 'orchestration-builder', 'SKILL.md')
  return builder === null ? [] : (fixedManifests(builder.text).get(role) ?? [])
}

/** Every file the brief's parts read: each read-first SKILL.md, then the tier file of its grid agent. A skill no folder holds is left out. */
export async function briefFiles($: EngineInterface, job: RexJob, task: string | null, role: string | null, agentType: string): Promise<readonly BriefFile[]> {
  const places = await skillPlaces($, job)
  const grid = gridOf(agentType)
  const wanted = [...(await readFirst($, job, task, role)).map(skill => ({ skill, file: 'SKILL.md' })), ...(grid === null ? [] : [{ skill: PROMPTING_SKILL, file: TIER_FILE[grid.tier] }])]
  const files: BriefFile[] = []
  for (const one of wanted) {
    const found = await resolveSkill($, places, one.skill, one.file)
    if (found !== null) {
      files.push({ skill: found.skill, file: found.file, source: found.source, hash: found.hash, text: found.text })
    }
  }
  return files
}

/** The lint key: one brief of one parent for one task of one job is sent back once. */
export function lintKey(parentAgentId: string | undefined, job: RexJob, task: string | null): string {
  return `${parentAgentId ?? 'main'}|${job.id}|${task ?? '-'}`
}

/** A run's own brief facts for a late assembly: its task and role as the spawn recorded them. */
export function lateFacts(run: RexRun): { readonly task: string | null; readonly role: string | null; readonly agentType: string } {
  return { task: run.task, role: run.role, agentType: run.planned ?? run.agentType }
}

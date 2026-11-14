// The tools this module serves, as data: name, what the model reads, input schema, where the tool waits, and who may call it.
import type { ToolEntry } from '../parts'

const LEVEL = { type: 'string', enum: ['low', 'medium', 'high'] } as const
const RATING = { type: 'string', enum: ['low', 'medium', 'high', 'none'] } as const
const JOB = { type: 'string', description: 'Your own job id; left out, your own job.' } as const

export const STATUS_TOOL: ToolEntry = {
  name: 'rex_status',
  description: 'Where a job stands: status, agents, spend against its limit, degraded marks, answers to parked questions. key=value lines.',
  inputSchema: { type: 'object', properties: { job: { type: 'string', description: 'A job id; left out, your own job.' } } },
  isDeferred: false,
  callers: ['main', 'builder', 'master', 'agent'],
}

export const JOB_OPEN_TOOL: ToolEntry = {
  name: 'rex_job_open',
  description:
    "Opens the job of this /orchestration-builder request, or returns the one you already own. Answers its id, merge answer, limit, repo, branch, base and worktree. The merge answer comes from the person, never from an argument.",
  inputSchema: {
    type: 'object',
    properties: {
      request: { type: 'string', description: "The person's request, unchanged." },
      type: { type: 'string', enum: ['feature', 'fix', 'refactor', 'chore', 'research', 'other'] },
      size: { type: 'string', enum: ['small', 'medium', 'large'] },
      consequence: LEVEL,
      langs: { type: 'array', items: { type: 'string' } },
      prefs: { type: 'object', properties: { duration: LEVEL, budget: LEVEL, autonomy: LEVEL } },
    },
    required: ['request'],
  },
  isDeferred: true,
  callers: ['main', 'builder'],
}

export const RECALL_TOOL: ToolEntry = {
  name: 'rex_recall',
  description: 'Lessons and orchestrators the store holds for your job (storage on only).',
  inputSchema: { type: 'object', properties: { job: { type: 'string' } }, required: ['job'] },
  isDeferred: true,
  callers: ['builder'],
}

export const JOB_PLAN_TOOL: ToolEntry = {
  name: 'rex_job_plan',
  description:
    'Records the plan of your job once: tasks with their skills, roles with their ratings. Answers the grid agent of each role (super-rex:model-...) and the number of skill files in the manifest. A second call needs amend: true.',
  inputSchema: {
    type: 'object',
    properties: {
      job: JOB,
      orchestrator: { type: 'string' },
      breakdown: { type: 'string' },
      tasks: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            id: { type: 'string' },
            role: { type: 'string' },
            skills: { type: 'array', items: { type: 'string' } },
            open_when: { type: 'array', items: { type: 'object', properties: { skill: { type: 'string' }, file: { type: 'string' }, when: { type: 'string' } }, required: ['skill', 'when'] } },
          },
          required: ['id', 'role', 'skills', 'open_when'],
        },
      },
      roles: {
        type: 'array',
        items: {
          type: 'object',
          properties: { role: { type: 'string' }, intelligence: RATING, reliability: RATING, consequence: RATING, verification: RATING },
          required: ['role', 'intelligence', 'reliability', 'consequence', 'verification'],
        },
      },
      overrides: { type: 'array', items: { type: 'object', properties: { subject: { type: 'string' }, item: { type: 'string' }, value: { type: 'string' }, reason: { type: 'string' } } } },
      amend: { type: 'boolean' },
    },
    required: ['orchestrator', 'breakdown', 'tasks', 'roles'],
  },
  isDeferred: true,
  callers: ['builder'],
}

export const JOB_CLOSE_TOOL: ToolEntry = {
  name: 'rex_job_close',
  description:
    'Records the outcome of your job and answers its close report: spend, runs, skills missed or extra, long final messages. A delivery marked merged needs a merge by rex_merge. Safe to repeat.',
  inputSchema: {
    type: 'object',
    properties: {
      job: JOB,
      status: { type: 'string', enum: ['delivered', 'blocked', 'failed', 'cancelled'] },
      outcome: { type: 'string', description: 'One line.' },
      deliveries: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            repo: { type: 'string' },
            branch: { type: 'string' },
            pr: { type: 'string' },
            outcome: { type: 'string', enum: ['merged', 'pr-open', 'kept-local', 'blocked'] },
            merge_commit: { type: 'string' },
            note: { type: 'string' },
          },
          required: ['repo', 'outcome'],
        },
      },
    },
    required: ['status', 'outcome', 'deliveries'],
  },
  isDeferred: true,
  callers: ['builder'],
}

export const OWN_TOOLS: readonly ToolEntry[] = [STATUS_TOOL, JOB_OPEN_TOOL, RECALL_TOOL, JOB_PLAN_TOOL, JOB_CLOSE_TOOL]

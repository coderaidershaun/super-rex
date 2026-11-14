// What the master module adds to the shared tables, as data: its sub-commands, its tools, and its gate checks; its own hooks serve each one.
import type { GateCheck, Subcommand, ToolEntry } from '../parts'

export const MASTER_SUBCOMMAND: Subcommand = { name: 'master', summary: 'Starts a master tree from your goal or charter file, or resumes one', grantsAuthority: true }
export const POLICY_SUBCOMMAND: Subcommand = { name: 'policy', summary: 'Shows, confirms or sets what a master may decide as you', grantsAuthority: true }
export const ANSWER_SUBCOMMAND: Subcommand = { name: 'answer', summary: 'Answers a parked question as you', grantsAuthority: true }

export const SUBCOMMANDS: readonly Subcommand[] = [MASTER_SUBCOMMAND, POLICY_SUBCOMMAND, ANSWER_SUBCOMMAND]

const LEVEL = { type: 'string', enum: ['low', 'medium', 'high'] } as const
const RATING = { type: 'string', enum: ['low', 'medium', 'high', 'none'] } as const
const JOB_IDS = { type: 'array', items: { type: 'string' } } as const

export const BUILD_TOOL: ToolEntry = {
  name: 'rex_build',
  description:
    "Opens one job of your tree and starts its builder in the job's own worktree, with the fixed builder brief and your request. Refused over the policy: spend of the tree, rate limit, job counts, consequence. Answers job, builder, status and worktree.",
  inputSchema: {
    type: 'object',
    properties: {
      request: { type: 'string', description: 'What the builder must deliver: the outcome, and the evidence that shows it is done.' },
      consequence: LEVEL,
      size: { type: 'string', enum: ['small', 'medium', 'large'] },
      repo: { type: 'string', description: 'Left out, the session root; else a path of the policy key repos.' },
      depends_on: { ...JOB_IDS, description: 'Jobs that must be delivered before this one starts.' },
      merge_after: { ...JOB_IDS, description: 'Jobs that must be merged before this one merges.' },
      budget_usd: { type: 'number', description: 'The spend limit of the job; at most per_job_usd of the policy.' },
      ratings: {
        type: 'object',
        properties: { intelligence: RATING, reliability: RATING, consequence: RATING, verification: RATING },
        required: ['intelligence', 'reliability', 'consequence', 'verification'],
      },
    },
    required: ['request', 'consequence'],
  },
  isDeferred: true,
  callers: ['master'],
}

export const CONTINUE_TOOL: ToolEntry = {
  name: 'rex_continue',
  description: 'Sends your message to the builder of one job of your tree and wakes it, as the answer to its question or to go on. Answers sent and job.',
  inputSchema: {
    type: 'object',
    properties: { job: { type: 'string' }, message: { type: 'string', description: 'What the builder reads.' } },
    required: ['job', 'message'],
  },
  isDeferred: true,
  callers: ['master'],
}

export const NOTE_TOOL: ToolEntry = {
  name: 'rex_note',
  description: 'Records one decision of yours in the tree: what you decided, why, and the option you did not take. Answers noted, the count of notes.',
  inputSchema: { type: 'object', properties: { text: { type: 'string' }, job: { type: 'string' } }, required: ['text'] },
  isDeferred: true,
  callers: ['master'],
}

export const ASK_OWNER_TOOL: ToolEntry = {
  name: 'rex_ask_owner',
  description:
    'Parks one question for the person and returns at once with its id; carry on with other jobs. Read the answer later with rex_status (answer.<id>=).',
  inputSchema: {
    type: 'object',
    properties: {
      question: { type: 'string' },
      options: { type: 'array', items: { type: 'string' }, minItems: 2, maxItems: 4 },
      recommendation: { type: 'string', description: 'One of the options.' },
      category: { type: 'string', enum: ['merge', 'scope', 'spend', 'other'] },
      job: { type: 'string' },
    },
    required: ['question', 'options', 'recommendation', 'category'],
  },
  isDeferred: true,
  callers: ['master'],
}

export const TOOLS: readonly ToolEntry[] = [BUILD_TOOL, CONTINUE_TOOL, NOTE_TOOL, ASK_OWNER_TOOL]

export const BUILD_TOOL_NAME = 'mcp__super-rex__rex_build'
export const MERGE_TOOL_NAME = 'mcp__super-rex__rex_merge'

export const GATE_CHECKS: readonly GateCheck[] = [
  { name: 'tree-ledger', site: { kind: 'tool', tool: BUILD_TOOL_NAME }, failure: 'closed' },
  { name: 'rate-limit-pause', site: { kind: 'tool', tool: BUILD_TOOL_NAME }, failure: 'closed' },
  { name: 'max-jobs', site: { kind: 'tool', tool: BUILD_TOOL_NAME }, failure: 'closed' },
  { name: 'policy-merge', site: { kind: 'tool', tool: MERGE_TOOL_NAME }, failure: 'closed' },
]

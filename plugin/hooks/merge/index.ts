// The only path by which a job's work reaches its base branch: rex_merge pushes the job's own squash commit after the person's merge answer.
import type { On } from 'claude-code'

import type { Parts, ToolEntry } from '../parts'
import { toolFailed } from '../tools/index'
import { serveMerge } from './steps'

export const MERGE_TOOL: ToolEntry = {
  name: 'rex_merge',
  description:
    "Merges your job: pushes its branch, one squash commit over origin/<base>, to the base as a fast forward, once the person's merge answer is yes. Takes no repo, branch or base. Answers merged=yes with merge_commit, or merged=no with a question id to wait on.",
  inputSchema: { type: 'object', properties: { job: { type: 'string', description: 'Your own job id; left out, your own job.' } } },
  isDeferred: true,
  callers: ['builder'],
}

export function register(on: On, parts: Parts): void {
  parts.tools.add(MERGE_TOOL)
  on('tool.call', { tool: 'mcp__super-rex__rex_merge' }, ($, e) => serveMerge($, e, MERGE_TOOL)).catch(($, _e, next) => toolFailed($, 'rex_merge', next.error))
}

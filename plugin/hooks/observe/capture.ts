// The 0.7 capture fixture of a run tree, converted once into the events the mod hears: spawns, model requests and ends, with the fixture's own totals.
// It mirrors tests/fixtures/capture/tree: six agents, two of them children of other agents, and their token counts.

export type CaptureEvent =
  | { readonly kind: 'spawn'; readonly agentId: string; readonly parentAgentId: string | null; readonly subagentType: string; readonly description: string; readonly toolUseId: string; readonly background: boolean }
  | { readonly kind: 'step'; readonly agentId: string; readonly model: string; readonly effort: 'low' | 'medium' | 'high'; readonly input: number; readonly output: number; readonly cacheRead: number; readonly cacheCreation: number }
  | { readonly kind: 'complete'; readonly agentId: string }

export const CAPTURE_EVENTS: readonly CaptureEvent[] = [
  { kind: 'spawn', agentId: 'a70d8a0f630d8701c', parentAgentId: null, subagentType: 'model-low-effort-medium', description: 'bg', toolUseId: 'toolu_01LPtckQHYXYFh6qT3ozFcLD', background: true },
  { kind: 'step', agentId: 'a70d8a0f630d8701c', model: 'claude-haiku-5-5', effort: 'medium', input: 2, output: 344, cacheRead: 6338, cacheCreation: 3673 },
  { kind: 'spawn', agentId: 'acefc7f2b8d9b97bf', parentAgentId: null, subagentType: 'model-medium-effort-low', description: 'lead', toolUseId: 'toolu_01T2bk99iix4HjrLFzGK9fte', background: false },
  { kind: 'step', agentId: 'acefc7f2b8d9b97bf', model: 'claude-sonnet-5-5', effort: 'low', input: 2, output: 176, cacheRead: 0, cacheCreation: 11350 },
  { kind: 'step', agentId: 'a70d8a0f630d8701c', model: 'claude-haiku-5-5', effort: 'medium', input: 2, output: 114, cacheRead: 10011, cacheCreation: 2025 },
  { kind: 'step', agentId: 'acefc7f2b8d9b97bf', model: 'claude-sonnet-5-5', effort: 'low', input: 2, output: 157, cacheRead: 11350, cacheCreation: 1963 },
  { kind: 'step', agentId: 'a70d8a0f630d8701c', model: 'claude-haiku-5-5', effort: 'medium', input: 2, output: 137, cacheRead: 12036, cacheCreation: 155 },
  { kind: 'step', agentId: 'a70d8a0f630d8701c', model: 'claude-haiku-5-5', effort: 'medium', input: 2, output: 196, cacheRead: 12191, cacheCreation: 237 },
  { kind: 'spawn', agentId: 'a0741c86685f0104e', parentAgentId: 'a70d8a0f630d8701c', subagentType: 'model-low-effort-low', description: 'bg-child', toolUseId: 'toolu_01HiK9kuB8hs1gRLpMhRzQ3U', background: false },
  { kind: 'step', agentId: 'acefc7f2b8d9b97bf', model: 'claude-sonnet-5-5', effort: 'low', input: 2, output: 203, cacheRead: 13313, cacheCreation: 260 },
  { kind: 'spawn', agentId: 'a8b103ab4ef48cdf5', parentAgentId: 'acefc7f2b8d9b97bf', subagentType: 'model-low-effort-low', description: 'worker', toolUseId: 'toolu_01QLPsrh2bxTi8rGEYxa5YzB', background: false },
  { kind: 'step', agentId: 'a0741c86685f0104e', model: 'claude-haiku-5-5', effort: 'low', input: 2, output: 88, cacheRead: 7564, cacheCreation: 3538 },
  { kind: 'step', agentId: 'a8b103ab4ef48cdf5', model: 'claude-haiku-5-5', effort: 'low', input: 2, output: 222, cacheRead: 7564, cacheCreation: 3547 },
  { kind: 'step', agentId: 'a0741c86685f0104e', model: 'claude-haiku-5-5', effort: 'low', input: 2, output: 12, cacheRead: 11102, cacheCreation: 1772 },
  { kind: 'complete', agentId: 'a0741c86685f0104e' },
  { kind: 'step', agentId: 'a8b103ab4ef48cdf5', model: 'claude-haiku-5-5', effort: 'low', input: 2, output: 9, cacheRead: 11111, cacheCreation: 1958 },
  { kind: 'complete', agentId: 'a8b103ab4ef48cdf5' },
  { kind: 'step', agentId: 'a70d8a0f630d8701c', model: 'claude-haiku-5-5', effort: 'medium', input: 2, output: 7, cacheRead: 12428, cacheCreation: 471 },
  { kind: 'complete', agentId: 'a70d8a0f630d8701c' },
  { kind: 'step', agentId: 'acefc7f2b8d9b97bf', model: 'claude-sonnet-5-5', effort: 'low', input: 2, output: 9, cacheRead: 13573, cacheCreation: 512 },
  { kind: 'complete', agentId: 'acefc7f2b8d9b97bf' },
  { kind: 'spawn', agentId: 'ab99f22c637e9bce7', parentAgentId: null, subagentType: 'model-high-effort-low', description: 'reviewer', toolUseId: 'toolu_01BVtAYmZSncEg3Muq9urg8U', background: false },
  { kind: 'spawn', agentId: 'a2caf7515d48b82f2', parentAgentId: null, subagentType: 'general-purpose', description: 'plain', toolUseId: 'toolu_011X8QxXJkvJCJFJFzPSN8uz', background: false },
  { kind: 'step', agentId: 'ab99f22c637e9bce7', model: 'claude-opus-5-5', effort: 'low', input: 2, output: 88, cacheRead: 0, cacheCreation: 11115 },
  { kind: 'step', agentId: 'ab99f22c637e9bce7', model: 'claude-opus-5-5', effort: 'low', input: 2, output: 11, cacheRead: 11115, cacheCreation: 1784 },
  { kind: 'complete', agentId: 'ab99f22c637e9bce7' },
  { kind: 'step', agentId: 'a2caf7515d48b82f2', model: 'claude-sonnet-5-5', effort: 'low', input: 2, output: 10, cacheRead: 7089, cacheCreation: 4422 },
  { kind: 'complete', agentId: 'a2caf7515d48b82f2' },
]

export type CaptureTotal = { readonly agentId: string; readonly parent: string; readonly depth: number; readonly input: number; readonly output: number; readonly cacheRead: number; readonly cacheCreation: number }

/** The fixture's expected.json, agent by agent: one model each, so its token counts are the agent's. */
export const CAPTURE_TOTALS: readonly CaptureTotal[] = [
  { agentId: 'a70d8a0f630d8701c', parent: 'main', depth: 1, input: 10, output: 798, cacheRead: 53004, cacheCreation: 6561 },
  { agentId: 'acefc7f2b8d9b97bf', parent: 'main', depth: 1, input: 8, output: 545, cacheRead: 38236, cacheCreation: 14085 },
  { agentId: 'a0741c86685f0104e', parent: 'a70d8a0f630d8701c', depth: 2, input: 4, output: 100, cacheRead: 18666, cacheCreation: 5310 },
  { agentId: 'a8b103ab4ef48cdf5', parent: 'acefc7f2b8d9b97bf', depth: 2, input: 4, output: 231, cacheRead: 18675, cacheCreation: 5505 },
  { agentId: 'ab99f22c637e9bce7', parent: 'main', depth: 1, input: 4, output: 99, cacheRead: 11115, cacheCreation: 12899 },
  { agentId: 'a2caf7515d48b82f2', parent: 'main', depth: 1, input: 2, output: 10, cacheRead: 7089, cacheCreation: 4422 },
]

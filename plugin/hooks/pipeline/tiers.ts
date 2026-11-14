// The tier text of a grid agent: which fenced blocks of which rex-agent-prompting tier file its model and effort level get.

export type Tier = 'low' | 'medium' | 'high' | 'xhigh'
export type Level = 'low' | 'medium' | 'high' | 'xhigh' | 'max'

export const TIERS: readonly Tier[] = ['low', 'medium', 'high', 'xhigh']
export const LEVELS: readonly Level[] = ['low', 'medium', 'high', 'xhigh', 'max']

/** The tier file of each model tier, inside the skill rex-agent-prompting. */
export const TIER_FILE: { readonly [tier in Tier]: string } = { low: 'haiku.md', medium: 'sonnet.md', high: 'opus.md', xhigh: 'fable.md' }

/**
 * The blocks each tier and level gets, by label, in order. Every spawned agent
 * works with no person, so Opus gets its stops text at each level; every brief
 * has a Return block, so Fable's report text is left out.
 */
export const TIER_BLOCKS: { readonly [tier in Tier]: { readonly [level in Level]: readonly string[] } } = {
  low: { low: ['Check', 'Finish'], medium: ['Check', 'Finish'], high: [], xhigh: [], max: [] },
  medium: { low: ['Finish', 'Check'], medium: ['Finish'], high: ['Stop'], xhigh: ['Stop', 'Review'], max: ['Stop', 'Review'] },
  high: { low: ['Stops'], medium: ['Stops'], high: ['Stops'], xhigh: ['Stops'], max: ['Stops'] },
  xhigh: { low: ['Finish'], medium: ['Finish'], high: [], xhigh: [], max: [] },
}

const GRID_AGENT = /^(?:super-rex:)?model-(low|medium|high|xhigh)-effort-(low|medium|high|xhigh|max)$/

export function gridOf(agentType: string): { readonly tier: Tier; readonly level: Level } | null {
  const found = GRID_AGENT.exec(agentType)
  const tier = TIERS.find(each => each === found?.[1])
  const level = LEVELS.find(each => each === found?.[2])
  return tier === undefined || level === undefined ? null : { tier, level }
}

const FENCE = '```'

/**
 * Each fenced block under `## Brief text`, by its label: the words before
 * ` text` on the nearest line above the fence (`Stops text.` gives `Stops`).
 */
export function briefBlocks(markdown: string): ReadonlyMap<string, string> {
  const blocks = new Map<string, string>()
  const lines = markdown.split('\n')
  const start = lines.findIndex(line => line.trim() === '## Brief text')
  let label: string | null = null
  let body: string[] | null = null
  for (const line of start === -1 ? [] : lines.slice(start + 1)) {
    if (body === null && line.startsWith('## ')) {
      break
    }
    if (line.trim() === FENCE) {
      if (body === null) {
        body = []
      } else {
        if (label !== null) {
          blocks.set(label, body.join('\n'))
        }
        body = null
        label = null
      }
      continue
    }
    if (body !== null) {
      body.push(line)
    } else if (line.trim() !== '') {
      label = /^(.*?) text\b/.exec(line.trim())?.[1] ?? null
    }
  }
  return blocks
}

export class MissingTierBlockError extends Error {
  readonly file: string
  readonly label: string

  constructor(file: string, label: string) {
    super(`the tier file ${file} has no fenced block labelled "${label} text" under ## Brief text`)
    this.name = 'MissingTierBlockError'
    this.file = file
    this.label = label
  }
}

/** The tier text of one grid agent, its blocks joined by a blank line; null for a level that gets none. */
export function tierText(tier: Tier, level: Level, markdown: string): string | null {
  const labels = TIER_BLOCKS[tier][level]
  if (labels.length === 0) {
    return null
  }
  const blocks = briefBlocks(markdown)
  return labels
    .map(label => {
      const text = blocks.get(label)
      if (text === undefined) {
        throw new MissingTierBlockError(TIER_FILE[tier], label)
      }
      return text
    })
    .join('\n\n')
}

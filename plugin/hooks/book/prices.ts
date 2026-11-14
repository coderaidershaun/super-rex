// List prices of the Claude API in billionths of a dollar per token, so the live tally is whole numbers; the binary keeps the cost of record.
import type { TurnUsage } from 'claude-code'

type Rates = { readonly input: number; readonly cacheWrite: number; readonly cacheRead: number; readonly output: number }

/** A cache write is priced as the 5-minute kind, 1.25 times input: a step's usage does not say which kind it was. */
function rates(input: number, cacheRead: number, output: number): Rates {
  return { input, cacheWrite: (input * 5) / 4, cacheRead, output }
}

/** Above this many prompt tokens in one request, Claude Haiku 5.5 charges its second rate card. */
const HAIKU_5_5_SMALL_PROMPT = 100_000

function ratesOf(model: string, promptTokens: number): Rates | null {
  switch (model) {
    case 'claude-fable-5-2':
    case 'claude-mythos-5-2':
      return rates(8_000, 200, 40_000)
    case 'claude-fable-5-1':
    case 'claude-mythos-5-1':
      return rates(10_000, 250, 50_000)
    case 'claude-fable-5':
    case 'claude-mythos-5':
      return rates(10_000, 1_000, 50_000)
    case 'claude-opus-5-5':
      return rates(4_000, 200, 20_000)
    case 'claude-opus-5':
    case 'claude-opus-4-8':
    case 'claude-opus-4-7':
    case 'claude-opus-4-6':
      return rates(5_000, 500, 25_000)
    case 'claude-sonnet-5-5':
      return rates(2_000, 100, 10_000)
    case 'claude-sonnet-5':
      return rates(2_000, 200, 10_000)
    case 'claude-sonnet-4-6':
      return rates(3_000, 300, 15_000)
    case 'claude-haiku-5-5':
      return promptTokens > HAIKU_5_5_SMALL_PROMPT ? rates(500, 50, 2_500) : rates(100, 10, 500)
    case 'claude-haiku-4-5':
      return rates(1_000, 100, 5_000)
    default:
      return null
  }
}

/** What one request cost in nanodollars; a model with no price costs 0 here, and the binary prices it later. */
export function priceNano(usage: TurnUsage): number {
  const prompt = usage.input_tokens + usage.cache_read_input_tokens + usage.cache_creation_input_tokens
  const found = ratesOf(usage.model, prompt)
  if (found === null) {
    return 0
  }
  const total =
    usage.input_tokens * found.input +
    usage.cache_creation_input_tokens * found.cacheWrite +
    usage.cache_read_input_tokens * found.cacheRead +
    usage.output_tokens * found.output
  return Math.round(total)
}

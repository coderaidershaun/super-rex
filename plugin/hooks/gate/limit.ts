// `/rex limit <job> [usd]`: shows a job's spend and limit, or sets the limit, from the person's own typing only.
import type { CommandRunInput, CommandRunResult, EngineInterface } from 'claude-code'

import { fail, MAX_LIMIT_USD, readBook, setLimit } from '../book/index'
import { originRefusal, rexLine } from '../commands/index'
import type { Subcommand } from '../parts'

export const LIMIT_SUBCOMMAND: Subcommand = { name: 'limit', summary: "Shows or sets a job's spend limit", grantsAuthority: true }

const AMOUNT = /^\d+(?:\.\d+)?$/

function dollars(nano: number): string {
  return (nano / 1e9).toFixed(2)
}

export async function serveLimit($: EngineInterface, event: CommandRunInput): Promise<CommandRunResult> {
  const refusal = originRefusal(event, LIMIT_SUBCOMMAND, false)
  if (refusal !== null) {
    await fail($, { kind: 'origin-not-person', action: 'rex limit', origin: event.origin.kind })
    return { text: refusal }
  }
  const [id = '', amount, ...rest] = rexLine(event).args.split(/\s+/).filter(word => word !== '')
  const book = await readBook($)
  const job = book.jobs.find(each => each.id === id)
  if (job === undefined) {
    return { text: `rex limit: no job ${id === '' ? 'named' : id} in this session, got ${id === '' ? 'none' : id}; name a job of /rex jobs` }
  }
  if (amount !== undefined) {
    const usd = Number(amount)
    if (!AMOUNT.test(amount) || rest.length > 0 || usd > MAX_LIMIT_USD) {
      const text = [amount, ...rest].join(' ')
      return { text: `rex limit: ${text} is not an amount from 0 to ${String(MAX_LIMIT_USD)} dollars, got ${text}` }
    }
    await setLimit($, job.id, { nano: Math.round(usd * 1e9), source: 'set' })
  }
  const shown = (await readBook($)).jobs.find(each => each.id === job.id) ?? job
  return { text: `job=${shown.id}\nspent_usd=${dollars(shown.spentNano)}\nlimit_usd=${dollars(shown.limit.nano)}\nlimit_source=${shown.limit.source}\n` }
}

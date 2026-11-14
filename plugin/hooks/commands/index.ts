// Routes the one slash command, /rex: other modules answer their own sub-commands, and this router answers its own and every unknown name.
import type { On } from 'claude-code'

import { fail, readBook } from '../book/index'
import type { Parts } from '../parts'
import { OWN_SUBCOMMANDS, route } from './router'

export { originRefusal, rexLine, unknownLine } from './router'
export type { RexLine } from './router'

export const REX_COMMAND = {
  name: 'rex',
  description: 'SuperRex: watch, status, jobs, doctor, prompts, limit, master, policy, answer',
  argumentHint: '<sub-command> [args]',
  immediate: true,
} as const

export function register(on: On, parts: Parts): void {
  for (const entry of OWN_SUBCOMMANDS) {
    parts.subcommands.add(entry)
  }

  on('session.start', { cwd: /(?:)/ }, async ($, e, next) => {
    const book = await readBook($)
    if (book.inert === null) {
      await $.command.register(REX_COMMAND)
    }
    return next(e)
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'session.start', module: 'commands', agentId: null, error: next.error.message ?? next.error.kind })
    return next(e)
  })

  on('command.run', { command: 'rex' }, ($, e) => route($, e, parts.subcommands)).catch((_$, e, next) => ({
    text: `rex: ${e.args.split(' ')[0] ?? ''} failed (${next.error.message ?? next.error.kind}); nothing was changed after the failure`,
  }))
}

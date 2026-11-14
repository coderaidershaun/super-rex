// Cuts the passing lines out of a test run's output before the agent reads it, because every line it reads stays in its context. An inert mod cuts nothing.
import type { On } from 'claude-code'

import { fail, readBook } from '../book/index'
import type { Parts } from '../parts'
import { isTestRun } from './command'
import { cutPassing } from './cut'

export { isTestRun } from './command'
export { cutPassing } from './cut'

export const SEE_EVERY_LINE = 'To see every line, run the command again with | cat at its end.'

export function register(on: On, _parts: Parts): void {
  on('tool.call', { tool: 'Bash' }, async ($, e, next) => {
    if (!isTestRun(e.command) || (await readBook($)).inert !== null) {
      return next(e)
    }
    const ran = await next(e)
    if (ran.deny !== undefined || ran.isError === true) {
      return ran
    }
    const stdout = cutPassing(ran.result.stdout)
    const stderr = cutPassing(ran.result.stderr)
    const left = stdout.left + stderr.left
    if (left === 0) {
      return ran
    }
    const note = `rex trim: left out ${String(left)} passing lines of ${String(stdout.of + stderr.of)}. ${SEE_EVERY_LINE}\n`
    return { result: { ...ran.result, stdout: `${stdout.text}${note}`, stderr: stderr.text } }
  }).catch(async ($, e, next) => {
    await fail($, { kind: 'observer-failed', event: 'tool.call', module: 'trim', agentId: e.agentId ?? null, error: next.error.message ?? next.error.kind })
    return next(e)
  })
}

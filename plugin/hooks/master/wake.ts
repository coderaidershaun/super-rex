// Wakes a master or a builder with a message, always from a timer and never from inside a hook, where a woken agent's calls would skip that hook.
import type { EngineInterface } from 'claude-code'

import { errorText } from '../book/index'

/** What a woken builder reads: go on from where its record says it stopped. */
export const CONTINUE_TEXT = (job: string): string => `Continue job ${job} from the first step not done.`

export function wakeLater($: EngineInterface, agentId: string, text: string): void {
  $.clock.after(0, () => {
    void $.session
      .send({ to: { agentId }, text })
      .then(sent => {
        if (!sent.isDelivered) {
          $.ui.status(`rex: ${agentId} was not woken: ${sent.reason}`)
        }
      })
      .catch((error: unknown) => {
        $.ui.status(`rex: ${agentId} was not woken: ${errorText(error)}`)
      })
  })
}

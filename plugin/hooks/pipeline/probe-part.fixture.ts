// A brief part that only a test adds, as another module adds one: pure text from the context the pipeline builds.
import type { BriefContext, BriefPart } from '../parts'

export const seenContexts: BriefContext[] = []

export const PROBE_PART: BriefPart = {
  name: 'probe',
  order: 50,
  text: ctx => {
    seenContexts.push(ctx)
    return `Probe: job ${ctx.job.id}, role ${ctx.role ?? 'none'}, task ${ctx.task ?? 'none'}, ${String(ctx.files.length)} files`
  },
}

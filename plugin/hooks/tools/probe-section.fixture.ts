// A status section that only a test adds, as another module adds one: pure lines from the context rex_status builds.
import type { StatusContext, StatusSection } from '../parts'

export const PROBE_SECTION: StatusSection = {
  name: 'probe',
  order: 25,
  lines: (ctx: StatusContext) => [`probe.caller=${ctx.caller.kind}`, `probe.job=${ctx.job?.id ?? 'none'}`, `probe.runs=${String(ctx.book.runs.length)}`],
}

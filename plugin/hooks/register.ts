// The hooks module of the super-rex plugin: it builds the shared tables and lets each module register its hooks, in an order that decides which hook sees a call first.
//
// Priorities, in order: correctness (authority fails closed), visible state (an observer that fails marks the job degraded), simplicity.
// Never: merge without a merge answer from the person or the pinned policy.
// Never: start a master or a builder from anything but the person's own prompt or the master's own tool.
// Never: let a failed observer block a prompt, a tool call or a spawn.
import type { Register } from 'claude-code'

import { register as registerAssign } from './assign/index'
import { register as registerBook } from './book/index'
import { register as registerCommands } from './commands/index'
import { register as registerGate } from './gate/index'
import { register as registerIngest } from './ingest/index'
import { register as registerMaster } from './master/index'
import { register as registerMerge } from './merge/index'
import { register as registerObserve } from './observe/index'
import { register as registerPane } from './pane/index'
import { createParts } from './parts'
import { register as registerPipeline } from './pipeline/index'
import { register as registerPrompts } from './prompts/index'
import { register as registerSkills } from './skills/index'
import { register as registerTools } from './tools/index'
import { register as registerTrim } from './trim/index'

export const register: Register = on => {
  const parts = createParts()

  registerBook(on, parts)
  registerObserve(on, parts)
  registerPrompts(on, parts)
  registerIngest(on, parts)
  registerGate(on, parts)
  registerPipeline(on, parts)
  registerTrim(on, parts)
  // The master's own tool registrations must stand above the tools dispatcher:
  // an agent that a hook spawned skips only that hook, so the builder the master
  // starts still reaches every tool the dispatcher serves.
  registerMaster(on, parts)
  registerAssign(on, parts)
  registerSkills(on, parts)
  registerMerge(on, parts)
  registerTools(on, parts)
  registerCommands(on, parts)
  registerPane(on, parts)
}

// Proves the library guard: an agent of an open job is refused the raw tools of the skill library and told to use the mod's tools, while a session with no open job and the mod's own call of the library server pass.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'

import type { RexBook } from '../types'
import { emptyBook } from './book/events'
import { closeJobIn } from './book/jobs'
import { startRunIn } from './book/runs'
import { ROOT, SESSION, START_MS, world } from './book/kit'
import { files, JOB, seededBook } from './tools/act-kit'
import { LIBRARY_GUARD_CHECKS, LIBRARY_TOOLS, libraryRefusal } from './library-guard'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const

const REFUSAL = [
  'rex library guard: an agent of a job must not call the raw tools of the skill library.',
  'Use the tools of the mod, so that the mod records each use of the library.',
  'To read a skill file, call rex_skill with the name of the skill.',
  'Add the name of the file when it is not SKILL.md.',
  'The tool rex_skill_search will replace the raw search tool.',
  'It does not exist yet, so use the skills that your brief names.',
].join(' ')

const RAW_TOOLS = ['mcp__plugin_super-rex_library__head', 'mcp__plugin_super-rex_library__search', 'mcp__plugin_super-rex_library__get'] as const

/** The engine below every plugin: it runs each tool call that reaches it and answers each call of an MCP server. */
function engine(on: On): { readonly ran: string[]; readonly served: string[] } {
  const ran: string[] = []
  const served: string[] = []
  on('mcp.call', (_$, e) => {
    served.push(`${e.server} ${e.tool}`)
    return { value: { content: [{ type: 'text', text: `served ${e.tool}` }], isError: false } }
  })
  on('tool.call', (_$, e) => {
    ran.push(e.tool)
    return { result: `ran ${e.tool}` }
  })
  return { ran, served }
}

function withRun(book: RexBook, agentId: string, parentAgentId: string | null): RexBook {
  return startRunIn(book, { agentId, toolUseId: null, parentAgentId, agentType: 'super-rex:model-high-effort-high', description: agentId, isOwnSpawn: false }, START_MS)
}

/** Each kind of agent of an open job: the builder as the main loop, a sub-agent of it, a builder that a master started, and a sub-agent of that builder. */
const JOB_AGENTS: readonly { readonly name: string; readonly seed: RexBook; readonly agentId: string | undefined }[] = [
  { name: 'the builder as the main loop', seed: seededBook(), agentId: undefined },
  { name: 'a sub-agent of the main loop builder', seed: withRun(seededBook(), 'w1', null), agentId: 'w1' },
  { name: 'a builder that runs as an agent', seed: seededBook({ builder: 'b1' }), agentId: 'b1' },
  { name: 'a sub-agent of a builder that runs as an agent', seed: withRun(withRun(seededBook({ builder: 'b1' }), 'b1', null), 'w2', 'b1'), agentId: 'w2' },
]

test('the guard names the three raw tools of the library server, each a check that fails closed', () => {
  expect([...LIBRARY_TOOLS]).toEqual([...RAW_TOOLS])
  expect(LIBRARY_GUARD_CHECKS).toEqual([
    { name: 'library-guard-head', site: { kind: 'tool', tool: 'mcp__plugin_super-rex_library__head' }, failure: 'closed' },
    { name: 'library-guard-search', site: { kind: 'tool', tool: 'mcp__plugin_super-rex_library__search' }, failure: 'closed' },
    { name: 'library-guard-get', site: { kind: 'tool', tool: 'mcp__plugin_super-rex_library__get' }, failure: 'closed' },
  ])
  expect(libraryRefusal()).toBe(REFUSAL)
})

for (const agent of JOB_AGENTS) {
  test(`each raw library tool is refused to ${agent.name} of an open job, with the refusal text`, async ($, on) => {
    files(on, {})
    const { ran } = engine(on)
    world(on, { seed: agent.seed })
    await $.session.start(START)

    const answers = []
    for (const tool of RAW_TOOLS) {
      answers.push(await $.tool.call({ tool, name: 'rex-code-tdd', ...(agent.agentId === undefined ? {} : { agentId: agent.agentId }) }))
    }

    expect(answers.map(answer => answer.deny)).toEqual([REFUSAL, REFUSAL, REFUSAL])
    expect(ran).toEqual([])
  })
}

test('in a session with no open job, each raw library tool reaches the server, from the main loop and from an agent', async ($, on) => {
  files(on, {})
  const { ran } = engine(on)
  const noJob: RexBook = withRun({ ...emptyBook(), session: SESSION }, 'w1', null)
  world(on, { seed: noJob })
  await $.session.start(START)

  const answers = []
  for (const tool of RAW_TOOLS) {
    answers.push(await $.tool.call({ tool }))
    answers.push(await $.tool.call({ tool, agentId: 'w1' }))
  }

  expect(answers.map(answer => answer.deny)).toEqual([undefined, undefined, undefined, undefined, undefined, undefined])
  expect(ran).toEqual(RAW_TOOLS.flatMap(tool => [tool, tool]))
})

test('after the job closed, its builder and its sub-agent reach the raw library tools again', async ($, on) => {
  files(on, {})
  const { ran } = engine(on)
  const closed = closeJobIn(withRun(seededBook(), 'w1', null), JOB, { status: 'cancelled', outcome: 'cancelled by the person', deliveries: [] }, START_MS)
  world(on, { seed: closed })
  await $.session.start(START)

  const fromMain = await $.tool.call({ tool: 'mcp__plugin_super-rex_library__get', name: 'rex-code-tdd' })
  const fromAgent = await $.tool.call({ tool: 'mcp__plugin_super-rex_library__get', name: 'rex-code-tdd', agentId: 'w1' })

  expect([fromMain.deny, fromAgent.deny]).toEqual([undefined, undefined])
  expect(ran).toEqual(['mcp__plugin_super-rex_library__get', 'mcp__plugin_super-rex_library__get'])
})

/** A mod tool that reads the library through `$.mcp.call`, as the mod's own library tools do. The test kit lets only a plugin's hooks module make that call. */
const READER = {
  name: 'library-reader',
  register(on: On): void {
    on('tool.call', { tool: 'mcp__probe__read_library' }, async ($, e) => {
      const answers: string[] = []
      for (const tool of ['head', 'search', 'get']) {
        const answer = await $.mcp.call('plugin:super-rex:library', tool, { name: 'rex-code-tdd' })
        answers.push(answer.content.map(block => (block.type === 'text' ? block.text : '')).join(''))
      }
      return { result: `${e.agentId ?? 'main'}: ${answers.join(', ')}` }
    })
  },
}

test('the guard sits on tool.call only: a $.mcp.call to the library server passes while a job is open, made inside a tool call of an agent of that job', { plugins: [READER] }, async ($, on) => {
  files(on, {})
  const { served } = engine(on)
  world(on, { seed: withRun(seededBook(), 'w1', null) })
  await $.session.start(START)

  const refusedRaw = await $.tool.call({ tool: 'mcp__plugin_super-rex_library__get', agentId: 'w1' })
  const viaMod = await $.tool.call({ tool: 'mcp__probe__read_library', agentId: 'w1' })

  expect(refusedRaw.deny).toBe(REFUSAL)
  expect(viaMod.deny).toBeUndefined()
  expect(viaMod.result).toBe('w1: served head, served search, served get')
  expect(served).toEqual(['plugin:super-rex:library head', 'plugin:super-rex:library search', 'plugin:super-rex:library get'])
})

test('when the guard cannot read the book, it refuses the raw library tool and still names rex_skill', async ($, on) => {
  files(on, {})
  const { ran } = engine(on)
  const session = world(on, { seed: { ...emptyBook(), session: SESSION } })
  await $.session.start(START)

  session.failNextBookRead()
  const refused = await $.tool.call({ tool: 'mcp__plugin_super-rex_library__search', query: 'tests' })

  expect(refused.deny).toBe(`rex library guard: the check of mcp__plugin_super-rex_library__search failed (HooksError: super-rex: $.state.get: the book could not be read), so the call is refused. ${REFUSAL.replace('rex library guard: a', 'A')}`)
  expect(ran).toEqual([])
})

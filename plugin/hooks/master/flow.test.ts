// Proves how a tree moves and ends: rex_report closes it only with nothing open, a master that stops with nothing open is woken and its tree closed after two wakes, a wake of the mod reaches the master, and the main loop does not answer the turns of the mod's own agents.
import { expect, test } from 'claude-code/testing'
import type { On, PromptSubmitInput } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import { ROOT, spawnInput } from '../book/kit'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, MASTER, masterWorld, POLICY_KEY, resultText, rex, settleLong, settleUntil, START, TREE, treeBook } from './master-kit'
import type { MasterOptions } from './master-kit'

const IDLE_TEXT =
  'rex: your turn ended with no open job and no waiting question. IF the goal of the charter is met, or cannot be met, THEN call rex_report with the final report. ELSE start the next step with rex_build, or park a question with rex_ask_owner.'
const REPORT = `${ROOT}/.rex/plans/${TREE}/final-report.md`
const OFFER = { agent: 'super-rex:master', description: 'master', source: 'plugin', provider: { plugin: 'super-rex', tier: 'user' } } as const

async function liveTree($: Engine, on: On, options: MasterOptions = {}) {
  const policy = await confirmedPolicy()
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')), ...options })
  await $.session.start(START)
  return session
}

function masterEnds($: Engine, answer: string): Promise<unknown> {
  return $.turn.complete({ agentId: MASTER, reason: 'answer', answer, durationMs: 1, isAborted: false, turnId: `turn-${answer}` })
}

test('rex_report refuses while a job of the tree is open, and from anyone but the master', async ($, on) => {
  const session = await liveTree($, on)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, agentId: MASTER, request: 'Fix the parser', consequence: 'low' })
  const job = lineOf(built, 'job') ?? ''

  const open = await $.tool.call({ tool: 'mcp__super-rex__rex_report', agentId: MASTER, report: 'Done.' })
  const main = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Done.' })
  const builder = await $.tool.call({ tool: 'mcp__super-rex__rex_report', agentId: lineOf(built, 'builder') ?? '', report: 'Done.' })

  expect(open.deny).toBe(`rex_report: job ${job} is still open, got status created; wait for it, or tell its builder what to do with rex_continue`)
  expect(main.deny).toBe('rex_report: only the master may call it, got main')
  expect(builder.deny).toBe('rex_report: only the master may call it, got builder')
  expect(session.book().trees[0]?.state).toBe('running')
  expect(session.written.filter(file => file.path === REPORT)).toEqual([])
})

test('rex_report refuses while a question of the tree waits, then closes the tree once it is answered', async ($, on) => {
  const session = await liveTree($, on)
  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, question: 'Ship it?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })
  const id = lineOf(asked, 'question') ?? ''

  const waiting = await $.tool.call({ tool: 'mcp__super-rex__rex_report', agentId: MASTER, report: 'Done.' })
  await $.command.run(rex(`answer ${id} 1`))
  const closed = await $.tool.call({ tool: 'mcp__super-rex__rex_report', agentId: MASTER, report: 'Result: nothing to build.' })

  expect(waiting.deny).toBe(`rex_report: question ${id} still waits for the person, got no answer; end your turn, and report after the answer`)
  expect(resultText(closed)).toBe(`tree=${TREE}\nstate=done\nreport=${REPORT}\n`)
  expect(session.written.find(file => file.path === REPORT)?.text).toContain('## Jobs\n\n- no job was built\n')
  expect([session.book().trees[0]?.state, session.book().trees[0]?.reportFile]).toEqual(['done', REPORT])
})

test('a master that stops with nothing open is woken twice with the exact text, then the mod closes its tree with its last message', { timeoutMs: 20_000 }, async ($, on) => {
  const session = await liveTree($, on)
  const toMaster = (): readonly string[] => session.sent.filter(sent => sent.to === MASTER).map(sent => sent.text)

  await masterEnds($, 'first stop')
  await settleUntil(session.clock, () => toMaster().length === 1)
  await masterEnds($, 'second stop')
  await settleUntil(session.clock, () => toMaster().length === 2)
  const stateAfterTwo = session.book().trees[0]?.state
  await masterEnds($, 'The parser needs no change.')
  await settleLong(session.clock)

  expect(toMaster()).toEqual([IDLE_TEXT, IDLE_TEXT])
  expect(stateAfterTwo).toBe('running')
  expect([session.book().trees[0]?.state, session.book().trees[0]?.reportFile]).toEqual(['done', REPORT])
  const report = session.written.find(file => file.path === REPORT)?.text ?? ''
  expect(report).toContain("## The master's report\n\nThe mod closed this tree because the master stopped twice with nothing open. The master's last message follows.\n\nThe parser needs no change.\n")
})

test('a master that stops while a job is open, or while a wake of the mod is on the way, is not woken idle', async ($, on) => {
  const session = await liveTree($, on)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, agentId: MASTER, request: 'Fix the parser', consequence: 'low' })

  await masterEnds($, 'builder runs')
  await $.tool.call({ tool: 'mcp__super-rex__rex_continue', agentId: MASTER, job: lineOf(built, 'job') ?? '', message: 'Go on.' })
  await settleLong(session.clock)

  expect(session.sent.filter(sent => sent.to === MASTER)).toEqual([])
  expect(session.book().trees[0]?.state).toBe('running')
})

test('rex_ask_owner sets the count of idle wakes back to 0', { timeoutMs: 20_000 }, async ($, on) => {
  const session = await liveTree($, on)
  const idle = (): number => session.sent.filter(sent => sent.to === MASTER && sent.text === IDLE_TEXT).length

  await masterEnds($, 'one')
  await settleUntil(session.clock, () => idle() === 1)
  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, question: 'Go on?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })
  await $.command.run(rex(`answer ${lineOf(asked, 'question') ?? ''} yes`))
  await settleUntil(session.clock, () => session.sent.length === 2)
  await masterEnds($, 'two')
  await settleUntil(session.clock, () => idle() === 2)
  await masterEnds($, 'three')
  await settleUntil(session.clock, () => idle() === 3)
  const stateAfterThree = session.book().trees[0]?.state
  await masterEnds($, 'four')
  await settleLong(session.clock)

  expect(stateAfterThree).toBe('running')
  expect(idle()).toBe(3)
  expect(session.book().trees[0]?.state).toBe('done')
})

test("rex_status of a master ends with the policy it works under, one policy.<key>= line each", async ($, on) => {
  await liveTree($, on)

  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: MASTER })
  const main = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })

  const keys = ['merge', 'merge_max_consequence', 'max_parallel_builders', 'max_jobs', 'consequence_ceiling', 'tier_cap', 'tree_usd', 'per_job_usd', 'raise_limits']
  expect(keys.map(key => lineOf(status, `policy.${key}`))).toEqual(['ask', 'medium', 'auto', '8', 'high', 'high', '150', 'none', 'ask'])
  expect(resultText(main)).not.toContain('policy.')
})

test('the master type is offered only while a wake of the mod is on the way, and only that wake resumes it', async ($, on) => {
  const session = await liveTree($, on)
  const resume = spawnInput({ subagentType: 'super-rex:master', description: 'resume', resume: { agentId: MASTER } })

  const atRest = (await $.agent.offer(OFFER)).isOffered
  const resumeAtRest = await $.agent.spawn(resume)
  await masterEnds($, 'stop')
  const onTheWay = (await $.agent.offer(OFFER)).isOffered
  const resumeOnTheWay = await $.agent.spawn(resume)
  await settleUntil(session.clock, () => session.sent.length > 0)
  await settleLong(session.clock)
  const after = (await $.agent.offer(OFFER)).isOffered

  expect(atRest).toBe(false)
  expect(resumeAtRest.deny).toBe('rex master: only /rex master starts a master, got a spawn by the main loop; run /rex master <goal>')
  expect(onTheWay).toBe(true)
  expect(resumeOnTheWay.deny).toBeUndefined()
  expect(session.sent.map(sent => sent.to)).toEqual([MASTER])
  expect(after).toBe(false)
})

/** The prompts that reach the session past the plugin; register it before the test first calls the engine. */
function promptsSeen(on: On): PromptSubmitInput[] {
  const received: PromptSubmitInput[] = []
  on('prompt.submit', (_$, e) => {
    received.push(e)
    return { text: e.text }
  })
  return received
}

function notification(agentId: string): PromptSubmitInput {
  return { text: `<task-notification>\n<task-id>${agentId}</task-id>\n<status>completed</status>\n</task-notification>`, origin: { kind: 'task-notification' }, wait: false }
}

test('the main loop gets a line, not a turn, when a master or a tree builder ends a turn, and any other notice passes', async ($, on) => {
  const received = promptsSeen(on)
  await liveTree($, on)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, agentId: MASTER, request: 'Fix the parser', consequence: 'low' })
  const job = lineOf(built, 'job') ?? ''

  const ofMaster = await $.prompt.submit(notification(MASTER))
  const ofBuilder = await $.prompt.submit(notification(lineOf(built, 'builder') ?? ''))
  const ofOther = await $.prompt.submit(notification('x9'))

  expect(ofMaster).toEqual({ drop: `rex: the master of tree ${TREE} ended its turn (tree ${TREE} runs; /rex watch shows it)` })
  expect(ofBuilder).toEqual({ drop: `rex: the builder of job ${job} ended its turn (tree ${TREE} runs; /rex watch shows it)` })
  expect(ofOther.drop).toBeUndefined()
  expect(received.map(e => e.text)).toEqual([notification('x9').text])
})

test("the master's notice after its tree closed reaches the main loop rewritten to read the final report", async ($, on) => {
  const received = promptsSeen(on)
  await liveTree($, on)
  await $.tool.call({ tool: 'mcp__super-rex__rex_report', agentId: MASTER, report: 'Result: nothing to build.' })

  await $.prompt.submit(notification(MASTER))

  expect(received.map(e => [e.text, e.origin.kind])).toEqual([
    [`rex: master tree ${TREE} is closed (done). Its final report is ${REPORT}. Read it and tell the person the result in a few lines.`, 'task-notification'],
  ])
})

// Proves how a tree moves and ends: rex_report closes it only with nothing open and only for the master, a question of a job that closed holds it open no more, a master's turn end never wakes or closes it, a wake of the mod reaches the master as a prompt of the plugin that carries no authority, the master's own steps go to the tree's ledger, and the main loop gets no turn for a tree builder's notice.
import { expect, test } from 'claude-code/testing'
import type { On, PromptSubmitInput } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import { ROOT } from '../book/kit'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, MAIN_JOB, mainJobBook, masterPrompt, masterWorld, POLICY_KEY, ran, resultText, rex, settleLong, settleUntil, START, TREE, treeBook } from './master-kit'
import type { MasterOptions } from './master-kit'

const REPORT = `${ROOT}/.rex/plans/${TREE}/final-report.md`
const PLUGIN = { kind: 'plugin', name: 'super-rex' } as const

async function liveTree($: Engine, on: On, options: MasterOptions = {}) {
  const policy = await confirmedPolicy()
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')), ...options })
  await $.session.start(START)
  return session
}

/** The main loop's turn ends: the master, while it runs a tree. */
function mainEnds($: Engine, answer: string): Promise<unknown> {
  return $.turn.complete({ reason: 'answer', answer, durationMs: 1, isAborted: false, turnId: `turn-${answer}` })
}

/** One model request of the main loop. */
async function mainStep($: Engine, turn: string): Promise<void> {
  const stream = $.turn.step({ turnId: turn, index: 0, model: 'claude-sonnet-5-5', effort: 'high', messageCount: 1 })
  for await (const chunk of stream) {
    expect(chunk).toBeDefined()
  }
  await stream.result
}

test('rex_report refuses while a job of the tree is open, and from a builder or an agent', async ($, on) => {
  const session = await liveTree($, on)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })
  const job = lineOf(built, 'job') ?? ''

  const open = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Done.' })
  const builder = await $.tool.call({ tool: 'mcp__super-rex__rex_report', agentId: lineOf(built, 'builder') ?? '', report: 'Done.' })
  const agent = await $.tool.call({ tool: 'mcp__super-rex__rex_report', agentId: 'x9', report: 'Done.' })

  expect(open.deny).toBe(`rex_report: job ${job} is still open, got status created; wait for it, or tell its builder what to do with rex_continue`)
  expect(builder.deny).toBe('rex_report: only the master may call it, got builder')
  expect(agent.deny).toBe('rex_report: only the master may call it, got agent')
  expect(session.book().trees[0]?.state).toBe('running')
  expect(session.written.filter(file => file.path === REPORT)).toEqual([])
})

test('rex_report refuses while a question of the tree waits, then closes the tree once it is answered, and the main loop is no master after', async ($, on) => {
  const session = await liveTree($, on)
  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', question: 'Ship it?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })
  const id = lineOf(asked, 'question') ?? ''

  const waiting = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Done.' })
  await $.command.run(rex(`answer ${id} 1`))
  const closed = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Result: nothing to build.' })
  const again = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Again.' })

  expect(waiting.deny).toBe(`rex_report: question ${id} still waits for the person, got no answer; end your turn, and report after the answer`)
  expect(resultText(closed)).toBe(`tree=${TREE}\nstate=done\nreport=${REPORT}\n`)
  expect(session.written.find(file => file.path === REPORT)?.text).toContain('## Jobs\n\n- no job was built\n')
  expect([session.book().trees[0]?.state, session.book().trees[0]?.reportFile]).toEqual(['done', REPORT])
  expect(again.deny).toBe('rex_report: only the master may call it, got main')
})

test("the master's turn that ends with nothing open is not woken, and its tree stays open until rex_report", { timeoutMs: 20_000 }, async ($, on) => {
  const session = await liveTree($, on)

  await mainEnds($, 'first stop')
  await mainEnds($, 'second stop')
  await mainEnds($, 'third stop')
  await settleLong(session.clock)

  expect(session.woken()).toEqual([])
  expect(session.sent).toEqual([])
  expect([session.book().trees[0]?.state, session.book().trees[0]?.reportFile]).toEqual(['running', null])
  expect(session.written).toEqual([])
})

test('rex_status of the master ends with the policy it works under, one policy.<key>= line each, and the main loop with no tree reads none', async ($, on) => {
  await liveTree($, on)

  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })
  await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Result: nothing to build.' })
  const main = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })

  const keys = ['merge', 'merge_max_consequence', 'max_parallel_builders', 'max_jobs', 'consequence_ceiling', 'tier_cap', 'per_job_usd', 'raise_limits']
  expect(keys.map(key => lineOf(status, `policy.${key}`))).toEqual(['ask', 'medium', 'auto', '8', 'high', 'high', 'none', 'ask'])
  expect(lineOf(status, 'policy.tree_usd')).toBeUndefined()
  expect(resultText(main)).not.toContain('policy.')
})

test("the main loop's own model steps go to the ledger of the tree it runs, and to no job", async ($, on) => {
  const session = await liveTree($, on)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })

  await mainStep($, 'turn-1')
  await mainStep($, 'turn-2')
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })

  const book = session.book()
  expect(book.trees[0]?.ledgerNano).toBe(4 * 1e9)
  expect(book.jobs.find(job => job.id === lineOf(built, 'job'))?.spentNano).toBe(0)
  expect(lineOf(status, 'tree_spent_usd')).toBe('4.00')
})

test("the main loop's steps go to no tree once its tree is closed", async ($, on) => {
  const policy = await confirmedPolicy()
  const open = await treeBook(policy)
  const session = masterWorld(on, { seed: { ...open, trees: open.trees.map(tree => ({ ...tree, state: 'done' as const })) }, store: { [POLICY_KEY]: policy } })
  await $.session.start(START)

  await mainStep($, 'turn-1')
  await $.command.run(rex('status'))

  expect(session.book().trees[0]?.ledgerNano).toBe(0)
})

test('a wake of the master enters as a prompt of the plugin, framed as its message, and no message is sent to any agent', async ($, on) => {
  const session = await liveTree($, on)
  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', question: 'Ship it?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })
  const id = lineOf(asked, 'question') ?? ''

  await $.command.run(rex(`answer ${id} Yes`))
  await settleUntil(session.clock, () => session.woken().length > 0)
  await settleLong(session.clock)

  const wakes = session.prompts.filter(e => e.origin.kind === 'plugin')
  expect(wakes.map(e => e.origin)).toEqual([PLUGIN])
  expect(wakes[0]?.text).toMatch(new RegExp(`^rex: question ${id} is answered: Yes\\. The tree now:\\n`))
  expect(wakes[0]?.command).toBeUndefined()
  expect(session.sent).toEqual([])
  expect(session.book().treeStartPrompt).toBeNull()
})

test("a prompt of the plugin records no merge answer, no limit, no job start and no tree start, where the person's own words do", async ($, on) => {
  const session = masterWorld(on, { seed: mainJobBook(), store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)
  const words = 'merge: yes. Set the spend limit of this job to 3 dollars'

  await $.prompt.submit({ text: `rex: ${words}`, origin: PLUGIN, wait: false })
  await $.prompt.submit({ ...masterPrompt('Ship the parser'), origin: PLUGIN })
  await $.prompt.submit({ text: '/super-rex:orchestration-builder Fix it', command: { name: 'super-rex:orchestration-builder', args: 'Fix it' }, origin: PLUGIN, wait: false })
  await $.command.run(rex('status'))
  const job = session.book().jobs.find(each => each.id === MAIN_JOB)
  const afterPlugin = [job?.merge, job?.limit, session.book().jobStartPrompt, session.book().treeStartPrompt]
  await $.prompt.submit({ text: words, origin: { kind: 'composer' }, wait: false })

  expect(afterPlugin).toEqual([null, { nano: 10_000_000_000, source: 'default' }, null, null])
  const byPerson = session.book().jobs.find(each => each.id === MAIN_JOB)
  expect([byPerson?.merge?.answer, byPerson?.merge?.source, byPerson?.limit]).toEqual(['yes', 'person-prompt', { nano: 3_000_000_000, source: 'set' }])
  expect(session.book().trees).toEqual([])
})

function notification(agentId: string): PromptSubmitInput {
  return { text: `<task-notification>\n<task-id>${agentId}</task-id>\n<status>completed</status>\n</task-notification>`, origin: { kind: 'task-notification' }, wait: false }
}

test('the main loop gets a line, not a turn, when a builder of its live tree ends a turn, and any other notice passes', async ($, on) => {
  const session = await liveTree($, on)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })
  const job = lineOf(built, 'job') ?? ''

  const ofBuilder = await $.prompt.submit(notification(lineOf(built, 'builder') ?? ''))
  const ofOther = await $.prompt.submit(notification('x9'))

  expect(ofBuilder).toEqual({ drop: `rex: the builder of job ${job} ended its turn (tree ${TREE} runs; /rex watch shows it)` })
  expect(ofOther.drop).toBeUndefined()
  expect(session.prompts.map(e => e.text)).toEqual([notification('x9').text])
})

test('the notice of a builder of a closed tree passes as it came', async ($, on) => {
  const policy = await confirmedPolicy()
  const open = await treeBook(policy)
  const closed = { ...open, trees: open.trees.map(tree => ({ ...tree, state: 'done' as const, builders: [{ job: '0gaq820002', agentId: 'b1' }] })) }
  const session = masterWorld(on, { seed: closed, store: { [POLICY_KEY]: policy } })
  await $.session.start(START)

  const answered = await $.prompt.submit(notification('b1'))

  expect(answered.drop).toBeUndefined()
  expect(session.prompts.map(e => [e.text, e.origin.kind])).toEqual([[notification('b1').text, 'task-notification']])
})

test('a question of a job that closed holds rex_report no more: the mod answers it Not needed and the tree closes', async ($, on) => {
  const session = await liveTree($, on, { processAnswer: byRule([{ starts: ['rex', 'job', 'show'], answer: ran('{}\n') }, ...buildRules('model-high-effort-high')]) })
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })
  const job = lineOf(built, 'job') ?? ''
  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', job, question: 'Keep the old errors?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })
  const id = lineOf(asked, 'question') ?? ''
  await $.tool.call({ tool: 'mcp__super-rex__rex_job_close', agentId: lineOf(built, 'builder') ?? '', status: 'blocked', outcome: 'needs a person', deliveries: [] })

  const closed = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Result: the parser is blocked.' })

  expect(resultText(closed)).toBe(`tree=${TREE}\nstate=done\nreport=${REPORT}\n`)
  expect(session.book().questions.map(question => [question.id, question.answer?.option, question.answer?.source])).toEqual([[id, 'Not needed', 'mod']])
})

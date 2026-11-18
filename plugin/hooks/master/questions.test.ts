// Proves the master's other tools: a question parked at once with a status line and its answer read back by number or text, a message that wakes one builder and never reaches a closed job or a lost builder, a job the master cancels, and a recorded decision that only the master, the main loop of the tree's session, may record.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import { openJobIn } from '../book/jobs'
import { ROOT, spawnInput, START_MS } from '../book/kit'
import { AUTHOR } from '../pipeline/assembly.fixture'
import type { Policy } from './policy'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, masterWorld, POLICY_KEY, ran, resultText, rex, settleUntil, START, TREE, treeBook } from './master-kit'

async function treeWithJob($: Engine, on: On) {
  const policy = await confirmedPolicy()
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, request: 'Fix the parser', consequence: 'low' })
  return { session, job: lineOf(built, 'job') ?? '', builder: lineOf(built, 'builder') ?? '' }
}

test('question_returns_id_at_once_and_answer_reads_back', async ($, on) => {
  const { session, job } = await treeWithJob($, on)

  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', question: 'Merge the parser now?', options: ['Merge now', 'Wait'], recommendation: 'Wait', category: 'merge', job })
  const id = lineOf(asked, 'question') ?? ''
  const before = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })
  const answered = await $.command.run(rex(`answer ${id} Wait`))
  const after = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })
  const ofJob = await $.tool.call({ tool: 'mcp__super-rex__rex_status', job })

  expect(resultText(asked)).toBe(`question=${id}\nstate=parked\n`)
  expect(session.book().questions[0]?.id).toBe(id)
  expect(lineOf(before, `question.${id}`)).toBe('parked')
  expect(answered.text).toBe(`question=${id}\nanswer=Wait\n`)
  expect(session.book().questions[0]?.answer?.source).toBe('command')
  expect([lineOf(after, `question.${id}`), lineOf(after, `answer.${id}`)]).toEqual(['answered', 'Wait'])
  expect([lineOf(ofJob, 'job'), lineOf(ofJob, `answer.${id}`), lineOf(ofJob, 'tree')]).toEqual([job, 'Wait', TREE])
})

test('rex_ask_owner refuses options and a recommendation that do not fit', async ($, on) => {
  const { session } = await treeWithJob($, on)

  const one = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', question: 'Go?', options: ['Yes'], recommendation: 'Yes', category: 'scope' })
  const off = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', question: 'Go?', options: ['Yes', 'No'], recommendation: 'Maybe', category: 'scope' })
  const builder = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: 'b1', question: 'Go?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })

  expect(one.deny).toBe('rex_ask_owner: options must be 2 to 4 texts, got ["Yes"]; call it again with 2 to 4 options')
  expect(off.deny).toBe('rex_ask_owner: recommendation must be one of the options, got Maybe; call it again with one of Yes, No')
  expect(builder.deny).toBe('rex_ask_owner: only the master may call it, got builder')
  expect(session.book().questions).toEqual([])
})

test('rex_note refuses a builder and an agent of its job with where their own decisions go', async ($, on) => {
  const { session, builder } = await treeWithJob($, on)
  const lead = await $.agent.spawn(spawnInput({ description: 'lead', parentAgentId: builder, prompt: AUTHOR, tool_use_id: 'toolu_lead' }))

  const note = { tool: 'mcp__super-rex__rex_note', text: 'Lexer first.' } as const
  const ofBuilder = await $.tool.call({ ...note, agentId: builder })
  const ofAgent = await $.tool.call({ ...note, agentId: lead.agentId ?? '' })

  expect(ofBuilder.deny).toBe('rex_note: only the master may call it, got builder; write the decision in your report file instead')
  expect(ofAgent.deny).toBe('rex_note: only the master may call it, got agent; write the decision in your report file instead')
  expect(session.book().trees[0]?.notes).toEqual([])
})

test('rex_continue_wakes_builder_of_job', async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)

  const sent = await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job, message: 'The person chose Wait; keep the branch and close the job.' })
  const wrong = await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job: '0gb0000000', message: 'x' })
  await settleUntil(session.clock, () => session.sent.length > 0)

  expect(resultText(sent)).toBe(`sent=yes\njob=${job}\n`)
  expect(wrong.deny).toBe(`rex_continue: job 0gb0000000 is not a job of your tree ${TREE}, got 0gb0000000; call it with one of ${job}`)
  expect(session.sent.map(each => [each.to, each.text])).toEqual([[builder, 'The person chose Wait; keep the branch and close the job.']])
})

test('rex_note_recorded_and_shown_in_status', async ($, on) => {
  const { session, job } = await treeWithJob($, on)

  const first = await $.tool.call({ tool: 'mcp__super-rex__rex_note', text: 'Split the parser step in two: lexer first; rejected one big job.' })
  const second = await $.tool.call({ tool: 'mcp__super-rex__rex_note', text: 'Lexer job waits on review.', job })
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status' })

  expect([resultText(first), resultText(second)]).toEqual(['noted=1\n', 'noted=2\n'])
  expect(lineOf(status, 'notes')).toBe('2')
  expect(lineOf(status, 'note.1')).toBe('Split the parser step in two: lexer first; rejected one big job.')
  expect(lineOf(status, 'note.2')).toBe(`${job}: Lexer job waits on review.`)
  expect([lineOf(status, 'tree'), lineOf(status, `job.${job}.status`), lineOf(status, `job.${job}.builder`)]).toEqual([TREE, 'created', 'b1'])
  expect(session.book().trees[0]?.notes.map(note => note.job)).toEqual([null, job])
})

const LONG = 'Should the parser keep the old error texts for every rule, or may each rule word its own error text now?'

test('a parked question shows a status line at once that names it, its text cut at 80 characters, and the command that answers it', async ($, on) => {
  const { session } = await treeWithJob($, on)

  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', question: LONG, options: ['Keep them', 'Word each', 'Ask later'], recommendation: 'Keep them', category: 'scope' })

  const id = lineOf(asked, 'question') ?? ''
  expect(session.statuses).toContain(`rex: question ${id} waits: ${LONG.slice(0, 80)}… Answer with /rex answer ${id} <1-3>`)
})

test('a dismissed dialog leaves the plain waiting line, and each answer moves the line to the oldest question that still waits, then clears it', async ($, on) => {
  on('tool.call', { tool: 'AskUserQuestion' }, () => ({ deny: "The user doesn't want to proceed with this tool use." }))
  const { session } = await treeWithJob($, on)
  const ask = async (question: string) => lineOf(await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', question, options: ['Merge now', 'Wait'], recommendation: 'Wait', category: 'scope' }), 'question') ?? ''

  const first = await ask('First?')
  await session.clock.advance(10)
  const second = await ask('Second?')
  await session.clock.advance(10)
  const dismissed = session.statusLine()
  await $.command.run(rex(`answer ${first} 1`))
  const afterFirst = session.statusLine()
  await $.command.run(rex(`answer ${second} 2`))

  expect(dismissed).toBe(`rex: question ${second} waits: Second? Answer with /rex answer ${second} <1-2>`)
  expect(afterFirst).toBe(`rex: question ${second} waits: Second? Answer with /rex answer ${second} <1-2>`)
  expect(session.statusLine()).toBeUndefined()
})

test('a dialog that fails for another cause than a dismiss keeps its error in the waiting line', async ($, on) => {
  on('tool.call', { tool: 'AskUserQuestion' }, () => {
    throw new Error('the dialog could not be drawn')
  })
  const { session } = await treeWithJob($, on)

  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', question: 'Go?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })
  await session.clock.advance(10)

  const id = lineOf(asked, 'question') ?? ''
  expect(session.statusLine()).toMatch(new RegExp(`^rex: question ${id} waits: Go\\? Answer with /rex answer ${id} <1-2> \\(the dialog failed: .+\\)$`))
})

test('/rex answer takes the option number or its text in any case, and needs no id when one question waits', async ($, on) => {
  const { session } = await treeWithJob($, on)
  const ask = (question: string) => $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', question, options: ['Merge now', 'Wait'], recommendation: 'Wait', category: 'scope' })

  const first = lineOf(await ask('First?'), 'question') ?? ''
  const second = lineOf(await ask('Second?'), 'question') ?? ''
  const noId = await $.command.run(rex('answer 2'))
  const byNumber = await $.command.run(rex(`answer ${first} 2`))
  const outOfRange = await $.command.run(rex(`answer ${second} 3`))
  const alone = await $.command.run(rex('answer MERGE NOW'))

  expect(noId.text).toBe(`rex answer: name a question and one of its options, got 2; run /rex answer <id> <option number>, with ${first}: 1 (Merge now), 2 (Wait); ${second}: 1 (Merge now), 2 (Wait)`)
  expect(byNumber.text).toBe(`question=${first}\nanswer=Wait\n`)
  expect(outOfRange.text).toBe(`rex answer: 3 is not an option of ${second}, got 3; one of 1 (Merge now), 2 (Wait)`)
  expect(alone.text).toBe(`question=${second}\nanswer=Merge now\n`)
  expect(session.book().questions.map(question => question.answer?.option)).toEqual(['Wait', 'Merge now'])
})

test('rex_cancel closes an open job of the tree cancelled and tells its builder to stop; it refuses a closed job, a job of no tree of the caller and a caller that is not the master, and rex_report then closes the tree', async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)

  const fromBuilder = await $.tool.call({ tool: 'mcp__super-rex__rex_cancel', agentId: builder, job, reason: 'x' })
  const cancelled = await $.tool.call({ tool: 'mcp__super-rex__rex_cancel', job, reason: 'the person stopped the tree' })
  const again = await $.tool.call({ tool: 'mcp__super-rex__rex_cancel', job, reason: 'again' })
  const wrong = await $.tool.call({ tool: 'mcp__super-rex__rex_cancel', job: '0gb0000000', reason: 'x' })
  await settleUntil(session.clock, () => session.sent.length > 0)
  const reported = await $.tool.call({ tool: 'mcp__super-rex__rex_report', report: 'Result: the person stopped the tree.' })

  expect(fromBuilder.deny).toBe('rex_cancel: only the master may call it, got builder')
  expect(resultText(cancelled)).toBe(`job=${job}\nstatus=cancelled\n`)
  expect(session.book().jobs.map(each => [each.status, each.outcome])).toEqual([['cancelled', 'cancelled by the master: the person stopped the tree']])
  expect(again.deny).toBe(`rex_cancel: job ${job} is closed already, got status cancelled; nothing to cancel`)
  expect(wrong.deny).toBe(`rex_cancel: job 0gb0000000 is not a job of your tree ${TREE}, got 0gb0000000; call it with one of ${job}`)
  expect(session.sent.map(each => [each.to, each.text])).toEqual([[builder, `rex: the master cancelled job ${job}: the person stopped the tree. The job is closed. Stop work, start no agent, and end your turn.`]])
  expect(session.book().trees[0]?.notes.map(note => [note.job, note.text])).toEqual([[job, 'rex: the master cancelled this job: the person stopped the tree']])
  expect(lineOf(reported, 'state')).toBe('done')
})

test('rex_continue refuses a closed job, and sends it nothing', async ($, on) => {
  const { session, job } = await treeWithJob($, on)
  await $.tool.call({ tool: 'mcp__super-rex__rex_cancel', job, reason: 'the step is left out' })
  await settleUntil(session.clock, () => session.sent.length > 0)

  const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job, message: 'Go on.' })

  expect(refused.deny).toBe(`rex_continue: job ${job} is closed, got status cancelled; a closed job reads no message: start a fix job with rex_build, or mark its step not built`)
  expect(session.sent.filter(each => each.text === 'Go on.')).toEqual([])
})

const LOST_JOB = '0gaq7z0001'

/** A live tree whose open job LOST_JOB has no builder in the tree, as after a resume whose question nobody answered yet. */
async function treeWithLostBuilder($: Engine, on: On, keys: Partial<Policy>) {
  const policy = await confirmedPolicy(keys)
  const input = {
    builder: 'b7',
    tree: TREE,
    origin: 'master',
    request: 'Fix the parser',
    repo: ROOT,
    base: 'main',
    type: 'fix',
    size: 'small',
    consequence: 'low',
    langs: ['rust'],
    prefs: { duration: null, budget: null, autonomy: null },
    limit: { nano: 10_000_000_000, source: 'default' },
    dependsOn: [],
    mergeAfter: [],
  } as const
  const seed = openJobIn(await treeBook(policy), input, LOST_JOB, START_MS).book
  const listed = `worktree ${ROOT}\nHEAD abc\n\nworktree ${ROOT}.worktrees/${LOST_JOB}\nHEAD abc\nbranch refs/heads/rex/${LOST_JOB}\n`
  const rules = [{ starts: ['git', '-C', ROOT, 'worktree', 'list', '--porcelain'], answer: ran(listed) }, ...buildRules('model-high-effort-high')]
  const session = masterWorld(on, { seed, store: { [POLICY_KEY]: policy }, processAnswer: byRule(rules) })
  await $.session.start(START)
  return session
}

test('rex_continue to an open job with no builder starts one under on_dead_builder continue, and its refusal says so', async ($, on) => {
  const session = await treeWithLostBuilder($, on, { on_dead_builder: 'continue' })

  const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job: LOST_JOB, message: 'Go on.' })

  expect(refused.deny).toBe(`rex_continue: job ${LOST_JOB} had no builder, got none; the mod started a new one that goes on from the job's record: call rex_continue again to send your message`)
  expect(session.spawned.map(spawn => [spawn.description, spawn.cwd])).toEqual([[`rex builder ${LOST_JOB}`, `${ROOT}.worktrees/${LOST_JOB}`]])
  expect(session.book().trees[0]?.builders).toEqual([{ job: LOST_JOB, agentId: 'b1' }])
  expect(session.book().questions).toEqual([])
})

test('rex_continue to an open job with no builder parks the question under on_dead_builder ask, and its refusal names it', async ($, on) => {
  const session = await treeWithLostBuilder($, on, { on_dead_builder: 'ask' })

  const refused = await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job: LOST_JOB, message: 'Go on.' })
  const again = await $.tool.call({ tool: 'mcp__super-rex__rex_continue', job: LOST_JOB, message: 'Go on.' })

  const asked = session.book().questions
  expect(asked.map(question => [question.job, question.question, question.options])).toEqual([
    [LOST_JOB, `The builder of job ${LOST_JOB} ended with the last session. Start a new builder for it?`, ['Start a new builder', 'Leave it blocked']],
  ])
  expect(refused.deny).toBe(`rex_continue: job ${LOST_JOB} has no builder, got none; question ${asked[0]?.id ?? ''} asks the person whether to start one: tell them, and end your turn`)
  expect(again.deny).toBe(refused.deny)
  expect(session.spawned).toEqual([])
})

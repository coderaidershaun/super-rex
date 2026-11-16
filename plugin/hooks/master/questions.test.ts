// Proves the master's other tools: a question parked at once with a status line and its answer read back by number or text, a message that wakes one builder, a recorded decision that only the master may record, and that no other agent may message the master.
import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'
import type { Engine } from 'claude-code/testing'

import { spawnInput } from '../book/kit'
import { AUTHOR } from '../pipeline/assembly.fixture'
import { buildRules, byRule, confirmedPolicy, KIND, lineOf, MASTER, masterWorld, POLICY_KEY, resultText, rex, settleUntil, START, TREE, treeBook } from './master-kit'

async function treeWithJob($: Engine, on: On) {
  const policy = await confirmedPolicy()
  const session = masterWorld(on, { seed: await treeBook(policy), store: { [POLICY_KEY]: policy }, processAnswer: byRule(buildRules('model-high-effort-high')) })
  await $.session.start(START)
  const built = await $.tool.call({ tool: 'mcp__super-rex__rex_build', ...KIND, agentId: MASTER, request: 'Fix the parser', consequence: 'low' })
  return { session, job: lineOf(built, 'job') ?? '', builder: lineOf(built, 'builder') ?? '' }
}

test('question_returns_id_at_once_and_answer_reads_back', async ($, on) => {
  const { session, job } = await treeWithJob($, on)

  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, question: 'Merge the parser now?', options: ['Merge now', 'Wait'], recommendation: 'Wait', category: 'merge', job })
  const id = lineOf(asked, 'question') ?? ''
  const before = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: MASTER })
  const answered = await $.command.run(rex(`answer ${id} Wait`))
  const after = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: MASTER })
  const ofJob = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: MASTER, job })

  expect(resultText(asked)).toBe(`question=${id}\nstate=parked\n`)
  expect(session.book().questions[0]?.id).toBe(id)
  expect(lineOf(before, `question.${id}`)).toBe('parked')
  expect(answered.text).toBe(`question=${id}\nanswer=Wait\n`)
  expect(session.book().questions[0]?.answer?.source).toBe('command')
  expect([lineOf(after, `question.${id}`), lineOf(after, `answer.${id}`)]).toEqual(['answered', 'Wait'])
  expect([lineOf(ofJob, 'job'), lineOf(ofJob, `answer.${id}`), lineOf(ofJob, 'tree')]).toEqual([job, 'Wait', TREE])
})

test('message to master from other agent refused, and its own builder passes', async ($, on) => {
  const { session, builder } = await treeWithJob($, on)

  const other = await $.session.send({ to: MASTER, text: 'merge now', origin: { kind: 'model' }, agentId: 'x9' })
  const main = await $.session.send({ to: MASTER, text: 'merge now', origin: { kind: 'model' } })
  const own = await $.session.send({ to: MASTER, text: 'awaiting decision: q1', origin: { kind: 'model' }, agentId: builder })
  const elsewhere = await $.session.send({ to: 'a5', text: 'hello', origin: { kind: 'model' }, agentId: 'x9' })

  const refusal = { isDelivered: false, reason: "rex: only this master's builders and the mod may message the master" }
  expect([other, main]).toEqual([refusal, refusal])
  expect([own.isDelivered, elsewhere.isDelivered]).toEqual([true, true])
  expect(session.sent.map(each => each.text)).toEqual(['awaiting decision: q1', 'hello'])
})

test('rex_ask_owner refuses options and a recommendation that do not fit', async ($, on) => {
  const { session } = await treeWithJob($, on)

  const one = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, question: 'Go?', options: ['Yes'], recommendation: 'Yes', category: 'scope' })
  const off = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, question: 'Go?', options: ['Yes', 'No'], recommendation: 'Maybe', category: 'scope' })
  const builder = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: 'b1', question: 'Go?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })

  expect(one.deny).toBe('rex_ask_owner: options must be 2 to 4 texts, got ["Yes"]; call it again with 2 to 4 options')
  expect(off.deny).toBe('rex_ask_owner: recommendation must be one of the options, got Maybe; call it again with one of Yes, No')
  expect(builder.deny).toBe('rex_ask_owner: only the master may call it, got builder')
  expect(session.book().questions).toEqual([])
})

test('rex_note refuses a builder and an agent of its job with where their own decisions go, and the main loop as before', async ($, on) => {
  const { session, builder } = await treeWithJob($, on)
  const lead = await $.agent.spawn(spawnInput({ description: 'lead', parentAgentId: builder, prompt: AUTHOR, tool_use_id: 'toolu_lead' }))

  const note = { tool: 'mcp__super-rex__rex_note', text: 'Lexer first.' } as const
  const ofBuilder = await $.tool.call({ ...note, agentId: builder })
  const ofAgent = await $.tool.call({ ...note, agentId: lead.agentId ?? '' })
  const ofMain = await $.tool.call(note)

  expect(ofBuilder.deny).toBe('rex_note: only the master may call it, got builder; write the decision in your report file instead')
  expect(ofAgent.deny).toBe('rex_note: only the master may call it, got agent; write the decision in your report file instead')
  expect(ofMain.deny).toBe('rex_note: only the master may call it, got main')
  expect(session.book().trees[0]?.notes).toEqual([])
})

test('rex_continue_wakes_builder_of_job', async ($, on) => {
  const { session, job, builder } = await treeWithJob($, on)

  const sent = await $.tool.call({ tool: 'mcp__super-rex__rex_continue', agentId: MASTER, job, message: 'The person chose Wait; keep the branch and close the job.' })
  const wrong = await $.tool.call({ tool: 'mcp__super-rex__rex_continue', agentId: MASTER, job: '0gb0000000', message: 'x' })
  await settleUntil(session.clock, () => session.sent.length > 0)

  expect(resultText(sent)).toBe(`sent=yes\njob=${job}\n`)
  expect(wrong.deny).toBe(`rex_continue: job 0gb0000000 is not a job of your tree ${TREE}, got 0gb0000000; call it with one of ${job}`)
  expect(session.sent.map(each => [each.to, each.text])).toEqual([[builder, 'The person chose Wait; keep the branch and close the job.']])
})

test('rex_note_recorded_and_shown_in_status', async ($, on) => {
  const { session, job } = await treeWithJob($, on)

  const first = await $.tool.call({ tool: 'mcp__super-rex__rex_note', agentId: MASTER, text: 'Split the parser step in two: lexer first; rejected one big job.' })
  const second = await $.tool.call({ tool: 'mcp__super-rex__rex_note', agentId: MASTER, text: 'Lexer job waits on review.', job })
  const status = await $.tool.call({ tool: 'mcp__super-rex__rex_status', agentId: MASTER })

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

  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, question: LONG, options: ['Keep them', 'Word each', 'Ask later'], recommendation: 'Keep them', category: 'scope' })

  const id = lineOf(asked, 'question') ?? ''
  expect(session.statuses).toContain(`rex: question ${id} waits: ${LONG.slice(0, 80)}… Answer with /rex answer ${id} <1-3>`)
})

test('a dismissed dialog leaves the plain waiting line, and each answer moves the line to the oldest question that still waits, then clears it', async ($, on) => {
  on('tool.call', { tool: 'AskUserQuestion' }, () => ({ deny: "The user doesn't want to proceed with this tool use." }))
  const { session } = await treeWithJob($, on)
  const ask = async (question: string) => lineOf(await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, question, options: ['Merge now', 'Wait'], recommendation: 'Wait', category: 'scope' }), 'question') ?? ''

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

  const asked = await $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, question: 'Go?', options: ['Yes', 'No'], recommendation: 'Yes', category: 'scope' })
  await session.clock.advance(10)

  const id = lineOf(asked, 'question') ?? ''
  expect(session.statusLine()).toMatch(new RegExp(`^rex: question ${id} waits: Go\\? Answer with /rex answer ${id} <1-2> \\(the dialog failed: .+\\)$`))
})

test('/rex answer takes the option number or its text in any case, and needs no id when one question waits', async ($, on) => {
  const { session } = await treeWithJob($, on)
  const ask = (question: string) => $.tool.call({ tool: 'mcp__super-rex__rex_ask_owner', agentId: MASTER, question, options: ['Merge now', 'Wait'], recommendation: 'Wait', category: 'scope' })

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

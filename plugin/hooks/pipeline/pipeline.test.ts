// Proves the spawn pipeline: lint refuses a brief once and then passes it marked, the assembled brief holds each part in order around the author's unchanged text, and other spawns pass as they came.
import { expect, test } from 'claude-code/testing'

import { ROOT, spawnInput, world } from '../book/kit'
import { files, JOB, resultText, seededBook } from '../tools/act-kit'
import { AUTHOR, plannedBook, SKILL_FILES, spawnsKept, TDD_HASH, TDD_TEXT } from './assembly.fixture'
import { SANDBOX_TEXT } from './sandbox'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const STOPS =
  'This run has no person to answer you. Do not stop to ask whether to continue, to report progress, to offer options, or because the task is large. Stop only when the acceptance evidence passes, or when you need a decision or an approval that this brief does not give.'
const SKILLS_BLOCK = [
  '<skills>',
  `<skill name="rex-code-tdd" source="plugin" hash="${TDD_HASH}">`,
  TDD_TEXT,
  '</skill>',
  'Open when (call rex_skill with the name and file):',
  '- rex-code-tdd/mocking.md: a test crosses a process',
  '</skills>',
].join('\n')

function returnLine(file: string): string {
  return `Return: write your full report to ${file}. Your final message has at most 2,000 characters: the path of that file, a summary of at most 5 lines, and one state: done, partial, awaiting decision, or blocked.`
}

test('lint_refuses_once_then_passes_marked', async ($, on) => {
  files(on, SKILL_FILES)
  const started = spawnsKept(on)
  const session = world(on, { seed: plannedBook('model-high-effort-high') })
  await $.session.start(START)
  const brief = AUTHOR.replace('Acceptance: `cargo test parser` passes.\n', '')

  const first = await $.agent.spawn(spawnInput({ description: 'engineer', subagentType: 'super-rex:model-high-effort-high', prompt: brief, tool_use_id: 'toolu_1' }))
  const second = await $.agent.spawn(spawnInput({ description: 'engineer', subagentType: 'super-rex:model-high-effort-high', prompt: brief, tool_use_id: 'toolu_2' }))

  expect(first.deny).toBe('rex brief check (1 of 1): Acceptance: is missing. Fix the brief and start the agent again.')
  expect(second.agentId).toBe('a1')
  expect(started).toHaveLength(1)
  const output = `${ROOT}/.rex/plans/${JOB}/engineer.md`
  expect(started[0]?.prompt).toBe([`Rex-Job: ${JOB}\nRex-Role: engineer`, SKILLS_BLOCK, brief, STOPS, SANDBOX_TEXT, returnLine(output)].join('\n\n'))
  const run = session.book().runs.find(each => each.agentId === 'a1')
  expect([run?.lint, run?.lintFindings, run?.assembled, run?.task, run?.outputFile, run?.skillsAttached]).toEqual(['passed-marked', ['Acceptance: is missing'], 'yes', 'T2', output, ['rex-code-tdd']])
  expect(run?.briefHash).toMatch(/^sha256:[0-9a-f]{64}$/)
})

test('a brief that passes lint is assembled once, and its spawn again by tool use id is not assembled twice', async ($, on) => {
  files(on, SKILL_FILES)
  const started = spawnsKept(on)
  const session = world(on, { seed: plannedBook('model-high-effort-high') })
  await $.session.start(START)

  await $.agent.spawn(spawnInput({ description: 'engineer', subagentType: 'super-rex:model-high-effort-high', prompt: AUTHOR, tool_use_id: 'toolu_7' }))
  await $.agent.spawn(spawnInput({ description: 'engineer', subagentType: 'super-rex:model-high-effort-high', prompt: 'Read the file.', tool_use_id: 'toolu_7' }))

  expect(started[0]?.prompt).toContain(`\n\n${AUTHOR}\n\n${STOPS}\n\n`)
  expect(started[1]?.prompt).toBe('Read the file.')
  expect(session.book().runs.find(each => each.agentId === 'a1')?.lint).toBe('pass')
})

test('lint finds each kind of finding in the author text only', async ($, on) => {
  files(on, SKILL_FILES)
  spawnsKept(on)
  world(on, { seed: plannedBook('model-high-effort-high') })
  await $.session.start(START)
  const brief = ['Task: T9', 'Outcome: think step by step. This is IMPORTANT.', 'Scope: never touch src/.', 'Context: <data>notes', 'Acceptance: it works.', 'Limits: none.'].join('\n')

  const refused = await $.agent.spawn(spawnInput({ description: 'engineer', subagentType: 'super-rex:model-high-effort-high', prompt: brief }))

  expect(refused.deny).toBe(
    'rex brief check (1 of 1): Task: T9 names no task of the plan; Acceptance: names no evidence (a command in backticks, a test, output, an exit code, a file or a log); "think step by step" is a line the effort level controls; IMPORTANT in capitals adds force, not meaning; "never" stands outside Limits:; <data> tags do not pair: 1 opened, 0 closed. Fix the brief and start the agent again.',
  )
})

test('resume_spawn_gets_gate_only', async ($, on) => {
  files(on, SKILL_FILES)
  const started = spawnsKept(on)
  const session = world(on, { seed: plannedBook('model-high-effort-high') })
  await $.session.start(START)
  await $.agent.spawn(spawnInput({ description: 'engineer', subagentType: 'super-rex:model-high-effort-high', prompt: AUTHOR }))

  const resumed = spawnInput({ description: 'engineer', subagentType: 'general-purpose', prompt: 'Go on.', resume: { agentId: 'a1' } })
  await $.agent.spawn(resumed)

  expect(started[1]).toEqual(resumed)
  expect(session.book().runs.find(each => each.agentId === 'a1')?.agentType).toBe('super-rex:model-high-effort-high')
})

test('a teammate, a workflow agent and an agent outside any job pass as they came', async ($, on) => {
  files(on, SKILL_FILES)
  const started = spawnsKept(on)
  world(on, { seed: plannedBook('model-high-effort-high') })
  await $.session.start(START)

  const teammate = spawnInput({ description: 'mate', isTeammate: true, prompt: 'Help.' })
  const workflow = spawnInput({ description: 'flow', workflow: { runId: 'w1', agentIndex: 0 }, prompt: 'Run.' })
  await $.agent.spawn(teammate)
  await $.agent.spawn(workflow)

  expect(started).toEqual([teammate, workflow])
})

test('an agent that started without its parts gets them late from rex_skill with no name', async ($, on) => {
  files(on, SKILL_FILES)
  spawnsKept(on)
  const session = world(on, { seed: plannedBook('model-high-effort-high') })
  await $.session.start(START)
  await $.agent.spawn(spawnInput({ description: 'engineer', subagentType: 'super-rex:model-high-effort-high', prompt: AUTHOR }))

  const late = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', agentId: 'a1' })
  const main = await $.tool.call({ tool: 'mcp__super-rex__rex_skill' })

  expect(resultText(late)).toBe(`assembly=late\n\n${SKILLS_BLOCK}\n\n${STOPS}\n\n${returnLine(`${ROOT}/.rex/plans/${JOB}/engineer.md`)}`)
  expect(main.deny).toBe("rex_skill: with no name it gives a job's agent the brief parts it is missing, got main with no job; call it with a skill name")
  expect(session.book().runs.find(each => each.agentId === 'a1')?.assembled).toBe('late')
})

test('a role that starts before the plan gets the fixed manifest of its Task line, and any other pre-plan agent gets no skills block', async ($, on) => {
  const builderSkill = '# Builder\n\n## Fixed manifests\n\n| Role | Read-first skills |\n| --- | --- |\n| decision-maker | `rex-code-philosophy` |\n'
  files(on, { ...SKILL_FILES, 'plugin:skills/orchestration-builder/SKILL.md': builderSkill, 'plugin:library/rex-code-philosophy/SKILL.md': 'Grug.\n' })
  const started = spawnsKept(on)
  world(on, { seed: seededBook() })
  await $.session.start(START)
  const decide = AUTHOR.replace('Task: T2', 'Task: decision-maker')
  const other = AUTHOR.replace('Task: T2', 'Task: scout')

  await $.agent.spawn(spawnInput({ description: 'decide', subagentType: 'super-rex:model-high-effort-high', prompt: decide }))
  await $.agent.spawn(spawnInput({ description: 'scout', subagentType: 'super-rex:model-high-effort-high', prompt: other }))

  expect(started[0]?.prompt).toMatch(/^Rex-Job: 0gaq820001\nRex-Role: decision-maker\n\n<skills>\n<skill name="rex-code-philosophy" source="plugin" hash="sha256:[0-9a-f]{64}">\nGrug\.\n\n<\/skill>\n<\/skills>\n\nTask: decision-maker\n/)
  expect(started[1]?.prompt).toMatch(/^Rex-Job: 0gaq820001\nRex-Role: none\n\nTask: scout\n/)
})

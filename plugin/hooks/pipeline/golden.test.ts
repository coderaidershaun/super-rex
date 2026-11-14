// The 20 golden briefs, one for each tier and level of the grid, and the fenced blocks of the tier files that they are made from.
import { expect, test } from 'claude-code/testing'

import { ROOT, spawnInput, world } from '../book/kit'
import { files, JOB } from '../tools/act-kit'
import { AUTHOR, plannedBook, SKILL_FILES, spawnsKept, TDD_HASH, TDD_TEXT } from './assembly.fixture'
import { SANDBOX_TEXT } from './sandbox'
import { TIER_FILE_COPIES } from './tier-files.fixture'
import { briefBlocks, LEVELS, TIER_BLOCKS, TIER_FILE, TIERS } from './tiers'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const

/** The blocks as the tier files hold them today, written out, so a change in a file or in the parser shows here. */
const BLOCK_TEXT: { readonly [file: string]: { readonly [label: string]: string } } = {
  'haiku.md': {
    Finish: 'Work until every item of the task is done. A partial result with a list of what is left is not a finished task. Stop before the end only if you need a decision or an approval that this brief does not give.',
    Check: 'Before you report, run a real check of your result: run the test or the command, or read the file that you wrote. Report what you ran and what it showed. If you could not check something, say so.',
  },
  'sonnet.md': {
    Finish: 'Keep working until everything in this brief is done and checked. Do not stop to ask whether to continue. Stop early only for a decision or an approval that this brief does not give.',
    Stop: 'When the work in this brief is done and checked, stop and report. Add nothing that the brief does not ask for: no extra tests, documents, or clean-up. If you see other work that is worth doing, name it in your report.',
    Check: 'Before you report, run a real check of your result: run the test or the command, or read the file that you wrote. Report what you ran and what it showed. If you could not check something, say so.',
    Review: 'Check your work one time. Do not start extra rounds of review, and do not start reviewer sub-agents, unless this brief asks for them.',
  },
  'opus.md': {
    Stops: 'This run has no person to answer you. Do not stop to ask whether to continue, to report progress, to offer options, or because the task is large. Stop only when the acceptance evidence passes, or when you need a decision or an approval that this brief does not give.',
  },
  'fable.md': {
    Finish: 'Do not end your turn while work that this brief asks for is still open. Work through ordinary obstacles such as errors, timeouts, and empty results: find the cause and try another way. Stop before the end only when you need a decision or an approval that this brief does not give, or when something was blocked on purpose. Ask for approval before each action that this brief says needs approval.',
  },
}

const SKILLS_BLOCK = [
  '<skills>',
  `<skill name="rex-code-tdd" source="plugin" hash="${TDD_HASH}">`,
  TDD_TEXT,
  '</skill>',
  'Open when (call rex_skill with the name and file):',
  '- rex-code-tdd/mocking.md: a test crosses a process',
  '</skills>',
].join('\n')

test('each fenced block of the tier files is found under its label', () => {
  for (const tier of TIERS) {
    const file = TIER_FILE[tier]
    const blocks = briefBlocks(TIER_FILE_COPIES[file] ?? '')
    const needed = [...new Set(LEVELS.flatMap(level => TIER_BLOCKS[tier][level]))]
    for (const label of needed) {
      expect([file, label, blocks.get(label)]).toEqual([file, label, BLOCK_TEXT[file]?.[label]])
    }
  }
  expect([...briefBlocks(TIER_FILE_COPIES['sonnet.md'] ?? '').keys()]).toEqual(['Finish', 'Stop', 'Check', 'Review', 'Ideas'])
  expect([...briefBlocks(TIER_FILE_COPIES['opus.md'] ?? '').keys()]).toEqual(['Stops', 'Direct'])
})

for (const tier of TIERS) {
  for (const level of LEVELS) {
    test(`golden_brief_${tier}_${level}`, async ($, on) => {
      const grid = `model-${tier}-effort-${level}`
      files(on, SKILL_FILES)
      const started = spawnsKept(on)
      world(on, { seed: plannedBook(grid) })
      await $.session.start(START)

      await $.agent.spawn(spawnInput({ description: 'engineer', subagentType: `super-rex:${grid}`, prompt: AUTHOR }))

      const blocks = TIER_BLOCKS[tier][level].map(label => BLOCK_TEXT[TIER_FILE[tier]]?.[label] ?? `missing ${label}`)
      const tierPart = blocks.length === 0 ? [] : [blocks.join('\n\n')]
      const returnPart = `Return: write your full report to ${ROOT}/.rex/plans/${JOB}/engineer.md. Your final message has at most 2,000 characters: the path of that file, a summary of at most 5 lines, and one state: done, partial, awaiting decision, or blocked.`
      expect(started[0]?.prompt).toBe([`Rex-Job: ${JOB}\nRex-Role: engineer`, SKILLS_BLOCK, AUTHOR, ...tierPart, SANDBOX_TEXT, returnPart].join('\n\n'))
    })
  }
}

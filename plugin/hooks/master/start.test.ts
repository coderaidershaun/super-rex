// Proves a tree's open: one first policy question in the person's words, a stored policy that still holds the retired key master_model, and a brief with the charter's own reading, the policy, the repo's places and the map of the code.
import { expect, test } from 'claude-code/testing'

import { ROOT } from '../book/kit'
import { READ_CHARTER } from '../prompts/index'
import { byRule, confirmedPolicy, lineOf, masterWorld, openedTree, placesRules, POLICY_KEY, ran, resultText, rex, START } from './master-kit'
import type { Rule } from './master-kit'

const MERGE_QUESTION = 'Merge each job of a master to its base branch when its checks pass?'
const USAGE = { input_tokens: 10, output_tokens: 12, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 }
const READING = 'Goal: ship the parser.\nAcceptance: cargo test passes.\nConstraints: none stated.'

function labelsOf(e: object): readonly string[] {
  const questions: unknown = Reflect.get(e, 'questions')
  const first: unknown = Array.isArray(questions) ? questions[0] : undefined
  const options: unknown = typeof first === 'object' && first !== null ? Reflect.get(first, 'options') : undefined
  return Array.isArray(options) ? options.map((option: unknown) => (typeof option === 'object' && option !== null ? String(Reflect.get(option, 'label')) : String(option))) : []
}

function questionOf(e: object): string {
  const questions: unknown = Reflect.get(e, 'questions')
  const first: unknown = Array.isArray(questions) ? questions[0] : undefined
  return typeof first === 'object' && first !== null ? String(Reflect.get(first, 'question')) : ''
}

const ANSWERS = [
  ['Yes, merge', 'allowed_if_checks_pass'],
  ['No, keep the branches', 'never'],
  ['Ask me for each job', 'ask'],
] as const

for (const [chosen, merge] of ANSWERS) {
  test(`the first tree of a repo asks one policy question, and ${chosen} stores merge ${merge} with max_parallel_builders auto`, async ($, on) => {
    const session = masterWorld(on)
    const asked: { readonly question: string; readonly labels: readonly string[] }[] = []
    on('tool.call', { tool: 'AskUserQuestion' }, (_$, e) => {
      asked.push({ question: questionOf(e), labels: labelsOf(e) })
      return { result: { questions: [], answers: { [questionOf(e)]: chosen } } }
    })
    await $.session.start(START)

    const answered = await openedTree($, 'Ship the parser')
    const policy = await $.command.run(rex('policy show'))

    expect(asked).toEqual([{ question: MERGE_QUESTION, labels: ['Yes, merge', 'No, keep the branches', 'Ask me for each job'] }])
    expect(lineOf(answered, 'merge')).toBe(merge)
    expect(policy.text).toContain(`\nmerge=${merge}\n`)
    expect(policy.text).toContain('\nmax_parallel_builders=auto\n')
    expect(policy.text).toContain('\nconfirmed=yes\n')
    expect(session.book().trees.map(tree => tree.state)).toEqual(['running'])
  })
}

test('a dismissed first policy question opens no tree and stores no policy', async ($, on) => {
  const session = masterWorld(on)
  on('tool.call', { tool: 'AskUserQuestion' }, () => ({ deny: "The user doesn't want to proceed with this tool use." }))
  await $.session.start(START)

  const answered = await openedTree($, 'Ship the parser')
  const policy = await $.command.run(rex('policy show'))

  expect(answered.deny).toBe('rex_tree_open: no confirmed policy for this repo; the person answers the policy question, or runs /rex policy confirm, then /super-rex:orchestration-master again')
  expect(policy.text).toContain('\nconfirmed=no\n')
  expect(session.book().trees).toEqual([])
})

test('a stored policy that still holds master_model reads as confirmed, opens a tree, and shows no master_model', async ($, on) => {
  const stored = { ...(await confirmedPolicy()), master_model: 'opus' }
  const session = masterWorld(on, { store: { [POLICY_KEY]: stored } })
  await $.session.start(START)

  const shown = await $.command.run(rex('policy show'))
  const answered = await openedTree($, 'Ship the parser')

  expect(shown.text).toContain('\nconfirmed=yes\n')
  expect(shown.text).not.toContain('master_model')
  expect(lineOf(answered, 'state')).toBe('running')
  expect(resultText(answered)).not.toContain('master_model')
  expect(session.book().trees.map(tree => tree.state)).toEqual(['running'])
})

test('/rex policy set master_model is refused, and changes nothing', async ($, on) => {
  const policy = await confirmedPolicy()
  masterWorld(on, { store: { [POLICY_KEY]: policy } })
  await $.session.start(START)

  const set = await $.command.run(rex('policy set master_model sonnet'))
  const shown = await $.command.run(rex('policy show'))

  expect(set.text).toBe('rex policy: policy key master_model must be no key of the policy any more: the master runs on the model of your session, got "sonnet"; nothing was changed')
  expect(shown.text).toContain('\nconfirmed=yes\n')
  expect(shown.text).not.toContain('master_model')
})

/** The lines `rex codebase` prints in a test: one for each file of a repo that size. */
function codebaseRules(files: number): readonly Rule[] {
  const lines = Array.from({ length: files }, (_, index) => `src/file${String(index + 1)}.rs  The file number ${String(index + 1)}.`)
  return [{ starts: ['rex', 'codebase', '--only', 'code', '--with-context'], answer: ran(`${lines.join('\n')}\n`) }]
}

test("the tree's brief holds the charter's reading with no Missing line, the policy, then the code map", async ($, on) => {
  const policy = await confirmedPolicy({ merge: 'allowed_if_checks_pass', per_job_usd: 20 })
  const session = masterWorld(on, { store: { [POLICY_KEY]: policy }, processAnswer: byRule([...placesRules('origin'), ...codebaseRules(3)]), modelAnswer: () => ({ isAnswered: true, text: READING, usage: USAGE }) })
  await $.session.start(START)

  const prompt = resultText(await openedTree($, 'Ship the parser'))

  expect(session.completions.map(each => each.system)).toEqual([READ_CHARTER])
  expect(READ_CHARTER).not.toContain('Missing')
  expect(prompt).toContain(`<reading>\n${READING}\n</reading>`)
  const block = [
    'merge=allowed_if_checks_pass',
    'merge_max_consequence=medium',
    'max_parallel_builders=auto',
    'max_jobs=8',
    'consequence_ceiling=high',
    'tier_cap=high',
    'tree_usd=150',
    'per_job_usd=20',
    'raise_limits=ask',
  ].join('\n')
  expect(prompt).toContain(`\n<policy>\n${block}\n</policy>\n`)
  expect(prompt).toContain('\n<codebase>\nsrc/file1.rs  The file number 1.\nsrc/file2.rs  The file number 2.\nsrc/file3.rs  The file number 3.\n</codebase>\n')
  expect(prompt.indexOf('</reading>')).toBeLessThan(prompt.indexOf('<policy>'))
  expect(prompt.indexOf('</policy>')).toBeLessThan(prompt.indexOf('<codebase>'))
})

test('the code map is cut at 150 lines with a line that counts the rest, and left out when rex codebase fails', async ($, on) => {
  masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy({ per_job_usd: null }) }, processAnswer: byRule([...placesRules('origin'), ...codebaseRules(170)]) })
  await $.session.start(START)

  const answered = await openedTree($, 'Ship the parser')

  const prompt = resultText(answered)
  const map = /<codebase>\n([\s\S]*)\n<\/codebase>/.exec(prompt)?.[1]?.split('\n') ?? []
  expect(map).toHaveLength(151)
  expect(map[149]).toBe('src/file150.rs  The file number 150.')
  expect(map[150]).toBe('(20 more lines cut)')
  expect(prompt).toContain('\nper_job_usd=none\n')
  expect(lineOf(answered, 'state')).toBe('running')
})

test('a failed rex codebase leaves the code map out and the tree still opens', async ($, on) => {
  masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() } })
  await $.session.start(START)

  const answered = await openedTree($, 'Ship the parser')

  expect(lineOf(answered, 'state')).toBe('running')
  expect(resultText(answered)).not.toContain('<codebase>')
  expect(resultText(answered)).toContain('<policy>')
})

for (const remote of ['origin', 'none'] as const) {
  test(`the tree's brief names the repo, its base and remote=${remote}, between the policy and the code map`, async ($, on) => {
    masterWorld(on, { store: { [POLICY_KEY]: await confirmedPolicy() }, processAnswer: byRule([...placesRules(remote), ...codebaseRules(1)]) })
    await $.session.start(START)

    const prompt = resultText(await openedTree($, 'Ship the parser'))

    expect(prompt).toContain(`\n<repo>\nrepo=${ROOT}\nbase=main\nremote=${remote}\n</repo>\n`)
    expect(prompt.indexOf('</policy>')).toBeLessThan(prompt.indexOf('<repo>'))
    expect(prompt.indexOf('</repo>')).toBeLessThan(prompt.indexOf('<codebase>'))
  })
}

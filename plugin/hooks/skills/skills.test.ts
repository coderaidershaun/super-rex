// Proves rex_skill: it refuses a path and a link that leads out of its skill folder, finds a skill in the fixed order of folders with the worktree rule, and answers its text with the hash of its bytes.
import { expect, test } from 'claude-code/testing'

import { ROOT, world } from '../book/kit'
import { sharedJson } from '../book/shared'
import { files, HOME, JOB, lineOf, resultText, seededBook } from '../tools/act-kit'
import { jsonField } from '../tools/binary'
import { hashText } from './index'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const
const WORKTREE = `${ROOT}.worktrees/${JOB}`

test('rex_skill refuses a path', async ($, on) => {
  files(on, { 'plugin:library/rex-code-tdd/SKILL.md': 'TDD\n' })
  world(on)
  await $.session.start(START)

  const dots = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: 'rex-code-tdd', file: '../../etc/passwd.md' })
  const slash = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: '/etc', file: 'SKILL.md' })
  const notMd = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: 'rex-code-tdd', file: 'run.sh' })
  const absolute = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: 'rex-code-tdd', file: '/etc/x.md' })

  expect(dots.deny).toBe('rex_skill: file must be a relative .md path inside the skill, with no .., got rex-code-tdd/../../etc/passwd.md; call it with a skill name such as rex-code-tdd and a file such as SKILL.md')
  expect(slash.deny).toBe('rex_skill: name must be a skill name of a-z, 0-9 and -, got /etc/SKILL.md; call it with a skill name such as rex-code-tdd and a file such as SKILL.md')
  expect(notMd.deny).toMatch(/^rex_skill: file must be a relative \.md path/)
  expect(absolute.deny).toMatch(/^rex_skill: file must be a relative \.md path/)
})

test('resolver order and worktree rule: main checkout, then the job worktree for skills main lacks, then plugin, then the person', async ($, on) => {
  files(on, {
    [`${ROOT}/.rex/skills/shared/SKILL.md`]: 'main copy\n',
    [`${WORKTREE}/.rex/skills/shared/SKILL.md`]: 'older worktree copy\n',
    [`${WORKTREE}/.rex/skills/job-written/SKILL.md`]: 'written by the job\n',
    'plugin:library/job-written/SKILL.md': 'plugin copy\n',
    'plugin:skills/rex-deliver/SKILL.md': 'deliver\n',
    'plugin:skills/rex-deliver/stopped.md': 'stopped\n',
    'plugin:library/rex-code-tdd/SKILL.md': 'library\n',
    [`${HOME}/.claude/skills/rex-code-tdd/SKILL.md`]: 'personal copy\n',
    [`${HOME}/.claude/skills/mine/SKILL.md`]: 'personal\n',
  })
  world(on, { seed: seededBook() })
  await $.session.start(START)

  const read = async (name: string, file?: string) => $.tool.call({ tool: 'mcp__super-rex__rex_skill', name, ...(file === undefined ? {} : { file }) })
  const shared = await read('shared')
  const written = await read('job-written')
  const deliver = await read('rex-deliver', 'stopped.md')
  const tdd = await read('rex-code-tdd')
  const mine = await read('mine')
  const agent = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: 'job-written', agentId: 'a9' })

  expect([lineOf(shared, 'source'), resultText(shared).split('\n\n')[1]]).toEqual(['main', 'main copy\n'])
  expect([lineOf(written, 'source'), resultText(written).split('\n\n')[1]]).toEqual(['worktree', 'written by the job\n'])
  expect([lineOf(deliver, 'source'), lineOf(deliver, 'file')]).toEqual(['plugin', 'stopped.md'])
  expect(lineOf(tdd, 'source')).toBe('plugin')
  expect(lineOf(mine, 'source')).toBe('user')
  expect(lineOf(agent, 'source')).toBe('plugin')
  expect(lineOf(shared, 'hash')).toBe(await hashText('main copy\n'))
})

test('a skill file hash is sha256 of its bytes, as the shared hash vectors write it', async ($, on) => {
  const vectors = jsonField(sharedJson('fixtures/shared/hashes/vectors.json'), 'files')
  const list: readonly unknown[] = Array.isArray(vectors) ? vectors : []
  const tree: { [path: string]: string } = {}
  list.forEach((vector, index) => {
    tree[`plugin:library/vector-${String(index)}/SKILL.md`] = String(jsonField(vector, 'text'))
  })
  files(on, tree)
  world(on)
  await $.session.start(START)

  expect(list).toHaveLength(6)
  for (const [index, vector] of list.entries()) {
    const read = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: `vector-${String(index)}` })
    expect(lineOf(read, 'hash')).toBe(String(jsonField(vector, 'hash')))
  }
})

test('rex_skill refuses a symbolic link that leads out of its skill folder, and follows one that stays inside', async ($, on) => {
  files(
    on,
    {
      [`${HOME}/.aws/credentials`]: 'secret\n',
      [`${HOME}/elsewhere/away/SKILL.md`]: 'not a skill of any root\n',
      [`${ROOT}/.rex/skills/inside/SKILL.md`]: 'inside\n',
      [`${HOME}/dotfiles/skills/mine/SKILL.md`]: 'personal\n',
    },
    {
      [`${ROOT}/.rex/skills/leak/SKILL.md`]: `${HOME}/.aws/credentials`,
      [`${ROOT}/.rex/skills/away`]: `${HOME}/elsewhere/away`,
      [`${ROOT}/.rex/skills/inside/again.md`]: `${ROOT}/.rex/skills/inside/SKILL.md`,
      [`${HOME}/.claude/skills`]: `${HOME}/dotfiles/skills`,
    },
  )
  world(on)
  await $.session.start(START)

  const leak = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: 'leak' })
  const away = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: 'away' })
  const again = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: 'inside', file: 'again.md' })
  const mine = await $.tool.call({ tool: 'mcp__super-rex__rex_skill', name: 'mine' })

  expect(leak.deny).toBe(`rex_skill: leak/SKILL.md leads outside its skill folder, got ${HOME}/.aws/credentials; name a skill of plugin/skills, plugin/library or .rex/skills`)
  expect(away.deny).toBe(`rex_skill: away/SKILL.md leads outside its skill folder, got ${HOME}/elsewhere/away/SKILL.md; name a skill of plugin/skills, plugin/library or .rex/skills`)
  expect(resultText(again).split('\n\n')[1]).toBe('inside\n')
  expect([lineOf(mine, 'source'), resultText(mine).split('\n\n')[1]]).toEqual(['user', 'personal\n'])
})

// Proves the cut of a test run: passing test lines and build steps are left out, every failure line and the summary stay, and other commands pass whole.
import { expect, test } from 'claude-code/testing'

import { ROOT, world } from '../book/kit'
import { files } from '../tools/act-kit'
import { cutPassing, isTestRun, SEE_EVERY_LINE } from './index'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const

const PASSING = ['running 3 tests', 'test parser::reads_empty ... ok', 'test parser::reads_one ... ok', 'test parser::slow ... ignored', '', 'test result: ok. 2 passed; 0 failed; 1 ignored', ''].join('\n')
const FAILING = [
  'running 2 tests',
  'test parser::reads_empty ... ok',
  'test parser::reads_one ... FAILED',
  '',
  'failures:',
  '',
  '---- parser::reads_one stdout ----',
  'test parser::inner ... ok',
  "thread 'parser::reads_one' panicked at src/parser.rs:9:5",
  '',
  'failures:',
  '    parser::reads_one',
  '',
  'test result: FAILED. 1 passed; 1 failed; 0 ignored',
  '',
].join('\n')
const BUILD = ['   Compiling rex v0.8.0 (/repo)', '    Finished `test` profile [unoptimized] target(s) in 2.1s', '     Running unittests src/lib.rs', ''].join('\n')

test('passing test lines trimmed, failure lines kept', () => {
  expect(cutPassing(PASSING)).toEqual({ text: 'running 3 tests\n\ntest result: ok. 2 passed; 0 failed; 1 ignored\n', left: 3, of: 6 })
  const failing = cutPassing(FAILING)
  expect(failing.text).toBe(FAILING.replace('test parser::reads_empty ... ok\n', ''))
  expect(failing.text).toContain('test parser::inner ... ok')
  expect(cutPassing(BUILD).text).toBe('     Running unittests src/lib.rs\n')
  expect(cutPassing('=== RUN   TestA\n--- PASS: TestA (0.00s)\nPASS\nok  \tpkg\t0.01s\n').text).toBe('ok  \tpkg\t0.01s\n')
  expect(cutPassing('tests/a.py::test_one PASSED [ 50%]\ntests/a.py::test_two FAILED [100%]\n==== 1 failed, 1 passed in 0.1s ====\n').text).toBe('tests/a.py::test_two FAILED [100%]\n==== 1 failed, 1 passed in 0.1s ====\n')
})

test('only one plain test command is cut', () => {
  expect(['cargo test', 'RUST_LOG=debug cargo test --locked', 'npm test', 'cargo test 2>&1'].filter(command => !isTestRun(command))).toEqual([])
  expect(['cargo test | tail', 'cargo test -- --nocapture', 'cd x && cargo test', 'cargo build', 'echo cargo test', 'cargo test > out.log'].filter(isTestRun)).toEqual([])
})

test('a test run reaches the agent with its passing lines cut and a note, and another command reaches it whole', async ($, on) => {
  files(on, {})
  on('tool.call', (_$, e) => ({ result: { stdout: e.tool === 'Bash' && e.command === 'ls' ? 'a\nb\n' : PASSING, stderr: BUILD, interrupted: false } }))
  world(on)
  await $.session.start(START)

  const tested = await $.tool.call({ tool: 'Bash', command: 'cargo test' })
  const listed = await $.tool.call({ tool: 'Bash', command: 'ls' })

  expect(tested.result).toEqual({ stdout: `running 3 tests\n\ntest result: ok. 2 passed; 0 failed; 1 ignored\nrex trim: left out 5 passing lines of 9. ${SEE_EVERY_LINE}\n`, stderr: '     Running unittests src/lib.rs\n', interrupted: false })
  expect(listed.result).toEqual({ stdout: 'a\nb\n', stderr: BUILD, interrupted: false })
})

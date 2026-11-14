// Which shell commands run tests: only their output is cut, and only when the command is one plain test run.

/** Every command that runs tests, as the words it starts with. */
export const TEST_COMMANDS: readonly string[] = [
  'cargo test', 'cargo nextest run', 'go test', 'pytest', 'python -m pytest', 'python3 -m pytest', 'uv run pytest',
  'npm test', 'npm run test', 'pnpm test', 'pnpm run test', 'yarn test', 'bun test', 'deno test', 'npx jest', 'npx vitest',
  'jest', 'vitest', 'mix test', 'rspec', 'bundle exec rspec', 'dotnet test', 'mvn test', 'gradle test', './gradlew test',
  'swift test', 'ctest', 'make test',
]

/** Words that ask the runner for output the agent wants to read; such a run is left whole. */
export const OUTPUT_FLAGS: readonly string[] = [
  '--nocapture', '--no-capture', '--show-output', '--list', '--listTests', '--help', '-h', '--version', '-s', '--capture=no',
  '--collect-only', '--co', '-v', '-vv', '-vvv', '--verbose', '--no-run',
]

/** A pipe, a redirect or a second command means the agent already chose what it sees, so the run is left whole. */
const CHOSEN_OUTPUT = ['|', '>', '<', ';', '&', '`', '$(', '\n', '\r', '\\', '#']

const ASSIGNMENT = /^[A-Za-z_][A-Za-z0-9_]*=/

/** True when the command is one test command, alone. */
export function isTestRun(command: string): boolean {
  const checked = command.replace(/2>&1/g, '')
  const words = checked.split(/\s+/).filter(word => word !== '')
  const isChosen = CHOSEN_OUTPUT.some(text => checked.includes(text)) || words.some(word => word === 'exit' || OUTPUT_FLAGS.includes(word))
  if (isChosen) {
    return false
  }
  const start = words.findIndex(word => !ASSIGNMENT.test(word))
  const run = start === -1 ? [] : words.slice(start)
  return TEST_COMMANDS.some(test => {
    const testWords = test.split(' ')
    return testWords.every((word, index) => run[index] === word)
  })
}

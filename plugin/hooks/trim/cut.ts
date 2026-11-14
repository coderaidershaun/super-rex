// Cuts the lines a test runner prints for a passing test or a build step, so an agent's context keeps only the summary and every line of a failure.

type Place = 'before-tests' | 'in-tests' | 'in-failure-report'

function isCargoTestsStart(line: string): boolean {
  return /^running \d+ tests?$/.test(line)
}

/** Where a line stands in cargo's output; a failure report ends only at cargo's `test result:` line. */
function placeAfter(place: Place, line: string): Place {
  if (line.startsWith('test result:')) {
    return 'in-tests'
  }
  const opensReport = line === 'failures:' || (line.startsWith('---- ') && line.endsWith(' ----'))
  if (place === 'in-failure-report' || opensReport) {
    return 'in-failure-report'
  }
  return isCargoTestsStart(line) ? 'in-tests' : place
}

const SUMMARY_STARTS = ['test result:', 'Summary [', 'ok ', 'ok\t', 'Tests:', 'Test Suites:', 'Tests ', 'Test Files ', 'Passed!', 'Total tests:', 'Finished in ', '[INFO] Tests run:', '[INFO] BUILD ']
const SUMMARY_PARTS = [' examples, ', ' tests, ', ' tests passed, ', ' passing (']

/** A line that reports the totals of a run; kept even when it has the form of a passing line. */
function isSummary(line: string): boolean {
  const trimmed = line.trim()
  const isPytestTotals = trimmed.startsWith('=') && [' passed', ' failed', ' error'].some(word => trimmed.includes(word))
  return isPytestTotals || SUMMARY_PARTS.some(part => trimmed.includes(part)) || SUMMARY_STARTS.some(start => trimmed.startsWith(start))
}

function isCargoTestOk(line: string): boolean {
  const found = /^test (\S+) \.\.\. (.*)$/.exec(line)
  return found !== null && (found[2] === 'ok' || (found[2] ?? '').startsWith('ignored'))
}

function isCargoBuildStep(line: string): boolean {
  if (!line.startsWith(' ')) {
    return false
  }
  const words = line.trim().split(/\s+/)
  const isStep = ['Compiling', 'Checking', 'Downloaded', 'Fresh'].includes(words[0] ?? '') && /^v\d/.test(words[2] ?? '')
  return isStep || (words[0] === 'Finished' && line.includes(' target(s) in '))
}

function isGoPass(line: string): boolean {
  const trimmed = line.trimStart()
  const isMarker = ['=== RUN ', '=== PAUSE ', '=== CONT '].some(start => line.startsWith(start))
  const isPass = (trimmed.startsWith('--- PASS: ') || trimmed.startsWith('--- SKIP: ')) && trimmed.endsWith('s)')
  return isMarker || isPass || line === 'PASS' || line.startsWith('ok  \t')
}

function isPytestPass(line: string): boolean {
  const found = /^(\S+::\S+) PASSED\s*(\[\s*\d+%\])?\s*$/.exec(line)
  return found !== null
}

function isJestPass(line: string): boolean {
  const isSuite = /^PASS \S+$/.test(line)
  const isTest = line.startsWith(' ') && ['✓ ', '✔ ', '√ '].some(mark => line.trimStart().startsWith(mark))
  return isSuite || isTest
}

function isDotnetPass(line: string): boolean {
  return line.startsWith(' ') && /^Passed \S.* \[.*\]$/.test(line.trimStart())
}

function isNextestPass(line: string): boolean {
  return /^PASS \[\s*[^\]]+s\] \S/.test(line.trimStart())
}

/** True only for a line in the exact form a runner prints for a passing test or a build step. */
function isPassing(line: string, place: Place): boolean {
  return isCargoTestOk(line) || (place === 'before-tests' && isCargoBuildStep(line)) || isGoPass(line) || isPytestPass(line) || isJestPass(line) || isNextestPass(line) || isDotnetPass(line)
}

export type Cut = { readonly text: string; readonly left: number; readonly of: number }

/** The output with each proven passing line left out; no line of a failure report is ever cut. */
export function cutPassing(output: string): Cut {
  const lines = output.split('\n')
  const hasEnd = output.endsWith('\n')
  const all = hasEnd ? lines.slice(0, -1) : lines
  const kept: string[] = []
  let place: Place = 'before-tests'
  for (const line of all) {
    place = placeAfter(place, line)
    const isCut = place !== 'in-failure-report' && isPassing(line, place) && !isSummary(line)
    if (!isCut) {
      kept.push(line)
    }
  }
  const text = kept.join('\n') + (hasEnd && kept.length > 0 ? '\n' : '')
  return { text, left: all.length - kept.length, of: all.length }
}

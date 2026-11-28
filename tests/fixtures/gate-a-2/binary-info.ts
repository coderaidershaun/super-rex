// Records what the measures ran on: the binary's version, the base commit, whether the code and library differ from it, and the vocabulary and index versions.
// Usage: node tests/fixtures/gate-a-2/binary-info.ts [--pool <folder>]
import { BASE_COMMIT, DEFAULT_POOL, flag, isObject, LIBRARY, main, readAnswer, readFlags, readJson, REX, run, say, ScriptError, search, shown, text, VOCABULARY } from './lib.ts'

/** The paths that the binary, the library and the vocabulary are built from. */
const SOURCES = ['src', 'plugin/library', 'plugin/vocab', 'Cargo.toml', 'Cargo.lock']

main(() => {
  const flags = readFlags(process.argv.slice(2), ['pool'])
  const pool = flag(flags, 'pool', DEFAULT_POOL)
  const version = run(REX, ['--version'])
  if (version.status !== 0) throw new ScriptError(`${REX} --version exited ${version.status}: ${version.stderr.trim()}`)
  say('i0', `binary ${shown(REX)}: ${version.stdout.trim()}`)
  const base = run('git', ['rev-parse', '--verify', `${BASE_COMMIT}^{commit}`])
  say('i1', `base commit ${BASE_COMMIT}: ${base.status === 0 ? 'present' : 'MISSING'}; ${run('git', ['log', '-1', '--format=%s', BASE_COMMIT]).stdout.trim()}`)
  const changed = run('git', ['diff', '--stat', BASE_COMMIT, '--', ...SOURCES]).stdout.trim()
  const untracked = run('git', ['ls-files', '--others', '--exclude-standard', '--', ...SOURCES]).stdout.trim()
  say('i2', `${SOURCES.join(', ')} against the base: ${changed === '' && untracked === '' ? 'no change, so the binary built here is the base binary' : `CHANGED: ${changed} ${untracked}`}`)
  const terms = readJson(VOCABULARY)
  say('i3', `vocabulary ${shown(VOCABULARY)} version ${isObject(terms) ? (text(terms['version']) ?? 'none') : 'none'}`)
  const probe = { requests: [{ id: 'index-version' }] }
  say('i4', `index version of the library alone: ${readAnswer(search([LIBRARY], probe)).indexVersion}`)
  say('i4', `index version of the library then the pool: ${readAnswer(search([LIBRARY, pool], probe)).indexVersion}`)
})

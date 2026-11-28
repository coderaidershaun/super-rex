// Fills `facts.repo_has` and `facts.repo_dependencies` of each request form from the repo at the base commit, as spec 9.1 says the mod does, replacing what the writer gave.
// Usage: node tests/fixtures/gate-a/fill-facts.ts --forms <folder> --out <folder> [--commit <sha>]
import { basename, join } from 'node:path'

import { BASE_COMMIT, ensureFolder, flag, isObject, type Json, JOB_DIR, jsonFiles, main, readFlags, readJson, requiredFlag, run, say, ScriptError, writeJson } from './lib.ts'

/** Every path that git tracks at the commit, sorted, as the repo's root sees them. */
function repoPaths(commit: string): string[] {
  const listed = run('git', ['-C', JOB_DIR, 'ls-tree', '-r', '--name-only', commit])
  if (listed.status !== 0) throw new ScriptError(`git ls-tree of ${commit} exited ${listed.status}: ${listed.stderr.trim()}`)
  return listed.stdout
    .split('\n')
    .filter((line) => line !== '')
    .sort()
}

/** The names under each dependencies table of the root Cargo.toml at the commit: [dependencies], [dev-dependencies], [build-dependencies] and target tables. */
function repoDependencies(commit: string): string[] {
  const shown = run('git', ['-C', JOB_DIR, 'show', `${commit}:Cargo.toml`])
  if (shown.status !== 0) throw new ScriptError(`git show ${commit}:Cargo.toml exited ${shown.status}: ${shown.stderr.trim()}`)
  const names = new Set<string>()
  let inDependencies = false
  for (const raw of shown.stdout.split('\n')) {
    const line = raw.trim()
    if (line.startsWith('[')) {
      inDependencies = /^\[(?:target\..+\.)?(?:dev-|build-)?dependencies\]$/.test(line)
      continue
    }
    const name = /^([A-Za-z0-9_-]+)\s*(?:=|\.)/.exec(line)?.[1]
    if (inDependencies && name !== undefined) names.add(name)
  }
  return [...names].sort()
}

/** The form with the two repo facts replaced. Other facts keep their order; the two go last. */
function fillForm(form: Json, paths: readonly string[], dependencies: readonly string[]): { form: Json; replaced: boolean } {
  if (!isObject(form)) throw new ScriptError('a form must be a JSON object')
  const given = form['facts']
  const facts = isObject(given) ? given : {}
  const replaced = 'repo_has' in facts || 'repo_dependencies' in facts
  const kept = Object.fromEntries(Object.entries(facts).filter(([name]) => name !== 'repo_has' && name !== 'repo_dependencies'))
  return { form: { ...form, facts: { ...kept, repo_has: [...paths], repo_dependencies: [...dependencies] } }, replaced }
}

main(() => {
  const flags = readFlags(process.argv.slice(2), ['forms', 'out', 'commit'])
  const forms = requiredFlag(flags, 'forms')
  const out = requiredFlag(flags, 'out')
  const commit = flag(flags, 'commit', BASE_COMMIT)
  const paths = repoPaths(commit)
  const dependencies = repoDependencies(commit)
  say('f0', `repo at ${commit}: ${paths.length} tracked paths, ${dependencies.length} dependencies (${dependencies.join(' ')})`)
  ensureFolder(out)
  let written = 0
  let replaced = 0
  for (const file of jsonFiles(forms)) {
    const filled = fillForm(readJson(file), paths, dependencies)
    writeJson(join(out, basename(file)), filled.form)
    written += 1
    if (filled.replaced) replaced += 1
  }
  say('f1', `${written} forms filled into ${out}; ${replaced} had repo facts from the writer, now replaced`)
})

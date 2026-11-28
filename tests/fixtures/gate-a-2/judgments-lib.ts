// Reads the judge's verdicts on tier A and B pairs from one or more judgments files, keyed by item, library and skill.
// A later file may add verdicts but never change one, because the old verdicts of the first measurement stay as they were.
import { isArray, isObject, readJson, ScriptError, text } from './lib.ts'

export type Verdict = { readonly verdict: string; readonly tier: string; readonly file: string }

export const VERDICTS = ['right', 'wrong', 'doubtful']

export function pairKey(id: string, library: string, skill: string): string {
  return `${id} ${library}/${skill}`
}

export function readVerdicts(files: readonly string[]): Map<string, Verdict> {
  const verdicts = new Map<string, Verdict>()
  for (const file of files) {
    const judgments = readJson(file)
    if (!isObject(judgments) || !isArray(judgments['items'])) throw new ScriptError(`${file} has no \`items\` list`)
    for (const item of judgments['items']) {
      if (!isObject(item) || !isArray(item['assignments'])) throw new ScriptError(`${file}: each item needs an id and an assignments list`)
      const id = text(item['id']) ?? ''
      for (const entry of item['assignments']) {
        const verdict = isObject(entry) ? text(entry['verdict']) : undefined
        if (!isObject(entry) || verdict === undefined || !VERDICTS.includes(verdict)) throw new ScriptError(`${file}: assignment of ${id} needs a verdict of ${VERDICTS.join(', ')}`)
        const key = pairKey(id, text(entry['library']) ?? 'library', text(entry['skill']) ?? '')
        const known = verdicts.get(key)
        if (known !== undefined && known.verdict !== verdict) throw new ScriptError(`${file} gives ${key} the verdict ${verdict}, but ${known.file} gave it ${known.verdict}; an old verdict is never changed`)
        if (known === undefined) verdicts.set(key, { verdict, tier: text(entry['tier']) ?? '', file })
      }
    }
  }
  return verdicts
}

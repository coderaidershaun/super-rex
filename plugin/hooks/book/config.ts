// Reads .rex/config.json, which says whether this session keeps a store and whether its jobs merge with no question, and the plugin's own version from its manifest.
import type { EngineInterface } from 'claude-code'

import { errorText } from './failure'

type Config = { readonly storage: 'on' | 'off'; readonly jobsFromSdk: boolean; readonly autoMerge: boolean }
type ConfigRead = { readonly kind: 'missing' } | { readonly kind: 'unreadable'; readonly reason: string } | { readonly kind: 'read'; readonly config: Config }

export async function readConfig($: EngineInterface, root: string): Promise<ConfigRead> {
  const path = `${root}/.rex/config.json`
  if (!(await $.fs.exists(path))) {
    return { kind: 'missing' }
  }
  let text: string
  try {
    text = await $.fs.read(path)
  } catch (error) {
    return { kind: 'unreadable', reason: errorText(error) }
  }
  return parseConfig(text)
}

export function parseConfig(text: string): ConfigRead {
  let value: unknown
  try {
    value = JSON.parse(text)
  } catch (error) {
    return { kind: 'unreadable', reason: errorText(error) }
  }
  if (typeof value !== 'object' || value === null) {
    return { kind: 'unreadable', reason: 'it holds no JSON object' }
  }
  const storage: unknown = Reflect.get(value, 'storage')
  const jobsFromSdk: unknown = Reflect.get(value, 'jobs_from_sdk')
  const autoMerge: unknown = Reflect.get(value, 'auto_merge')
  if (storage !== 'on' && storage !== 'off') {
    return { kind: 'unreadable', reason: `storage must be "on" or "off", got ${JSON.stringify(storage) ?? 'nothing'}` }
  }
  if (jobsFromSdk !== undefined && typeof jobsFromSdk !== 'boolean') {
    return { kind: 'unreadable', reason: `jobs_from_sdk must be true or false, got ${JSON.stringify(jobsFromSdk)}` }
  }
  if (autoMerge !== undefined && typeof autoMerge !== 'boolean') {
    return { kind: 'unreadable', reason: `auto_merge must be true or false, got ${JSON.stringify(autoMerge)}` }
  }
  return { kind: 'read', config: { storage, jobsFromSdk: jobsFromSdk ?? true, autoMerge: autoMerge ?? false } }
}

/** The plugin's own version, from its manifest; `unknown` when the manifest cannot be read. */
export async function pluginVersion($: EngineInterface): Promise<string> {
  try {
    const value: unknown = JSON.parse(await $.fs.read(`${$.plugin.root}/.claude-plugin/plugin.json`))
    const version: unknown = typeof value === 'object' && value !== null ? Reflect.get(value, 'version') : undefined
    return typeof version === 'string' ? version : 'unknown'
  } catch (error) {
    return `unknown (${errorText(error)})`
  }
}

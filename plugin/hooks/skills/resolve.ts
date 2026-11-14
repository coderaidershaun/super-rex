// Finds a skill by name in the folders an agent may read skills from, in a fixed order, and gives its text with the hash of its bytes.
import type { EngineInterface } from 'claude-code'

import type { RexSkillSource } from '../../types'

export const SKILL_NAME = /^[a-z0-9-]+$/
export const SKILL_FILE = /^[A-Za-z0-9._/-]+$/

export type ResolvedSkill = {
  readonly skill: string
  readonly file: string
  readonly source: RexSkillSource
  readonly path: string
  readonly hash: string
  readonly text: string
}

/** Where skill folders live for one caller; `worktree` is the folder of the caller's job, when it has one. */
export type SkillPlaces = {
  readonly repo: string
  readonly worktree: string | null
  readonly pluginRoot: string
  readonly home: string | null
}

type Folder = { readonly source: RexSkillSource; readonly dir: string }

/**
 * The main checkout first, so a skill a feedback edit changed there is not
 * hidden by an older copy a job's worktree holds; the worktree then serves
 * only skills the main checkout does not have, which are the ones its job
 * wrote. Then the plugin's own skills and library, then the person's own.
 */
export function skillFolders(places: SkillPlaces): readonly Folder[] {
  const folders: Folder[] = [{ source: 'main', dir: `${places.repo}/.rex/skills` }]
  if (places.worktree !== null) {
    folders.push({ source: 'worktree', dir: `${places.worktree}/.rex/skills` })
  }
  folders.push({ source: 'plugin', dir: `${places.pluginRoot}/skills` }, { source: 'plugin', dir: `${places.pluginRoot}/library` })
  if (places.home !== null) {
    folders.push({ source: 'user', dir: `${places.home}/.claude/skills` })
  }
  return folders
}

/** Null when the name and file are safe to look up inside a skill folder, else what is wrong. */
export function nameProblem(name: string, file: string): string | null {
  if (!SKILL_NAME.test(name)) {
    return 'name must be a skill name of a-z, 0-9 and -'
  }
  const isInside = SKILL_FILE.test(file) && !file.startsWith('/') && !file.split('/').includes('..') && file.endsWith('.md')
  return isInside ? null : 'file must be a relative .md path inside the skill, with no ..'
}

function bytesOf(base64: string): Uint8Array {
  const binary = atob(base64)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index)
  }
  return bytes
}

export async function sha256Of(bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))
  return `sha256:${[...digest].map(byte => byte.toString(16).padStart(2, '0')).join('')}`
}

export async function hashText(text: string): Promise<string> {
  return sha256Of(new TextEncoder().encode(text))
}

/** Where a path lands with every symbolic link followed; undefined when it leads nowhere or cannot be resolved. */
async function realPathOf($: EngineInterface, path: string): Promise<string | undefined> {
  try {
    return (await $.fs.stat(path, { resolve: true })).realPath
  } catch (error) {
    if (error instanceof Error) {
      return undefined
    }
    throw error
  }
}

/** True when `child` is a path below the folder `parent`, by spelling; both must already be real paths. */
function isBelow(child: string, parent: string): boolean {
  const folder = parent.replace(/[\\/]+$/, '')
  return child.length > folder.length && child.startsWith(folder) && (child[folder.length] === '/' || child[folder.length] === '\\')
}

export type SkillLookup =
  | { readonly kind: 'found'; readonly skill: ResolvedSkill }
  | { readonly kind: 'missing' }
  | { readonly kind: 'outside'; readonly landed: string | null }

/**
 * The skill's folder is the first one in order that holds its SKILL.md; its
 * files come from that folder only. The mod reads files the sandbox may deny
 * an agent, and an agent can write links into `.rex/skills`, so the skill
 * folder must land inside its root and the file inside the skill folder.
 */
export async function findSkill($: EngineInterface, places: SkillPlaces, name: string, file: string): Promise<SkillLookup> {
  if (nameProblem(name, file) !== null) {
    return { kind: 'missing' }
  }
  for (const folder of skillFolders(places)) {
    if (!(await $.fs.exists(`${folder.dir}/${name}/SKILL.md`))) {
      continue
    }
    const path = `${folder.dir}/${name}/${file}`
    if (!(await $.fs.exists(path))) {
      return { kind: 'missing' }
    }
    const root = await realPathOf($, folder.dir)
    const skillDir = await realPathOf($, `${folder.dir}/${name}`)
    const landed = await realPathOf($, path)
    if (root === undefined || skillDir === undefined || landed === undefined || !isBelow(skillDir, root) || !isBelow(landed, skillDir)) {
      return { kind: 'outside', landed: landed ?? null }
    }
    // HAZARD: a folder on the resolved path that is swapped for a link between
    // the check above and this read is still followed. Reading the resolved
    // path, not the one asked for, keeps that gap as small as it can be here.
    const read = await $.fs.read(landed, { as: 'bytes' })
    const bytes = bytesOf(read.base64)
    return { kind: 'found', skill: { skill: name, file, source: folder.source, path: landed, hash: await sha256Of(bytes), text: new TextDecoder().decode(bytes) } }
  }
  return { kind: 'missing' }
}

/** The skill file, or null when no folder holds it or it leads outside its skill folder. */
export async function resolveSkill($: EngineInterface, places: SkillPlaces, name: string, file: string): Promise<ResolvedSkill | null> {
  const lookup = await findSkill($, places, name, file)
  return lookup.kind === 'found' ? lookup.skill : null
}

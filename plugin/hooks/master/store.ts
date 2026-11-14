// Reads and writes the person's policy for the session's repo in the plugin's own store, and hashes it so a tree can pin the policy it started under.
import type { EngineInterface } from 'claude-code'

import { hashText } from '../skills/index'
import { hashedText, policyOf } from './policy'
import type { Policy } from './policy'

export type StoredPolicy = { readonly policy: Policy; readonly hash: string; readonly isConfirmed: boolean }

/** The session root as the file system resolves it, so two spellings of one repo share one policy. */
export async function realRoot($: EngineInterface): Promise<string> {
  const root = await $.session.root()
  const stat = await $.fs.stat(root, { resolve: true })
  return stat.realPath ?? root
}

export async function policyStoreKey($: EngineInterface): Promise<string> {
  const hash = await hashText(await realRoot($))
  return `policy:${hash.replace(/^sha256:/, '')}`
}

export function policyHash(policy: Policy): Promise<string> {
  return hashText(hashedText(policy))
}

/** The stored policy of this repo, or null when the person never stored one; a stored value of the wrong shape throws `PolicyShapeError`. */
export async function readPolicy($: EngineInterface): Promise<StoredPolicy | null> {
  const stored = await $.store.get(await policyStoreKey($))
  if (stored === undefined) {
    return null
  }
  const policy = policyOf(stored)
  const hash = await policyHash(policy)
  return { policy, hash, isConfirmed: policy.confirmed_hash === hash }
}

/** Stores the policy; `confirm` also records its hash as the one the person confirmed. */
export async function writePolicy($: EngineInterface, policy: Policy, confirm: 'confirm' | 'keep'): Promise<StoredPolicy> {
  const hash = await policyHash(policy)
  const written: Policy = confirm === 'confirm' ? { ...policy, confirmed_hash: hash } : policy
  await $.store.set(await policyStoreKey($), written)
  return { policy: written, hash, isConfirmed: written.confirmed_hash === hash }
}

// The policy a running tree is pinned to: a tree whose policy changed since its master started is paused, and refuses new builds and merges until the person confirms.
import type { EngineInterface } from 'claude-code'

import type { RexBook, RexTree } from '../../types'
import { fail } from '../book/index'
import { PolicyShapeError } from './policy'
import type { Policy } from './policy'
import { readPolicy } from './store'

export function isLiveTree(tree: RexTree): boolean {
  return tree.state === 'running' || tree.state === 'paused'
}

/** The trees of this session that run, or wait paused for the person to confirm a changed policy. */
export function liveTrees(book: RexBook): readonly RexTree[] {
  return book.trees.filter(tree => tree.session === book.session && isLiveTree(tree))
}

/** The live tree this agent is the master of: running, or paused by a policy change. */
export function treeOfMaster(book: RexBook, agentId: string | null): RexTree | undefined {
  return agentId === null ? undefined : book.trees.find(tree => tree.master === agentId && isLiveTree(tree))
}

export type Pinned = { readonly kind: 'pinned'; readonly policy: Policy; readonly hash: string } | { readonly kind: 'refused'; readonly text: string }

export async function pinnedPolicy($: EngineInterface, tree: RexTree): Promise<Pinned> {
  let stored
  try {
    stored = await readPolicy($)
  } catch (error) {
    if (error instanceof PolicyShapeError) {
      return { kind: 'refused', text: `rex: the stored policy of this repo is unreadable, got ${error.message}; run /rex policy set ${error.key} <value>` }
    }
    throw error
  }
  if (stored === null) {
    return { kind: 'refused', text: `rex: the policy of this repo is gone since master tree ${tree.id} started, got none; confirm one with /rex policy confirm` }
  }
  if (tree.state === 'paused' || stored.hash !== tree.policyHash) {
    if (tree.state === 'running') {
      await fail($, { kind: 'policy-changed', tree: tree.id, pinned: tree.policyHash, current: stored.hash })
    }
    return { kind: 'refused', text: `rex: policy changed since this master started, got hash ${stored.hash}; confirm it with /rex policy confirm` }
  }
  return { kind: 'pinned', policy: stored.policy, hash: stored.hash }
}

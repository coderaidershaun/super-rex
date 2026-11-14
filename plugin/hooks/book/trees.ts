// Pure changes of master trees and of the spawns the mod is about to make, each with its event for the binary.
import type { RexBook, RexPendingSpawn, RexTree, RexTreeNote, RexTreeTick } from '../../types'
import { queued, replaceJob } from './events'

export type TreeInput = Omit<RexTree, 'session' | 'master' | 'ledgerNano' | 'builders' | 'ticks' | 'notes' | 'startedAt' | 'closedAt' | 'reportFile'>

function replaceTree(book: RexBook, id: string, change: (tree: RexTree) => RexTree): RexBook {
  return { ...book, trees: book.trees.map(tree => (tree.id === id ? change(tree) : tree)) }
}

export function findTree(book: RexBook, id: string): RexTree | undefined {
  return book.trees.find(tree => tree.id === id)
}

export function treeIn(book: RexBook, input: TreeInput, at: number): RexBook {
  const old = findTree(book, input.id)
  const tree: RexTree = {
    ...input,
    session: book.session,
    master: null,
    ledgerNano: old?.ledgerNano ?? 0,
    builders: old?.builders ?? [],
    ticks: [],
    notes: old?.notes ?? [],
    startedAt: at,
    closedAt: null,
    reportFile: null,
  }
  const trees = old === undefined ? [...book.trees, tree] : book.trees.map(each => (each.id === tree.id ? tree : each))
  return queued({ ...book, trees }, at, 'tree.open', 'main', null, {
    id: tree.id,
    charter: tree.charter,
    charter_hash: tree.charterHash,
    policy_hash: tree.policyHash,
    cap_nano: tree.capNano,
    session: tree.session,
  })
}

export function linkIn(book: RexBook, treeId: string, jobId: string, agentId: string, at: number): RexBook {
  const tree = findTree(book, treeId)
  if (tree === undefined) {
    return book
  }
  const builders = [...tree.builders.filter(builder => builder.job !== jobId), { job: jobId, agentId }]
  const linked = replaceTree(book, treeId, old => ({ ...old, builders }))
  const owned = replaceJob(linked, jobId, job => ({ ...job, owner: { session: book.session, builder: agentId }, updatedAt: at }))
  return queued(owned, at, 'tree.link', tree.master ?? 'main', jobId, { tree: treeId, job: jobId, agent_id: agentId })
}

export function noteIn(book: RexBook, treeId: string, note: RexTreeNote, at: number): RexBook {
  const tree = findTree(book, treeId)
  if (tree === undefined) {
    return book
  }
  const noted = replaceTree(book, treeId, old => ({ ...old, notes: [...old.notes, note] }))
  return queued(noted, at, 'tree.note', tree.master ?? 'main', note.job, { tree: treeId, text: note.text })
}

/** Keeps the last few ticks of each job only: the watchdog compares the newest ones. */
const TICKS_KEPT = 50

export function tickIn(book: RexBook, treeId: string, tick: RexTreeTick): RexBook {
  return replaceTree(book, treeId, old => ({ ...old, ticks: [...old.ticks, tick].slice(-TICKS_KEPT) }))
}

/** `running` and `paused` change a live tree; `done`, `blocked` and `lost` close it. */
export function treeStateIn(book: RexBook, treeId: string, state: RexTree['state'], reportFile: string | null, at: number): RexBook {
  const tree = findTree(book, treeId)
  if (tree === undefined || tree.state === state) {
    return book
  }
  const agent = tree.master ?? 'main'
  if (state === 'running' || state === 'paused') {
    return queued(replaceTree(book, treeId, old => ({ ...old, state })), at, 'tree.state', agent, null, { tree: treeId, state })
  }
  const closed = replaceTree(book, treeId, old => ({ ...old, state, closedAt: at, reportFile }))
  return queued(closed, at, 'tree.close', agent, null, { tree: treeId, state, report_file: reportFile })
}

export function pendingIn(book: RexBook, spawn: RexPendingSpawn): RexBook {
  return { ...book, pendingSpawns: [...book.pendingSpawns.filter(old => old.description !== spawn.description), spawn] }
}

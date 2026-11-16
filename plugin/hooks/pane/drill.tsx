// The tree view of the watch pane: a breadcrumb and a level rail over the node in view, its children as rows that each go one level down, then its waiting questions and facts.
// The rows come before the facts, so a short inline pane still shows the way down.
// Keys: a digit goes down to that row, `u` goes one level up, `n` shows the next nine rows; each breadcrumb part goes back to its level.
import type { RexBook, RexJob, RexRun, RexTree } from '../../types'
import type { PaneElements, PaneView, PaneViewContext } from '../parts'
import { cut, duration, labelOf, nodeLine, pathTo, railLevels, usd } from './outline'
import type { Level, OutlineNode } from './outline'

type Drawn = ReturnType<PaneElements['Box']>

/** Enough of `sha256:<hex>` to tell two policies apart at a glance. */
const HASH_SHOWN = 15
/** A digit reaches one row, so a page holds nine; `n` turns to the next. */
export const PAGE_ROWS = 9
/** A plain Button draws its hotkey and a colon before its label: `1: `. */
const HOTKEY_CELLS = 3
const CRUMB_GAP = ' › '

function tokenCount(run: RexRun): number {
  return run.tokens.input + run.tokens.output + run.tokens.cacheRead + run.tokens.cacheCreation
}

function listed(items: readonly string[]): string {
  return items.length === 0 ? 'none' : items.join(', ')
}

function treeFacts(book: RexBook, tree: RexTree, node: OutlineNode): readonly string[] {
  return [
    `state: ${tree.state} · master ${tree.master ?? 'none'} · ran ${duration(node.ranMs ?? 0)}`,
    ...tree.charter.split('\n').filter(line => line.trim() !== '').map((line, index) => `${index === 0 ? 'charter: ' : '         '}${line.trim()}`),
    `spend: ${usd(node.spentNano)} of ${usd(tree.capNano)} (master ${usd(tree.ledgerNano)})`,
    `policy: ${tree.policyHash.slice(0, HASH_SHOWN)}`,
    ...tree.notes.map(note => `note: ${note.job === null ? '' : `${note.job}: `}${note.text}`),
    `report: ${tree.reportFile ?? 'not written'}`,
    ...(book.jobs.some(job => job.tree === tree.id) ? [] : ['No job built yet.']),
  ]
}

function phaseOf(job: RexJob): string {
  if (job.status !== 'created' && job.status !== 'planned') {
    return job.status
  }
  if (job.plan === null) {
    return 'planning'
  }
  return job.deliveries.length === 0 ? 'building' : 'delivering'
}

function jobFacts(book: RexBook, job: RexJob): readonly string[] {
  const failed = job.doneChecks.filter(check => check.result === 'fail').map(check => check.check)
  const checks = job.doneChecks.length === 0 ? 'none' : failed.length === 0 ? 'passed' : `failed ${failed.join(', ')}`
  const builder = job.owner.builder === 'main' ? 'the main session' : job.owner.builder
  return [
    `status: ${job.status} · phase ${phaseOf(job)} · builder ${builder}`,
    `request: ${job.request}`,
    `orchestrator: ${job.plan?.orchestrator ?? 'no plan yet'}`,
    `merge: ${job.merge === null ? 'not answered' : `${job.merge.answer} (${job.merge.source})`}${job.mergeAfter.length === 0 ? '' : ` · after ${job.mergeAfter.join(', ')}`}`,
    `spend: ${usd(job.spentNano)} of ${usd(job.limit.nano)} (${job.limit.source})`,
    `worktree: ${job.worktree ?? 'none'} · branch ${job.branch} onto ${job.base}`,
    ...job.deliveries.map(delivery => `delivery: ${delivery.outcome} ${delivery.repo}${delivery.mergeCommit === null ? '' : ` ${delivery.mergeCommit.slice(0, 12)}`}${delivery.note === null ? '' : ` · ${delivery.note}`}`),
    `done checks: ${checks}`,
    `degraded: ${listed(job.degraded)}`,
    `decisions: ${String(job.decisions.length)}, in the decisions tab`,
    `report: ${job.outputFile ?? 'not written'}`,
    ...(book.runs.some(run => run.job === job.id) ? [] : ['No agent started yet.']),
  ]
}

function agentFacts(run: RexRun, node: OutlineNode): readonly string[] {
  const ran = run.ran === null ? 'not run yet' : `${run.ran.model} at ${run.ran.effort ?? 'no'} effort`
  const lint = run.lint === null ? 'none' : `${run.lint}${run.lintFindings.length === 0 ? '' : ` (${String(run.lintFindings.length)} findings)`}`
  return [
    `agent: ${run.agentId} · role ${run.role ?? 'none'} · task ${run.task ?? 'none'}`,
    `grid agent: planned ${run.planned ?? 'none'} · ran ${ran}`,
    `state: ${node.status} · ${duration(node.ranMs ?? 0)} · ${String(tokenCount(run))} tokens · ${usd(run.spentNano)}`,
    `brief check: ${lint}${run.marks.length === 0 ? '' : ` · marks ${run.marks.join(', ')}`}`,
    `skills given: ${listed(run.skillsAttached)}`,
    `skills opened: ${listed(run.skillsOpened)}`,
    `report: ${run.outputFile ?? 'none'}`,
  ]
}

function sessionFacts(node: OutlineNode): readonly string[] {
  return node.children.length === 0 ? ['Nothing runs yet. Start a job with /super-rex:orchestration-builder, or a tree with /rex master <goal>.'] : []
}

export function factsOf(book: RexBook, node: OutlineNode): readonly string[] {
  switch (node.kind) {
    case 'tree': {
      const tree = book.trees.find(each => each.id === node.ref)
      return tree === undefined ? [] : treeFacts(book, tree, node)
    }
    case 'job': {
      const job = book.jobs.find(each => each.id === node.ref)
      return job === undefined ? [] : jobFacts(book, job)
    }
    case 'agent': {
      const run = book.runs.find(each => each.agentId === node.ref)
      return run === undefined ? [] : agentFacts(run, node)
    }
    case 'session':
      return sessionFacts(node)
  }
}

/** How deep the node in view is, and the levels of its run in order with the current one in brackets: `level 2 of 4 · master › [builder] › lead › worker`. */
export function railLine(path: readonly OutlineNode[]): string {
  const here = path[path.length - 1]
  const top = path[1] ?? path[0]
  if (here === undefined || top === undefined) {
    return ''
  }
  const levels = railLevels(top)
  if (here.kind === 'session') {
    return `the session, above every level · ${levels.join(' › ')}`
  }
  const depth = levels.indexOf(here.level) + 1
  return `level ${String(depth)} of ${String(levels.length)} · ${levels.map(level => (level === here.level ? `[${level}]` : level)).join(' › ')}`
}

function crumbLabel(node: OutlineNode, isShort: boolean): string {
  if (node.kind === 'session') {
    return 'session'
  }
  return isShort ? node.level : labelOf(node)
}

/** The labels of the breadcrumb; when the whole path is wider than the pane, the parts above the node in view shrink to their level. */
export function crumbLabels(path: readonly OutlineNode[], columns: number): readonly string[] {
  const full = path.map(node => crumbLabel(node, false))
  if (full.join(CRUMB_GAP).length <= columns) {
    return full
  }
  const short = path.map((node, index) => crumbLabel(node, index < path.length - 1))
  const before = short.slice(0, -1).join(CRUMB_GAP)
  return [...short.slice(0, -1), cut(short[short.length - 1] ?? '', Math.max(1, columns - before.length - CRUMB_GAP.length))]
}

function levelsOf(nodes: readonly OutlineNode[]): string {
  const levels: Level[] = []
  for (const node of nodes) {
    if (!levels.includes(node.level)) {
      levels.push(node.level)
    }
  }
  return levels.join(' or ')
}

function treeView({ elements, book, columns, actions, at, page, outline }: PaneViewContext): Drawn {
  const { Box, Text, Button } = elements
  const path = pathTo(outline, at)
  const here = path[path.length - 1]
  const parent = path[path.length - 2]
  if (here === undefined) {
    return <Text dimColor>No session yet.</Text>
  }
  const labels = crumbLabels(path, columns)
  const pages = Math.max(1, Math.ceil(here.children.length / PAGE_ROWS))
  const shownPage = Math.min(page, pages - 1)
  const first = shownPage * PAGE_ROWS
  const rows = here.children.slice(first, first + PAGE_ROWS)
  const nextPage = (shownPage + 1) % pages
  return (
    <Box flexDirection="column">
      <Box flexDirection="row">
        {path.map((node, index) => {
          const label = labels[index] ?? ''
          const gap = index === 0 ? '' : CRUMB_GAP
          return node === here ? (
            <Text key={`crumb:${node.id}`} bold color="suggestion">
              {gap}
              {label}
            </Text>
          ) : (
            <Box key={`crumb:${node.id}`} flexDirection="row">
              <Text dimColor>{gap}</Text>
              <Button key={`crumb:${node.id}`} plain dimColor label={label} onPress={() => { actions.go(node.id) }} />
            </Box>
          )
        })}
      </Box>
      <Text bold>{railLine(path)}</Text>
      <Text bold>{cut(nodeLine(here), columns)}</Text>
      {parent !== undefined && <Button key="up" plain hotkey="u" label={cut(`up to ${crumbLabel(parent, false)}`, columns - HOTKEY_CELLS)} onPress={() => { actions.go(parent.id) }} />}
      {rows.length > 0 && <Text dimColor>{cut(`one level down, ${levelsOf(rows)}: press ${rows.length === 1 ? '1' : `1-${String(rows.length)}`}${pages > 1 ? ` · rows ${String(first + 1)}-${String(first + rows.length)} of ${String(here.children.length)}` : ''}`, columns)}</Text>}
      {rows.map((child, index) => (
        <Button key={`go:${child.id}`} plain hotkey={String(index + 1)} label={cut(nodeLine(child), columns - HOTKEY_CELLS)} onPress={() => { actions.go(child.id) }} />
      ))}
      {pages > 1 && <Button key="more" plain hotkey="n" label={nextPage === 0 ? 'back to rows 1-9' : `next rows ${String(nextPage * PAGE_ROWS + 1)}-${String(Math.min(here.children.length, (nextPage + 1) * PAGE_ROWS))}`} onPress={() => { actions.turn(nextPage) }} />}
      {here.questions.map(question => (
        <Box flexDirection="column">
          <Text color="warning">? {question.question}</Text>
          <Text dimColor>
            {question.job === null ? `for tree ${question.tree ?? 'none'}` : `for job ${question.job}`} · recommended: {question.recommendation} · press q to answer, or type /rex answer {question.id} &lt;1-{String(question.options.length)}&gt;
          </Text>
        </Box>
      ))}
      {factsOf(book, here).map(fact => (
        <Text>{fact}</Text>
      ))}
    </Box>
  )
}

export const TREE_VIEW: PaneView = { name: 'tree', title: 'Tree', order: 20, draw: treeView }

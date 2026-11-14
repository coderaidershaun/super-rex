// Every failure the mod expects, each with the one class that decides what happens next, so the same failure gets the same reaction in every module.

export type FailureClass = 'halt' | 'reset' | 'retry' | 'reject'

/** A failure as a value: its kind and the values the code saw when it failed. */
export type RexFailure =
  | { readonly kind: 'policy-changed'; readonly tree: string; readonly pinned: string; readonly current: string }
  | { readonly kind: 'ingest-conflict'; readonly seq: number | null; readonly stderr: string }
  | { readonly kind: 'ingest-no-store'; readonly stderr: string }
  | { readonly kind: 'config-unreadable'; readonly reason: string }
  | { readonly kind: 'merge-outside-rex'; readonly job: string; readonly commit: string; readonly base: string }
  | { readonly kind: 'migrate-unknown-version'; readonly version: number }
  | { readonly kind: 'ingest-wrong-secret'; readonly stream: string; readonly stderr: string }
  | { readonly kind: 'ingest-halted'; readonly reason: string }
  | { readonly kind: 'builder-stuck'; readonly job: string; readonly agentId: string; readonly wakes: number; readonly isLimit: boolean }
  | { readonly kind: 'builder-dead-on-resume'; readonly job: string; readonly agentId: string }
  | { readonly kind: 'ingest-failed'; readonly reason: string }
  | { readonly kind: 'builder-api-error'; readonly job: string; readonly agentId: string; readonly tries: number; readonly isLimit: boolean }
  | { readonly kind: 'merge-answer-missing'; readonly job: string }
  | { readonly kind: 'authority-check-failed'; readonly check: string; readonly error: string }
  | { readonly kind: 'origin-not-person'; readonly action: string; readonly origin: string }
  | { readonly kind: 'caller-not-allowed'; readonly tool: string; readonly caller: string }
  | { readonly kind: 'bad-argument'; readonly tool: string; readonly argument: string; readonly value: string }
  | { readonly kind: 'spend-limit-reached'; readonly job: string; readonly spentNano: number; readonly limitNano: number }
  | { readonly kind: 'depth'; readonly depth: number }
  | { readonly kind: 'fork'; readonly job: string }
  | { readonly kind: 'tree-ledger'; readonly tree: string; readonly spentNano: number; readonly capNano: number }
  | { readonly kind: 'rate-limit-pause'; readonly percent: number; readonly window: string }
  | { readonly kind: 'lint-findings'; readonly findings: readonly string[] }
  | { readonly kind: 'merge-push-rejected'; readonly exitCode: number; readonly stderr: string }
  | { readonly kind: 'merge-lock-held'; readonly repo: string; readonly job: string }
  | { readonly kind: 'merge-not-fast-forward'; readonly branch: string; readonly base: string }
  | { readonly kind: 'merge-path-asks'; readonly paths: readonly string[] }
  | { readonly kind: 'binary-read-failed'; readonly argv: readonly string[]; readonly exitCode: number; readonly stderr: string }
  | { readonly kind: 'preflight-failed'; readonly step: number; readonly text: string }
  | { readonly kind: 'policy-missing'; readonly repo: string }
  | { readonly kind: 'observer-failed'; readonly event: string; readonly module: string; readonly agentId: string | null; readonly error: string }
  | { readonly kind: 'restatement-failed'; readonly reason: string }
  | { readonly kind: 'gate-check-failed'; readonly check: string; readonly job: string | null; readonly error: string }
  | { readonly kind: 'question-unanswered-headless'; readonly job: string; readonly question: string }

export type RexFailureKind = RexFailure['kind']

export function failureClass(kind: RexFailureKind): FailureClass {
  switch (kind) {
    case 'policy-changed':
    case 'ingest-conflict':
    case 'ingest-no-store':
    case 'config-unreadable':
    case 'merge-outside-rex':
    case 'migrate-unknown-version':
    case 'ingest-halted':
    case 'question-unanswered-headless':
      return 'halt'
    case 'ingest-wrong-secret':
    case 'builder-stuck':
    case 'builder-dead-on-resume':
      return 'reset'
    case 'ingest-failed':
    case 'builder-api-error':
      return 'retry'
    case 'merge-answer-missing':
    case 'authority-check-failed':
    case 'origin-not-person':
    case 'caller-not-allowed':
    case 'bad-argument':
    case 'spend-limit-reached':
    case 'depth':
    case 'fork':
    case 'tree-ledger':
    case 'rate-limit-pause':
    case 'lint-findings':
    case 'merge-push-rejected':
    case 'merge-lock-held':
    case 'merge-not-fast-forward':
    case 'merge-path-asks':
    case 'binary-read-failed':
    case 'preflight-failed':
    case 'policy-missing':
    case 'observer-failed':
    case 'restatement-failed':
    case 'gate-check-failed':
      return 'reject'
    default:
      return assertNever(kind)
  }
}

export function assertNever(value: never): never {
  throw new Error(`unhandled value: ${JSON.stringify(value)}`)
}

/** What an error that a hook caught says about itself, for a mark or a status line. */
export function errorText(error: unknown): string {
  if (error instanceof Error) {
    return `${error.name}: ${error.message}`
  }
  return String(error)
}

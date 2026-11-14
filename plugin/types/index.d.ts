// The live book of the super-rex mod as Claude Code keeps it for the session: every module reads it, and only the book module writes it.
// Its shape is the contract between the modules, so a field is added here before any module relies on it.

/** JSON data, as `$.state` and `$.store` keep it. */
export type RexJson = string | number | boolean | null | readonly RexJson[] | { readonly [key: string]: RexJson }

/** Who started a job: the person by prompt origin, or a master tree. */
export type RexOriginKind = 'composer' | 'bridge' | 'sdk' | 'master'

export type RexLevel = 'low' | 'medium' | 'high'

/** A role rating, as `rex oms` takes it. */
export type RexRating = RexLevel | 'none'

export type RexJobStatus = 'created' | 'planned' | 'blocked' | 'failed' | 'delivered' | 'cancelled' | 'lost'

/** Token counts of one request, a run or a job; each a safe integer. */
export type RexTokens = {
  readonly input: number
  readonly output: number
  readonly cacheRead: number
  readonly cacheCreation: number
}

/** The key that a job's open-job rule is about: one open job for each pair. `builder` is `main` or an agent id. */
export type RexOwner = {
  readonly session: string
  readonly builder: string
}

/** The latest merge answer of a job. A person's answer replaces an earlier one; `question` names the parked question it answered. */
export type RexMergeAnswer = {
  readonly answer: 'yes' | 'no'
  readonly source: 'dialog' | 'person-prompt' | 'sdk-prompt' | 'question' | 'policy'
  readonly question: string | null
  readonly policyHash: string | null
  readonly at: number
}

/** A spend limit in nanodollars: a safe integer, so at most 9,000,000 dollars, which every setter refuses to pass. */
export type RexLimit = {
  readonly nano: number
  readonly source: 'default' | 'set' | 'policy'
}

export type RexOpenWhen = {
  readonly skill: string
  readonly file: string | null
  readonly when: string
}

export type RexTask = {
  readonly id: string
  readonly role: string
  readonly skills: readonly string[]
  readonly openWhen: readonly RexOpenWhen[]
}

export type RexRatings = {
  readonly intelligence: RexRating
  readonly reliability: RexRating
  readonly consequence: RexRating
  readonly verification: RexRating
}

/** A plan role and the grid agent `rex oms` gave it, without the plugin prefix. */
export type RexRole = {
  readonly role: string
  readonly ratings: RexRatings
  readonly gridAgent: string
  readonly conflict: string | null
  readonly omsVersion: number
}

export type RexSkillSource = 'main' | 'plugin' | 'user' | 'worktree'

/** One skill file a plan names, with the content hash it had when the plan was recorded. */
export type RexSkillFile = {
  readonly skill: string
  readonly file: string
  readonly hash: string
  readonly source: RexSkillSource
}

export type RexPlan = {
  readonly orchestrator: string
  readonly breakdown: string
  readonly tasks: readonly RexTask[]
  readonly roles: readonly RexRole[]
  readonly manifest: readonly RexSkillFile[]
}

/** Why a skill or a rating is on a task or role: a table rule, the decision agent's answer, an override, or the plan as written. */
export type RexDecision = {
  readonly assignmentId: string
  readonly subject: string
  readonly item: string
  readonly value: string
  readonly source: 'rule' | 'decision-agent' | 'override' | 'plan'
  readonly reason: string
  readonly tableVersion: string | null
  readonly overlayVersion: string | null
}

export type RexDoneCheck = {
  readonly check: 'fetched' | 'merged' | 'base_current' | 'branches_gone' | 'worktrees_gone' | 'output_exists' | 'checks_on_base'
  readonly result: 'pass' | 'fail' | 'not-set'
  readonly at: number
}

export type RexDelivery = {
  readonly repo: string
  readonly branch: string | null
  readonly pr: string | null
  readonly outcome: 'merged' | 'pr-open' | 'kept-local' | 'blocked'
  readonly mergeCommit: string | null
  readonly note: string | null
}

export type RexJob = {
  readonly id: string
  readonly owner: RexOwner
  readonly tree: string | null
  readonly origin: RexOriginKind
  readonly request: string
  readonly restatement: string | null
  /** Where the job's work lives and lands, fixed when the job opens: the mod never takes them from a tool argument. */
  readonly repo: string
  readonly branch: string
  readonly base: string
  readonly worktree: string | null
  readonly type: 'feature' | 'fix' | 'refactor' | 'chore' | 'research' | 'other' | null
  readonly size: 'small' | 'medium' | 'large' | null
  readonly consequence: RexLevel | null
  readonly langs: readonly string[]
  readonly prefs: {
    readonly duration: RexLevel | null
    readonly budget: RexLevel | null
    readonly autonomy: RexLevel | null
  }
  readonly status: RexJobStatus
  readonly outcome: string | null
  readonly merge: RexMergeAnswer | null
  readonly mergeQuestion: string | null
  readonly limit: RexLimit
  readonly spentNano: number
  readonly tokens: RexTokens
  readonly plan: RexPlan | null
  readonly decisions: readonly RexDecision[]
  readonly deliveries: readonly RexDelivery[]
  readonly degraded: readonly string[]
  readonly dependsOn: readonly string[]
  readonly mergeAfter: readonly string[]
  readonly outputFile: string | null
  readonly retries: number
  readonly stuckWakes: number
  readonly doneChecks: readonly RexDoneCheck[]
  readonly createdAt: number
  readonly updatedAt: number
}

/** One agent of the session. `parent` is `main` or an agent id; `parentSource` says whether the engine or the mod gave it. */
export type RexRun = {
  readonly agentId: string
  readonly toolUseId: string | null
  readonly job: string | null
  /** Set on a master's own run: its usage goes to the tree's ledger and to no job. */
  readonly tree: string | null
  readonly parent: string
  readonly parentSource: 'engine' | 'mod'
  readonly depth: number
  readonly role: string | null
  readonly task: string | null
  readonly agentType: string
  readonly planned: string | null
  readonly ran: { readonly model: string; readonly effort: string | null } | null
  readonly marks: readonly string[]
  readonly state: 'open' | 'done' | 'lost'
  readonly endReason: 'answer' | 'aborted' | 'refusal' | 'error' | null
  readonly startedAt: number
  readonly endedAt: number | null
  readonly tokens: RexTokens
  readonly spentNano: number
  readonly briefHash: string | null
  readonly assembled: 'yes' | 'no' | 'late'
  readonly lint: 'pass' | 'passed-marked' | null
  readonly lintFindings: readonly string[]
  readonly skillsAttached: readonly string[]
  readonly skillsOpened: readonly string[]
  readonly answerChars: number | null
  readonly outputFile: string | null
}

/** A brief that lint sent back once; the key is parent, job and task. */
export type RexLintAttempt = {
  readonly key: string
  readonly findings: readonly string[]
  readonly at: number
}

export type RexAnswer = {
  readonly option: string
  readonly source: 'dialog' | 'pane' | 'command'
  readonly at: number
}

export type RexQuestion = {
  readonly id: string
  readonly tree: string | null
  readonly job: string | null
  readonly question: string
  readonly options: readonly string[]
  readonly recommendation: string
  readonly category: 'merge' | 'scope' | 'spend' | 'other'
  /** The job whose merge answer this question decides; its answer becomes that answer. */
  readonly mergeFor: string | null
  readonly askedAt: number
  readonly answer: RexAnswer | null
}

export type RexTreeBuilder = {
  readonly job: string
  readonly agentId: string
}

export type RexTreeTick = {
  readonly at: number
  readonly job: string
  readonly spentNano: number
}

export type RexTreeNote = {
  readonly at: number
  readonly job: string | null
  readonly text: string
}

/** A master and the jobs it started, with its charter as the person wrote it. */
export type RexTree = {
  readonly id: string
  readonly session: string
  readonly master: string | null
  readonly charter: string
  readonly charterHash: string
  readonly policyHash: string
  readonly state: 'running' | 'paused' | 'blocked' | 'done' | 'lost'
  readonly ledgerNano: number
  readonly capNano: number
  readonly builders: readonly RexTreeBuilder[]
  readonly ticks: readonly RexTreeTick[]
  readonly notes: readonly RexTreeNote[]
  readonly startedAt: number
  readonly closedAt: number | null
  readonly reportFile: string | null
}

/** A spawn the mod is about to make, so the run that its `agent.spawn` raises is known before `$.agent.spawn` resolves. */
export type RexPendingSpawn = {
  readonly description: string
  readonly kind: 'master' | 'builder'
  readonly tree: string
  readonly job: string | null
  readonly at: number
}

/** One event waiting for `rex ingest`, in the form of the shared batch schema. */
export type RexEvent = {
  readonly seq: number
  readonly at: string
  readonly kind: string
  readonly agent: string
  readonly job: string | null
  readonly data: { readonly [key: string]: RexJson }
}

/** The stream of this load of the mod to `rex ingest`; `halted` holds the reason once ingest stopped. */
export type RexIngest = {
  readonly stream: string
  readonly secret: string
  readonly nextSequence: number
  readonly acked: number
  readonly failures: number
  readonly resets: number
  readonly halted: string | null
  readonly dropped: number
  readonly queue: readonly RexEvent[]
}

/** The person's prompt that may open a job, until a job opens or another prompt replaces it. */
export type RexJobStartPrompt = {
  readonly origin: RexOriginKind
  readonly text: string
  readonly restatement: string | null
  readonly restatementSkipped: string | null
  readonly at: number
}

/** Versions the pane header shows; `binary` and `claudeCode` stay null until read (`calls/version.json`, `$.session.version()`). */
export type RexVersions = {
  readonly mod: string
  readonly binary: string | null
  readonly claudeCode: string | null
}

export type RexBook = {
  readonly session: string
  readonly versions: RexVersions
  /** Permission asks seen at `tool.check` this session (decision `ask`), for the band. */
  readonly permissionAsks: number
  readonly inert: string | null
  readonly storage: 'on' | 'off'
  readonly prompts: 'off' | 'observe' | 'improve'
  readonly jobsFromSdk: boolean
  readonly jobStartPrompt: RexJobStartPrompt | null
  readonly jobs: readonly RexJob[]
  readonly runs: readonly RexRun[]
  readonly lintAttempts: readonly RexLintAttempt[]
  readonly questions: readonly RexQuestion[]
  readonly trees: readonly RexTree[]
  readonly pendingSpawns: readonly RexPendingSpawn[]
  readonly ingest: RexIngest | null
}

declare module 'claude-code' {
  interface PluginState {
    'super-rex': {
      book: Shaped<RexBook>
      view: string
    }
  }
}

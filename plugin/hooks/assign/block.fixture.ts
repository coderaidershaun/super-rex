// The tasks of the decision block of job 0ganyb6d6v (decision-maker-2.md), each with the Skills and Open when the decision agent gave it by hand under 0.7's table.
// The facts beside them (role, writes code, languages) come from the same block: its role table and each task's Owns column.
export type BlockTask = {
  readonly id: string
  readonly role: string
  readonly writesCode: boolean
  readonly langs: readonly string[]
  readonly skills: readonly string[]
  readonly openWhen: readonly string[]
}

export const BLOCK_TASKS: readonly BlockTask[] = [
  { id: 'T1', role: 'architect', writesCode: true, langs: ['ts', 'rust'], skills: ['rex-code-spike', 'rex-code-design-sketch', 'rex-code-validation-plan', 'rex-code-failure-policy', 'rex-code-philosophy'], openWhen: ['rex-code-failure-policy/failure-table.md', 'rex-code-priorities', 'rex-code-priorities/write.md', 'rex-code-folder-management', 'rex-mermaid-diagrams', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts', 'rex-gap-ts/setup.md', 'rex-code-external-libraries', 'rex-code-commenting'] },
  { id: 'T2', role: 'guard-engineer', writesCode: true, langs: ['rust'], skills: ['rex-code-philosophy', 'rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-code-tdd'], openWhen: ['rex-code-tests-contract-seams', 'rex-code-failure-policy', 'rex-code-tests-unit-testing', 'rex-code-tests-fitness-functions', 'rex-code-tests-integration-testing', 'rex-code-tdd/mocking.md', 'rex-code-folder-management', 'rex-code-folder-management/move.md', 'rex-code-external-libraries', 'rex-code-smells'] },
  { id: 'T3', role: 'author', writesCode: false, langs: [], skills: ['rex-skill-language'], openWhen: ['rex-agent-prompting'] },
  { id: 'T4', role: 'engineer', writesCode: true, langs: ['ts'], skills: ['rex-code-philosophy', 'rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts', 'rex-code-tdd'], openWhen: ['rex-code-failure-policy', 'rex-agent-prompting', 'rex-agent-prompting/sonnet.md', 'rex-skill-language', 'rex-code-tests-unit-testing', 'rex-code-tests-contract-seams', 'rex-code-tests-fitness-functions', 'rex-code-tdd/mocking.md', 'rex-code-folder-management', 'rex-code-external-libraries'] },
  { id: 'T5', role: 'guard-engineer', writesCode: true, langs: ['ts'], skills: ['rex-code-philosophy', 'rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts', 'rex-code-tdd'], openWhen: ['rex-code-failure-policy', 'rex-agent-prompting', 'rex-code-tests-unit-testing', 'rex-code-tests-contract-seams', 'rex-code-tests-fitness-functions', 'rex-code-tests-integration-testing', 'rex-code-tdd/mocking.md', 'rex-code-folder-management', 'rex-code-external-libraries'] },
  { id: 'T6', role: 'engineer', writesCode: true, langs: ['ts'], skills: ['rex-code-philosophy', 'rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts', 'rex-code-tdd'], openWhen: ['rex-skill-language', 'rex-agent-prompting', 'rex-code-tests-unit-testing', 'rex-code-tests-fitness-functions', 'rex-code-tests-contract-seams', 'rex-code-tdd/mocking.md', 'rex-code-failure-policy', 'rex-code-folder-management'] },
  { id: 'T7', role: 'guard-engineer', writesCode: true, langs: ['ts'], skills: ['rex-code-philosophy', 'rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts', 'rex-code-tdd'], openWhen: ['rex-code-failure-policy', 'rex-skill-language', 'rex-agent-prompting', 'rex-code-tests-unit-testing', 'rex-code-tests-contract-seams', 'rex-code-tests-fitness-functions', 'rex-code-tests-integration-testing', 'rex-code-tdd/mocking.md', 'rex-code-folder-management'] },
  { id: 'T8', role: 'architect', writesCode: true, langs: ['ts'], skills: ['rex-code-validation-plan', 'rex-code-tests-integration-testing', 'rex-code-philosophy'], openWhen: ['rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts'] },
  { id: 'T9', role: 'author', writesCode: false, langs: [], skills: [], openWhen: ['rex-mermaid-diagrams'] },
  { id: 'T10', role: 'optimiser', writesCode: true, langs: ['rust', 'ts'], skills: ['rex-code-philosophy', 'rex-code-commenting', 'rex-code-ergonomics', 'rex-code-error-writing', 'rex-gap-ts', 'rex-code-smells'], openWhen: ['rex-code-improve-codebase-architecture', 'rex-code-tdd', 'rex-code-tests-unit-testing', 'rex-code-failure-policy', 'rex-code-priorities', 'rex-skill-language', 'rex-agent-prompting', 'rex-code-folder-management'] },
]

# TypeScript Gap — Setup

Exact compiler + lint settings for rule 1 of `rex-gap-ts`. Each TypeScript package in repo uses all of them.

## Compiler

`tsconfig.json` has these `compilerOptions`. Never set one of them to a weaker value.

```json
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noImplicitOverride": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "noPropertyAccessFromIndexSignature": true,
    "useUnknownInCatchVariables": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "allowUnreachableCode": false,
    "forceConsistentCasingInFileNames": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "skipLibCheck": false,
    "target": "ES2022"
  }
}
```

| Flag | Rust protection it gives |
| --- | --- |
| `strict` | Null safety, no implicit `any`, sound function types. |
| `noUncheckedIndexedAccess` | `v.get(i)` gives `Option`: `arr[i]` gives `T \| undefined`. |
| `exactOptionalPropertyTypes` | Absent field and `None` value are different. |
| `noImplicitReturns` | Each branch returns a value. |
| `noFallthroughCasesInSwitch` | No silent fall from one `match` arm to next. |
| `noPropertyAccessFromIndexSignature` | Map lookup looks different from field access. |
| `useUnknownInCatchVariables` | Caught error has type `unknown`, not `any`. |
| `noUnusedLocals`, `noUnusedParameters` | Compiler warns on unused names. |
| `target: ES2022` | `bigint` literals and `Error` `cause` exist. |

## Lint

Use `typescript-eslint` with type information (`strictTypeChecked` + `stylisticTypeChecked` presets). These rules stay at `error`:

- [ ] `@typescript-eslint/no-explicit-any`
- [ ] `@typescript-eslint/no-unsafe-assignment`, `no-unsafe-member-access`, `no-unsafe-call`, `no-unsafe-return`, `no-unsafe-argument`
- [ ] `@typescript-eslint/no-non-null-assertion`
- [ ] `@typescript-eslint/consistent-type-assertions` with `assertionStyle: "as"`, `objectLiteralTypeAssertions: "never"`
- [ ] `@typescript-eslint/switch-exhaustiveness-check`
- [ ] `@typescript-eslint/no-floating-promises`, `no-misused-promises`
- [ ] `@typescript-eslint/only-throw-error`
- [ ] `@typescript-eslint/prefer-readonly`, `prefer-readonly-parameter-types` (IF it gives too much noise on library types, THEN keep `prefer-readonly` only)
- [ ] `@typescript-eslint/strict-boolean-expressions`
- [ ] `@typescript-eslint/restrict-template-expressions`, `restrict-plus-operands`
- [ ] `@typescript-eslint/prefer-nullish-coalescing`
- [ ] `@typescript-eslint/ban-ts-comment` (lets `@ts-expect-error` only with description)
- [ ] `eqeqeq` set to `always`
- [ ] `no-empty` (blocks empty `catch`)
- [ ] `radix`

IF you must turn off a rule for one line, THEN use `// eslint-disable-next-line <rule> -- <reason>` + `SMELL` flag comment. Never turn off a rule for a whole file.

## Gate

Type check + lint run in repo checks (git hooks), same as Rust checks. Zero errors to commit.

---
name: rex-gap-ts
description: >
  Gap skill for TypeScript: each protection that Rust gives by default and
  TypeScript does not, written as a hard rule with Rust and TypeScript
  examples side by side.
when_to_use: >
  Use when you write, change, or review TypeScript code, a tsconfig, or lint
  settings, or when a Rust code rulebook tells you to obey the gap skill of
  TypeScript.
---

# TypeScript Gap — Hard Rules

Rust code rulebooks (`rex-code-*`) still apply to TypeScript. Apply intent of each rule in TypeScript idiom. This skill adds what Rust compiler gives free and TypeScript compiler does not.

Each rule below closes one gap. IF a rule and a Rust rulebook disagree, THEN this skill wins for TypeScript.

## Files of this skill

| Read | When |
| --- | --- |
| `rex-gap-ts/setup.md` | IF you create or change `tsconfig.json`, lint config, or a package of TypeScript code, THEN read it. Exact compiler + lint settings. |

## Words

| Term | Meaning |
| --- | --- |
| Gap | Protection that Rust gives by default and TypeScript does not. |
| Unchecked cast | `as T`, `<T>x`, or `x!` on value that code did not prove is `T`. |
| Loose promise | Promise that code does not `await`, return, or give `.catch`. |
| Boundary | Place where data comes in from outside: JSON, file, env, network, process. |
| Safe integer | Integer with absolute value of max `2^53 - 1` (`Number.MAX_SAFE_INTEGER`). |

## Rules

- [ ] 1. Strict compiler. Obey `setup.md`. Never turn off a strict flag.
- [ ] 2. No `any`. No unchecked cast.
- [ ] 3. `null` + `undefined` handled by type.
- [ ] 4. Exhaustive `switch` on union, with `never` check.
- [ ] 5. Errors = typed values or typed classes with context. No silent `catch`.
- [ ] 6. No loose promise.
- [ ] 7. `readonly` data by default.
- [ ] 8. No implicit coercion.
- [ ] 9. Integer that can pass safe integer = `bigint` or text.

### 1. Strict compiler

Rust: compiler refuses unsound code. No flag turns this off.
TypeScript: default compiler lets many unsound forms pass.

- [ ] Use every flag in `setup.md`.
- [ ] Type check = `tsc --noEmit` with zero errors. Lint = zero errors.
- [ ] Never write `// @ts-ignore` or `// @ts-nocheck`.
- [ ] IF a type error is wrong and you cannot fix type, THEN write `// @ts-expect-error <reason>`. Add `SMELL` flag comment.

### 2. No `any`, no unchecked cast

Rust: no type `any`. Cast of unknown data fails at parse.

```rust
let cfg: Config = serde_json::from_str(text)?;
```

TypeScript: `JSON.parse` gives `any`. `as` trusts you.

```ts
// Not
const cfg = JSON.parse(text) as Config;
// Yes
const cfg: Result<Config, ParseError> = parseConfig(JSON.parse(text) as unknown);
```

- [ ] Data at boundary = `unknown`. Validate it with schema (for example `zod`) or type guard. Then use typed value.
- [ ] No `as T` on unknown data. `as const` is OK.
- [ ] No non-null assertion `x!`. Prove value with `if` or narrow it.
- [ ] Type guard (`x is T`) must test each field that type promises.
- [ ] Use `unknown`, not `any`, in generic code.

### 3. `null` and `undefined` by type

Rust: no null. `Option<T>` forces each caller to handle `None`.

```rust
fn find(id: u32) -> Option<User>
match find(7) { Some(u) => use_user(u), None => return Err(NotFound(7)) }
```

TypeScript: `strictNullChecks` gives this only when on. Index access gives `undefined` silently unless `noUncheckedIndexedAccess` is on.

```ts
function find(id: number): User | undefined
const u = find(7);
if (u === undefined) return err({ kind: "NotFound", id: 7 });
useUser(u);
```

- [ ] Return type says `| undefined` (or `| null`) when value can be absent. Pick one per codebase.
- [ ] Optional field = `field?: T` only when absent is valid. With `exactOptionalPropertyTypes`, absent and `undefined` differ.
- [ ] Use `??` for default. Never `||` for default: `||` replaces `0`, `""`, `false`.

### 4. Exhaustive `switch`

Rust: `match` on enum fails to compile when one variant has no arm.

```rust
match shape { Shape::Circle(r) => area_c(r), Shape::Square(s) => s * s }
```

TypeScript: `switch` with a missing case compiles. Add `never` check in `default`.

```ts
type Shape = { kind: "circle"; r: number } | { kind: "square"; s: number };
switch (shape.kind) {
  case "circle": return areaC(shape.r);
  case "square": return shape.s * shape.s;
  default: return assertNever(shape);
}
function assertNever(x: never): never { throw new Error(`unhandled: ${JSON.stringify(x)}`); }
```

- [ ] Model each variant as discriminated union with literal `kind` field. Not `enum`.
- [ ] Each `switch` on union has `default` that calls `assertNever`.
- [ ] Lint rule `switch-exhaustiveness-check` stays on.

### 5. Errors as typed values

Rust: `Result<T, E>` + `?`. Error type is in signature. Caller cannot ignore it.

```rust
#[derive(thiserror::Error, Debug)]
enum LoadError { #[error("read {path}")] Read { path: PathBuf, #[source] source: io::Error } }
fn load(path: &Path) -> Result<Config, LoadError>
```

TypeScript: `throw` is not in signature. `catch (e)` gives `unknown`.

```ts
type LoadError = { kind: "Read"; path: string; cause: unknown };
type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };
function load(path: string): Result<Config, LoadError>
// Or a typed class with context
class ReadError extends Error {
  constructor(readonly path: string, options: { cause: unknown }) {
    super(`read ${path}`, options);
    this.name = "ReadError";
  }
}
```

- [ ] Expected failure = `Result` value with union of typed errors. Caller must handle it.
- [ ] Throw only for bug or at boundary that a library forces. Throw only `Error` subclass. Never throw text or plain object.
- [ ] Each error carries context: what action, which input, `cause`.
- [ ] Each `catch` does one of these: return typed error, throw new typed error with `cause`, or handle case by name. Never empty `catch`. Never `catch` that only logs and continues.
- [ ] In `catch (e)`, `e` is `unknown`. Narrow with `instanceof` before use.

### 6. No loose promise

Rust: unused `Future` does nothing, and compiler warns. Unused `Result` warns (`#[must_use]`).

```rust
send(msg).await?;
```

TypeScript: promise runs without `await`. Its rejection is lost or kills process.

```ts
// Not
send(msg);
// Yes
await send(msg);
void send(msg).catch((e: unknown) => report(new SendError(msg.id, { cause: e })));
```

- [ ] Each promise: `await` it, return it, or give it `.catch` that handles error.
- [ ] Lint rules `no-floating-promises` + `no-misused-promises` stay on.
- [ ] Use `Promise.allSettled` IF one failure must not hide others. ELSE use `Promise.all`.
- [ ] No `async` function as callback where caller ignores promise (`forEach`, event handler) unless it has own `.catch`.

### 7. `readonly` by default

Rust: binding and borrow are immutable unless `mut`.

```rust
fn total(items: &[Item]) -> u64
```

TypeScript: everything is mutable unless you say `readonly`.

```ts
function total(items: readonly Item[]): bigint
type Item = { readonly id: string; readonly qty: number };
```

- [ ] Use `const`. Use `let` only when value changes.
- [ ] Fields of type + parameters of array type = `readonly` unless code mutates them.
- [ ] Use `ReadonlyArray`, `ReadonlyMap`, `ReadonlySet`, `Readonly<T>` for shared data.
- [ ] Mutable state stays in one owner. Owner returns `readonly` view.

### 8. No implicit coercion

Rust: no implicit conversion. `"1" + 1` does not compile. `as` and `From` are explicit.

```rust
let n: u32 = text.parse()?;
```

TypeScript: `==`, `+` on mixed types, truthiness all convert silently.

```ts
const n = Number(text);
if (!Number.isInteger(n)) return err({ kind: "NotInteger", text });
```

- [ ] Use `===` and `!==`. Never `==` or `!=`.
- [ ] Convert with `Number(x)`, `String(x)`, `BigInt(x)`. Then validate (`Number.isNaN`, `Number.isInteger`). Never unary `+x`. Never `parseInt` without radix.
- [ ] `if (x)` only when `x` is `boolean`. ELSE compare explicitly: `x !== undefined`, `x.length > 0`.
- [ ] Template text and `+` take only `string` or `number`. Lint rules `restrict-template-expressions` + `restrict-plus-operands` stay on.

### 9. Large integers

Rust: `u64`, `i64`, `u128` are exact. Overflow panics in debug.

```rust
let id: u64 = 18_446_744_073_709_551_615;
```

TypeScript: `number` is float64. Above safe integer, integers lose digits silently. `JSON.parse` also loses them.

```ts
const id: bigint = 18_446_744_073_709_551_615n;
const idText: string = "18446744073709551615"; // id from JSON stays text
```

- [ ] IF integer can pass safe integer (ids, money in minor units, nanoseconds, byte counts, hashes), THEN use `bigint` or text.
- [ ] At boundary, read such integer as text. Convert with `BigInt(text)`. Never let it pass through `number`.
- [ ] Never mix `bigint` and `number` in math. Convert explicitly.
- [ ] IF value is `number` integer, THEN test `Number.isSafeInteger` at boundary.

## Review gate

Before you finish TypeScript work:

- [ ] `tsc --noEmit` = zero errors.
- [ ] Lint = zero errors, zero disabled rules without `SMELL` flag.
- [ ] Grep finds no `: any`, `as any`, `@ts-ignore`, `@ts-nocheck`, `catch {}`.
- [ ] Lint rule `eqeqeq` finds no `==` or `!=`.

---
name: rex-code-ergonomics
description: Hard rules for ergonomic Rust code. Naming, types, traits, iterators, lifetimes, panics, fn signatures. Bad/good examples throughout. Use when writing or reviewing Rust, designing public APIs, naming types/fns, choosing String vs &str, Result vs panic, generic vs dyn, builder vs struct literal, or user asks "is this idiomatic Rust", "ergonomic API", "make this nicer".
disable-model-invocation: false
user-invocable: true
---

# Rust Ergonomics — Hard Rules

These rules HARD. No compromise. Apply every Rust file.

Sister skills:
- `rex-code-philosophy` — why (complexity-averse, habitability)
- `rex-code-commenting` — no comments unless WHY non-obvious
- `rex-code-error-writing` — thiserror/anyhow rules + Result patterns (read for all error work)

This skill = Rust-specific applications. Errors deferred to error-writing skill.

Default: **caller's life > author's cleverness**. Public API = your gift to future self.

## Names

- **snake_case fns/vars. PascalCase types. SCREAMING_SNAKE consts.** Non-negotiable.
- **No abbreviations.** `usr_cnt` -> `user_count`. `cfg` OK (idiomatic). `ctx` OK. `mgr` no.
- **Boolean = predicate.** `is_active`, `has_token`, `can_retry`. Never `active`, `flag`, `valid`.
- **Type name carry meaning.** `UserId` not `Id`. `OrderTotal` not `Amount`.
- **Verb on fn. Noun on type.** `parse_url` + `Url`. Not `url_parser` + `parse`.

```rust
// bad
fn proc(d: &Vec<u8>, f: bool) -> Result<Vec<u8>, Box<dyn Error>>

// good
fn decompress_payload(data: &[u8], strict: bool) -> Result<Vec<u8>, DecompressError>
```

## Types — newtype the domain

Primitive obsession = bug factory. Wrap domain primitives.

```rust
// bad — mix UserId + OrderId silently
fn ban(user_id: u64, reason: String)
fn refund(order_id: u64)
ban(order_id, refund_reason); // compiles. wrong.

// good — type system catch swap
struct UserId(u64);
struct OrderId(u64);
fn ban(user: UserId, reason: BanReason)
ban(OrderId(42), reason); // compile error. saved.
```

- **Newtype = ~free.** `#[repr(transparent)]` if perf paranoid.
- **`NonZeroU32`, `NonEmpty<T>`** when invariant exist. Push to type, drop runtime check.
- **No `bool` flags in public fn.** Two bools = 4 states = caller confusion. Use enum.

```rust
// bad
fn copy(src: &Path, dst: &Path, overwrite: bool, follow_symlinks: bool)
copy(&a, &b, true, false); // what mean true false?

// good
fn copy(src: &Path, dst: &Path, opts: CopyOpts)
copy(&a, &b, CopyOpts { overwrite: Yes, symlinks: Follow });
```

## String types — pick right one

| Want | Type |
|------|------|
| Borrow, read-only | `&str` |
| Owned, mutable | `String` |
| Read-only, sometimes owned | `Cow<'_, str>` |
| Owned, immutable, cheap clone | `Arc<str>` or `Box<str>` |

- **Public fn args: `&str` not `&String`.** Caller pass either.
- **Return `String` if you create it.** Don't fight lifetimes for fun.
- **`Box<str>` over `String` for stored-once data.** Skip capacity field.

## Errors

**See `rex-code-error-writing` for full rules.** Summary:

- **Lib: `thiserror`. Bin: `anyhow`.** Never `Box<dyn Error>` in lib pub API.
- **`?` everywhere.** `.unwrap()` = test only. `.expect("WHY")` at boundary if must.
- **Panic = bug only.** Bad input → `Result`. Invariant violated → `panic!`.

For variant design, `#[from]`/`#[source]`, error messages, `anyhow::Context`, panic-vs-Result decision table → load `rex-code-error-writing`.

## Function signatures

- **Take `&[T]` not `&Vec<T>`. Take `&str` not `&String`. Take `&Path` not `&PathBuf`.** Caller flexibility.
- **Return owned (`Vec`, `String`) when you allocate.** No lifetime games.
- **`impl Trait` in return for hide-the-iterator.** `-> impl Iterator<Item = User>`.
- **Generic in arg, concrete in return.** `fn read(r: impl Read) -> Vec<u8>`.
- **More than 3 args -> struct.** Builder if many optional.

```rust
// bad
fn render(w: u32, h: u32, fmt: u8, q: u8, alpha: bool, gamma: f32) -> Vec<u8>

// good
struct RenderOpts { width: u32, height: u32, format: Format, quality: u8, alpha: bool, gamma: f32 }
fn render(opts: RenderOpts) -> Vec<u8>
// or builder if half optional
let img = Render::new(800, 600).quality(90).alpha().build();
```

## Methods on types (locality)

Per philosophy: behavior on the thing. Not free fn somewhere.

```rust
// bad — caller hunt
mod user_utils {
    pub fn activate(u: &mut User) { u.active = true; }
}
user_utils::activate(&mut user);

// good — discoverable via .
impl User {
    pub fn activate(&mut self) { self.active = true; }
}
user.activate();  // IDE autocomplete win
```

## Iterators

- **Chain readable. Break clever.** 3-step chain OK. 8-step = extract or for-loop.
- **`collect::<Vec<_>>()` only when need `Vec`.** Often `for_each` or pass iter onward.
- **Avoid `.iter().map().collect().iter().filter()...`** Allocate once.
- **`.unwrap()` in iter chain = sin.** Use `try_fold` / `collect::<Result<_,_>>()`.

```rust
// bad — alloc twice, panic on miss
let names: Vec<String> = users.iter().map(|u| u.name.clone()).collect();
let active: Vec<String> = names.into_iter().filter(|n| !n.is_empty()).collect();

// good — single pass, owned strings only when kept
let active: Vec<String> = users.iter()
    .filter(|u| !u.name.is_empty())
    .map(|u| u.name.clone())
    .collect();
```

## Option / Result combinators

- **`?` first.** Combinators second. Match last resort.
- **`.unwrap_or_default()`, `.unwrap_or_else(|| ...)`, `.ok_or(err)`** > match.
- **`if let Some(x) = ...` for one branch.** `match` for two+ variants.

```rust
// bad
let name = match user.name {
    Some(n) => n,
    None => String::from("anon"),
};

// good
let name = user.name.unwrap_or_else(|| "anon".into());
```

## Lifetimes — minimize

- **Owned > borrowed in struct fields.** Lifetime in struct = viral pain.
- **Elision works 90%.** Don't annotate when compiler infer.
- **`'static` bound only when necessary.** Spreads.
- **`Cow` when sometimes-borrowed-sometimes-owned.**

```rust
// bad — every caller fight lifetime
struct Parser<'a> { input: &'a str, cursor: usize }

// good if perf allow — owned, no lifetime virus
struct Parser { input: String, cursor: usize }
```

## Traits

- **Small trait. Single purpose.** `Read`, `Write`, `Display`. Not `EverythingDoer`.
- **Default methods OK.** Reduce impl burden.
- **`dyn Trait` if heterogeneous collection.** `impl Trait` if known at compile.
- **`From`/`Into` for conversion.** `TryFrom`/`TryInto` if can fail. Free `?` integration.

```rust
// good — From give caller ergonomics
impl From<&str> for UserId {
    fn from(s: &str) -> Self { UserId(s.parse().unwrap()) }
}
let id: UserId = "42".into();
```

## Derive everything safe

`#[derive(Debug, Clone, PartialEq, Eq, Hash)]` on data types. Default. No reason not.

`#[derive(Default)]` if all fields default-able. Else `impl Default` manual.

## Panic discipline

See `rex-code-error-writing` for full panic-vs-Result table. Short version: panic on bug, Result on bad input.

## Module organization

- **`mod foo;` per concept, not per type.** `auth/` not `auth_user.rs` + `auth_token.rs` + `auth_session.rs`.
- **`pub use` for clean facade.** Re-export at crate root.
- **`pub(crate)`** > `pub` when only internal. Shrink API surface.
- **Tests in same file.** `#[cfg(test)] mod tests { ... }`. Locality.

## Async traps

- **`async fn` in trait: stable since 1.75.** Use it. No `#[async_trait]` macro any more (mostly).
- **`Send` bound by default.** Drop only if proven need.
- **`tokio::spawn` capture = `'static`.** Clone `Arc` not borrow.
- **`.await` hold `MutexGuard` = deadlock waiting.** Drop guard before await.

## Comments

Per `rex-code-commenting`: default no comment. Rust-specific:

- **No `// returns user` on `fn -> User`.** Type say it.
- **`///` doc comment on pub item.** Required by lint (`missing_docs`). Write WHY, example, panics, errors.
- **`//` inline comment only when WHY non-obvious.** Lifetime quirk, perf hack, vendor bug.

```rust
// bad
/// Gets the user by id
fn get_user(id: UserId) -> Option<User>

// good
/// Look up user by id. Returns `None` if user soft-deleted or never existed.
/// Hot path — uses cached lookup. See [`User::reload`] to force fresh.
fn get_user(id: UserId) -> Option<User>
```

## Checklist before commit

- [ ] No `bool` flag in public fn (use enum)
- [ ] Public fn take `&str`/`&[T]`/`&Path` not owned-borrow
- [ ] Newtype around domain primitives
- [ ] Methods on types, not free fns in `_utils` modules
- [ ] No lifetime annotation in struct unless necessary
- [ ] `///` docs on every `pub` item
- [ ] No comment restating what code do
- [ ] More than 3 args -> struct or builder
- [ ] Error rules → `rex-code-error-writing` checklist
- [ ] Comment rules → `rex-code-commenting` checklist

Fail any -> fix before push.

---
name: rex-code-ergonomics
description: >
  Hard rules for ergonomic Rust: naming, types, traits, iterators, lifetimes,
  panics, and fn signatures.
when_to_use: >
  Use when you write or review Rust, design public APIs, name types and fns,
  choose String vs &str, Result vs panic, generic vs dyn, or builder vs struct
  literal, or the user asks "is this idiomatic Rust", "ergonomic API", "make
  this nicer".
disable-model-invocation: false
user-invocable: true
---

# Rust Ergonomics — Hard Rules

These rules HARD. No compromise. Apply every Rust file.

Sister skills:
- `rex-code-philosophy` — why (complexity-averse, habitability)
- `rex-code-commenting` — no comments unless WHY non-obvious
- `rex-code-error-writing` — thiserror/anyhow rules + Result patterns (read for all error work)

This skill = Rust-specific applications. Error rules: see error-writing skill.

Default: **caller's life > author's cleverness**. Public API = your gift to future self.

IF you want bad + good code for a section, THEN read `.claude/skills/rex-code-ergonomics/examples.md`. It has same headings.

## Names

- **snake_case fns/vars. PascalCase types. SCREAMING_SNAKE consts.** Non-negotiable.
- **No abbreviations.** `usr_cnt` -> `user_count`. `cfg` OK (idiomatic). `ctx` OK. `mgr` no.
- **Boolean = predicate.** `is_active`, `has_token`, `can_retry`. Never `active`, `flag`, `valid`.
- **Type name carry meaning.** `UserId` not `Id`. `OrderTotal` not `Amount`.
- **Verb on fn. Noun on type.** `parse_url` + `Url`. Not `url_parser` + `parse`.

## Types — newtype the domain

Primitive obsession = bug factory. Wrap domain primitives.

- **Newtype = ~free.** IF perf paranoid, THEN `#[repr(transparent)]`.
- **`NonZeroU32`, `NonEmpty<T>`** when invariant exist. Push to type, drop runtime check.
- **No `bool` flags in public fn.** Two bools = 4 states = caller confusion. Use enum.

## String types — pick right one

| Want | Type |
|------|------|
| Borrow, read-only | `&str` |
| Owned, mutable | `String` |
| Read-only, sometimes owned | `Cow<'_, str>` |
| Owned, immutable, cheap clone | `Arc<str>` or `Box<str>` |

- **Public fn args: `&str` not `&String`.** Caller pass either.
- **IF you create it, THEN return `String`.** Do not fight lifetimes for fun.
- **`Box<str>` over `String` for stored-once data.** Skip capacity field.

## Errors + panics

**Lib: `thiserror`. Bin: `anyhow`.** Panic on bug, `Result` on bad input.
All error + panic rules (`?`, `.unwrap()`, panic-vs-Result table): `.claude/skills/rex-code-error-writing/SKILL.md`.

## Function signatures

- **Take `&[T]` not `&Vec<T>`. Take `&str` not `&String`. Take `&Path` not `&PathBuf`.** Caller flexibility.
- **Return owned (`Vec`, `String`) when you allocate.** No lifetime games.
- **`impl Trait` in return for hide-the-iterator.** `-> impl Iterator<Item = User>`.
- **Generic in arg, concrete in return.** `fn read(r: impl Read) -> Vec<u8>`.
- **More than 3 args -> struct.** IF many optional, THEN builder.

## Methods on types (locality)

Per philosophy: behavior on thing. Not free fn somewhere.

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

## Option / Result combinators

- **`?` first.** Combinators second. Match last resort.
- **`.unwrap_or_default()`, `.unwrap_or_else(|| ...)`, `.ok_or(err)`** > match.
- **`if let Some(x) = ...` for one branch.** `match` for two+ variants.

## Lifetimes — minimize

- **Owned > borrowed in struct fields.** Lifetime in struct = viral pain.
- **Elision works 90%.** Do not annotate when compiler infer.
- **`'static` bound only when necessary.** Spreads.
- **`Cow` when sometimes-borrowed-sometimes-owned.**

## Traits

- **Small trait. Single purpose.** `Read`, `Write`, `Display`. Not `EverythingDoer`.
- **Default methods OK.** Reduce impl burden.
- **IF heterogeneous collection, THEN `dyn Trait`.** IF known at compile, THEN `impl Trait`.
- **`From`/`Into` for conversion.** IF can fail, THEN `TryFrom`/`TryInto`. Free `?` integration.

## Derive everything safe

`#[derive(Debug, Clone, PartialEq, Eq, Hash)]` on data types. Default. No reason not.

IF all fields default-able, THEN `#[derive(Default)]`. ELSE manual `impl Default`.

## Module organization

**`pub(crate)` > `pub` when only internal.** Shrink API surface.
All module, file, and visibility rules: `.claude/skills/rex-code-folder-management/SKILL.md`.

## Async traps

- **`async fn` in trait: stable since 1.75.** Use it. No `#[async_trait]` macro any more (mostly).
- **`Send` bound by default.** Drop only IF proven need.
- **`tokio::spawn` capture = `'static`.** Clone `Arc` not borrow.
- **`.await` hold `MutexGuard` = deadlock risk.** Drop guard before await.

## Comments

Per `rex-code-commenting`: default no comment. Rust-specific:

- **No `// returns user` on `fn -> User`.** Type say it.
- **`///` doc comment on pub item.** Lint (`missing_docs`) requires it. Write WHY, example, panics, errors.
- **`//` inline comment only when WHY non-obvious.** Lifetime quirk, perf hack, vendor bug.

## Checklist before commit

- [ ] No `bool` flag in public fn (use enum)
- [ ] Public fn take `&str`/`&[T]`/`&Path` not owned-borrow
- [ ] Newtype around domain primitives
- [ ] Methods on types, not free fns in `_utils` modules
- [ ] No lifetime annotation in struct unless necessary
- [ ] `///` docs on every `pub` item
- [ ] More than 3 args -> struct or builder
- [ ] Error rules → `rex-code-error-writing` checklist
- [ ] Comment rules → `rex-code-commenting` ergonomics check

IF any fail, THEN fix before push.

# Branching and dispatch

Mispredicted branches, cold paths, static against dynamic dispatch, bounds checks.

Typical figures. Measure on your own hardware.

## Symptom

- Hot loop is fast on test data, slow or uneven on real market data.
- Loop branches on tick prices, packet arrival order or order matching results.
- Rare error code sits inline in a hot fn.
- Hotpath calls through `dyn Trait`, `Box<dyn Trait>` or a `fn` pointer.

## Confirm

Method: `.claude/skills/rex-code-latency/measure.md`. Counters: `.claude/skills/rex-code-latency/measurement/counters.md`.

1. Count `branches` and `branch-misses` around the loop on replayed production data.
2. IF misses per iteration are near 0, THEN stop: branches are not the cause. A random 50/50 branch gives up to 0.5.
3. Read the release assembly (`cargo asm`). A source `if` is a branch only IF it shows `j*` after a `cmp` on data. Debug builds add overflow-check branches.
4. For a layout problem, count L1i misses and hotpath instructions up to `ret`.
5. Benchmark both forms on production-like data. Keep the winner.

## Rules

Data-dependent branches:

- [ ] Remove a branch only IF Confirm shows misses and a `j*`, AND it depends on unpredictable outside data near 50/50.
- [ ] Keep a branch that goes about 99% one way. A predicted branch is nearly free. A `cmov` waits for both inputs every pass.
- [ ] Write the clear `if` first. LLVM turns a small `if` body into `cmov` (x86_64) or `csel` (aarch64) by itself.
- [ ] IF the branch stays (body has a store, a call or more work), THEN use `std::hint::select_unpredictable(cond, a, b)`. It takes any `T`, also `&mut`: select the target, then run one path.
- [ ] Predication form: `sum += u64::from(v) * u64::from(v >= thr)`. Neither form guarantees no branch.

Cold paths:

- [ ] Mark a branch cold only IF it is taken under 1% of the time: errors, corrupt packets, limit breaches. Never at 60/40: the 40% side pays, and throughput can collapse.
- [ ] Put `std::hint::cold_path()` first in the cold arm. Put the cold work in a `#[cold]` `#[inline(never)]` fn that also builds the error value.
- [ ] Hints change block layout, not the CPU branch predictor.
- [ ] Panic paths are cold already. `?` and bounds checks need no hint.

Dispatch:

- [ ] No `dyn Trait` or `fn` pointer call on the hotpath: one vtable load, one indirect jump, no inlining.
- [ ] IF the type is known at build time, THEN use a generic: `fn route<G: Gateway>(g: &G)`. Each type gets its own copy: many copies fill L1i.
- [ ] IF the type set is closed but chosen at run time, THEN use an `enum` with `match`, or `enum_dispatch`. Arms can inline.
- [ ] IF variants mix at random in one loop, THEN the tag branch is data-dependent. Group by variant first.
- [ ] Move version and feature checks to build time: associated `const` + inline `const { assert!(G::VERSION >= MIN) }`, a cargo feature, or a const generic (`Engine<const RISK_CHECKS: bool>`).
- [ ] `cargo check` skips a `const` assert in generic code: run `cargo build` before push.

Bounds checks:

- [ ] IF a hot loop stores, THEN remove its bounds checks. Use iterators or `zip`, one re-slice to a common length up front, or a `[T; N]` array with a masked index (`book[usize::from(level & 0x0F)]`). A read-only loop gets checks hoisted already.
- [ ] Check input lengths once at the hotpath edge and return a typed error.

## Numbers

| Item | Figure |
| --- | --- |
| Misprediction, pipeline flush | 15–20 cycles |
| Misprediction that also evicts hot code from L1i | hundreds of ns |
| Branch on random data | about 50% correct: one flush per two iterations |

## Rust tools

- `std::hint::select_unpredictable`: stable since 1.88.0, not `const`.
- `std::hint::cold_path`: stable since 1.95.0, `const`. `likely` / `unlikely` are nightly. On stable: `#[inline(always)] const fn likely(b: bool) -> bool { if !b { cold_path() } b }`.
- `enum_dispatch` 0.3.13, no features: writes `impl Trait for Enum` as a `match`.
- `cargo-show-asm` 0.2.64: `cargo asm --lib --intel --simplify <fn>`. `cmov*` / `csel` = select. `jmp reg` / `call reg` = indirect jump. `panic_bounds_check` = bounds check.
- `cargo-pgo` 0.3.0 + `rustup component add llvm-tools-preview`: `cargo pgo build`, run on a replay, `cargo pgo optimize`. It can turn a hot `dyn` call into a compare plus a direct call.

## Traps

- CAUTION: Do not use predication or `select_unpredictable` for constant-time cryptography. The `cmov` lowering is not guaranteed on any platform, so timing can leak. Use `subtle` 2.6.1.
- CAUTION: Do not replace a bounds check with `get_unchecked`. An index out of range is undefined behavior, and the safe forms give the same loop.
- `cargo asm` prints "Can't find any items" for a small `pub` leaf fn in a lib. Add `#[inline(never)]`, or read the caller.
- A PGO profile from a quiet market can mark the burst path cold. Rebuild it when the strategy changes.

## Code

`.claude/skills/rex-code-latency/cpu/branching-code.md`:

- Branch that LLVM keeps, removed with `select_unpredictable`. Read when `j*` stays.
- Packet validator with a cold error path. Read to mark an error branch cold.
- Generic gateway with a build-time version check. Read to replace `dyn Trait`.

## See also

- IF the loop misses cache, not branches, THEN read `.claude/skills/rex-code-latency/cpu/cache-hierarchy.md`.

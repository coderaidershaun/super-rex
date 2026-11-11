# Branching and dispatch

Mispredicted branches, cold paths, static against dynamic dispatch, bounds checks.

## Symptom

- Hot loop is fast on test data, slow or uneven on real market data.
- Loop branches on tick prices, packet arrival order or order matching results.
- Rare error code sits inline in a hot fn.
- Hotpath calls through `dyn Trait`, `Box<dyn Trait>` or a `fn` pointer.
- Make an `if` branchless? Add `likely`?

## Confirm

Method: `.claude/skills/rex-code-latency/measure.md`. Counters: `.claude/skills/rex-code-latency/measurement/counters.md`.

1. Count `branches` and `branch-misses` around loop on replayed production data.
2. IF misses per iteration are near 0, THEN stop: branches are not the cause. A random 50/50 branch gives up to 0.5.
3. IF the loop has many branches, THEN find the one that misses: `perf record -e branch-misses:pp` on the replay (test host), then `perf annotate`. Check on your host.
4. Read release assembly (`cargo asm`). A source `if` is a branch only IF it shows `j*` after a `cmp` on data.
5. For a layout problem, count L1i misses and hotpath instructions up to `ret`.
6. Benchmark both forms on production-like data. Keep the winner.

## Rules

Data-dependent branches:

- [ ] Remove a branch only IF Confirm shows misses and a `j*`, AND it depends on unpredictable outside data near 50/50.
- [ ] Keep a branch that goes about 99% one way: predicted = nearly free. A `cmov` waits for both inputs every pass.
- [ ] Write the clear `if` first. LLVM makes a small `if` body `cmov` (x86_64) or `csel` (aarch64) by itself.
- [ ] IF the branch stays (body stores or calls), THEN use `std::hint::select_unpredictable(cond, a, b)`. It takes any `T`, also `&mut`.
- [ ] Predication form: `sum += u64::from(v) * u64::from(v >= thr)`. Neither form guarantees no branch.
- [ ] A parse loop mostly misses on the end of each variable-length field. Parse fixed-width, 8 bytes at once (SWAR), or with SIMD.

Cold paths:

- [ ] Mark a branch cold only IF taken under 1% of the time: errors, corrupt packets, limit breaches. Never at 60/40.
- [ ] Put `std::hint::cold_path()` first in the cold arm. Put the cold work in a `#[cold]` `#[inline(never)]` fn that also builds the error value.
- [ ] Hints change block layout, not the branch predictor.
- [ ] Panic paths (bounds checks, `unwrap`, overflow) are cold already. A `?` return is not: IF its error is rare, THEN `match` and call `cold_path()` in the `Err` arm.

Dispatch:

- [ ] No `dyn Trait` or `fn` pointer call on the hotpath: indirect jump (`dyn` adds a vtable load), no inlining.
- [ ] IF the type is known at build time, THEN use a generic: `fn route<G: Gateway>(g: &G)`. Each type gets its own copy: many copies fill L1i.
- [ ] IF the type set is closed but chosen at run time, THEN use an `enum` with `match`, or `enum_dispatch`. Arms can inline.
- [ ] A dense `match` can compile to a jump table: an indirect jump again. Check with `cargo asm`.
- [ ] IF variants mix at random in one loop, THEN tag branch is data-dependent. Group by variant first.
- [ ] Move version and feature checks to build time: inline `const` assert, cargo feature, or const generic.
- [ ] `cargo check` skips a `const` assert in generic code: `cargo build` catches it.

Bounds checks:

- [ ] IF `cargo asm` shows `panic_bounds_check` in a hot loop, THEN use iterators, `zip`, one re-slice to a common length, or a `[T; N]` array with a masked index.
- [ ] Check input lengths once at hotpath edge: return a typed error.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Figure |
| --- | --- |
| Misprediction, pipeline flush | 15–20 cycles |
| Misprediction that also evicts hot code from L1i | hundreds of ns |

## Rust tools

- `std::hint::select_unpredictable`: stable since 1.88.0, not `const`.
- `std::hint::cold_path`: stable since 1.95.0, `const`. `likely` / `unlikely` are nightly. On stable: `#[inline(always)] const fn likely(b: bool) -> bool { if !b { cold_path() } b }`.
- `enum_dispatch` 0.3.13, no features: writes `impl Trait for Enum` as a `match`.
- `cargo-show-asm` 0.2.64: `cargo asm --lib --intel --simplify <fn>`. Look for `cmov*` / `csel` (select), `jmp reg` / `call reg` (indirect jump), `panic_bounds_check`.
- `cargo-pgo` 0.3.0 + `llvm-tools-preview`: `cargo pgo build`, run on a replay, `cargo pgo optimize`. Can turn a hot `dyn` call into compare + direct call.

## Traps

- CAUTION: Do not use predication or `select_unpredictable` for constant-time cryptography. The `cmov` lowering is not guaranteed on any platform, so timing can leak. Use `subtle` 2.6.1.
- CAUTION: Do not replace a bounds check with `get_unchecked`. An index out of range is undefined behavior, and the safe forms give the same loop.
- `cargo asm` finds no small `pub` leaf fn of a lib: add `#[inline(never)]`, or read the caller.
- A PGO profile from a quiet market can mark the burst path cold. Rebuild it when the strategy changes.

## Code

`.claude/skills/rex-code-latency/cpu/branching-code.md`:

- Route with `select_unpredictable`: read when `j*` stays.
- Packet validator with cold error path: read to mark an error branch cold.
- Generic gateway with build-time version check: read to replace `dyn Trait`.

## See also

- IF the loop misses cache, not branches, THEN read `.claude/skills/rex-code-latency/cpu/cache-hierarchy.md`.

# Zero allocation

Keep the global allocator off the hotpath: hidden allocations, proof, arenas, pools.

## Symptom

- p99 spikes in `Box::new`, `Vec::push` or `format!`, worse with more threads.
- Spike each time a vector or map grows.
- First-touch spikes after `vec![0; n]` or `Vec::with_capacity(n)`.
- Spike when a `Vec<Arc<T>>` clears or the last `Arc` drops.
- "Does this hotpath allocate?"

## Confirm

Measure as `.claude/skills/rex-code-latency/measure.md` says.

1. Count heap calls on the hotpath thread in a measured window, release profile. Expect 0.
2. IF count > 0, THEN find each call stack with `dhat`.
3. IF count = 0 and first-touch spikes stay, THEN cause = page faults. Read `.claude/skills/rex-code-latency/os/huge-pages.md`.

## Rules

Hidden allocations. None on the hotpath:

- [ ] `Box::new`, `Arc::new`, `Rc::new`.
- [ ] `Vec::push`, `String::push_str`, `HashMap::insert` past capacity.
- [ ] `String`, `format!`, `to_string()`, `collect()`, boxed closure.
- [ ] `Box<dyn Error>`, `anyhow::Error`. Use a `thiserror` enum with `Copy` fields.
- [ ] Panic: payload and hook output allocate.
- [ ] Frees: drop of `Box`, `Vec`, `String`, last `Arc`. `Vec<Arc<T>>::clear()` = one free per element.

Startup:

- [ ] Allocate all memory at startup, after the thread pins (local NUMA node).
- [ ] Reserve every container to peak size. Fixed size: `Box<[T]>`, which cannot grow.
- [ ] Pre-fault on purpose: write one byte per 4096-byte page, or fill a `Vec` once then `clear()`, or `nix::sys::mman::mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE)`.
- [ ] Scratch `Vec` built at startup, `clear()` per tick: keeps capacity, = arena for one type.

| IF | THEN |
| --- | --- |
| Data dies together (per tick) | `bumpalo` arena |
| Independent lifetimes, one type | `slab`, or `Box<[Option<T>]>` + `Vec<u32>` LIFO free stack (slot freed last is hot in L1) |
| Slot keys leave the owner | `slotmap` |
| Capacity known at build time | `heapless::Vec`, `arrayvec::ArrayVec` |
| `Vec` or map in pre-faulted pool memory | `allocator-api2` `Allocator` impl + `hashbrown` |
| Lifetimes overlap (session of hours, order of ms) | Pool, never arena |

- [ ] Arena: `Bump::with_capacity(n)`, then `set_allocation_limit(Some(bump.allocated_bytes()))`: full arena returns `Err`, not a new chunk. `reset()` at tick start.

Pools:

- [ ] One pool per thread, no lock. Keep it `!Sync` (`RefCell` inside): a second thread does not compile. Shared pool = atomics + false sharing.
- [ ] IF pool is empty or request too large, THEN return `Err` from a `#[cold]` `#[inline(never)]` fn. Never fall back to heap.
- [ ] Round block size up to 64. Refuse `layout.align()` above block alignment.
- [ ] Pool-backed container: `try_reserve_exact` once, then `push_within_capacity`. A plain `push` that gets `AllocError` aborts.

Shared ownership:

- [ ] Each `Arc::clone` = atomic add on a shared cache line. A move is free. Make the last drop happen on a housekeeping core.

Proof:

- [ ] One zero-allocation test per hotpath entry point, release profile.
- [ ] Compile-time proof: hotpath in a `#![no_std]` crate with no `extern crate alloc`. Keep signatures concrete: a caller's generic `T` can allocate.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Value |
| --- | --- |
| Allocator lock collision | 1 ns → several thousand ns |
| Access to scattered heap node | 100 ns |
| Arena allocate | one addition (`bumpalo`: + one bounds check) |
| `Option<T>`, `T` has a niche (`bool`) | 0 extra bytes |
| `hashbrown::HashMap<u32, u64>`, `try_reserve(3)` | needs a block above 64 bytes |

## Rust tools

- `bumpalo` 3.20.3, `collections`: runs no `Drop` for arena values, so plain data only. `!Sync`. Chunk comes from the global allocator.
- `bump-scope` 2.4.1: arena on your own allocator (huge page). Not compile-checked.
- `slab` 0.4.12: `insert` grows when full. Test `len() < capacity()` first.
- `slotmap` 1.1.1: stale key gets `None`.
- `heapless` 0.9.3, `arrayvec` 0.7.8: `no_std`. `push` returns the item when full.
- `allocator-api2` 0.2.21, `alloc`: not 0.4, because `hashbrown` 0.17 and `bumpalo` 3.20 implement 0.2.
- `hashbrown` 0.17.1, `allocator-api2`, `default-hasher`.
- `memmap2` 0.9.12 (`populate()`, `huge()` act on Linux only). `nix` 0.31.3, `mman` (`mlockall`).
- `assert_no_alloc` 1.1.2: aborts on alloc or free inside a closure. Set `default-features = false`, or `disable_release` turns it off in release.
- `dhat` 0.3.3: call stack of each block. Tests only: global mutex per call.
- `basedrop` 0.1.3: sends drops to a collector thread. Few users: read its code first.

## Traps

- [ ] `vec![0; n]` uses `alloc_zeroed` and gets lazy zero pages. `vec![T::default(); n]` with all-zero bytes: same.
- [ ] Clippy `slow_vector_initialization` pushes a fill loop toward `vec![0; n]`. Keep the loop with `#[expect(clippy::slow_vector_initialization, reason = "...")]`.
- [ ] `mimalloc` or `tikv-jemallocator` lowers the average. Its slow path still locks, searches and calls `mmap`.
- [ ] `smallvec::SmallVec` spills to the heap with no error.
- [ ] Optimizer can delete a `malloc`/`free` pair: debug and release counts differ.
- [ ] `assert_no_alloc` abort says "memory allocation of N bytes failed", not "forbidden".

## Code

`.claude/skills/rex-code-latency/memory/zero-alloc-code.md`:

- Counting global allocator, `assert_no_alloc`, `dhat`: read before you write a zero-allocation test.
- Fixed-block pool behind `Allocator`: read IF a `Vec` or map must live in pre-faulted pool memory.

## See also

- IF pool memory comes from huge pages, THEN read `.claude/skills/rex-code-latency/os/huge-pages.md`.
- IF other threads read pooled objects, THEN read `.claude/skills/rex-code-latency/memory/reclamation.md`.
- IF a log call allocates, THEN read `.claude/skills/rex-code-latency/measurement/logging.md`.

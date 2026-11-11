# Cache line layout

False sharing, hot fields in one line, alignment, padding, build-time checks.

## Symptom

- Two threads on separate cores each write their own variable, and both run slower than one thread alone.
- Per-core counters, indices or flags sit side by side in one struct or array.
- Hot state of one thread is over 64 bytes, or larger than its alignment.
- Latency rose after someone added a field to a hot struct.

## Confirm

Method: `.rex/skills/rex-code-latency/measure.md`. Counters: `.rex/skills/rex-code-latency/measurement/counters.md`.

False sharing:

1. Print the address of each variable that a different core uses. IF two are under 128 bytes apart and one core writes, THEN they can share a line or line pair.
2. Time one write alone, then with the other cores running. IF it rises from about 1 ns to hundreds of ns, THEN suspect false sharing.
3. Linux proof, on a test host: `perf c2c record`, then `perf c2c report`: lines with HITM loads (line taken from a modified copy in another core). Check on your host.
4. IF no core writes the line, THEN it is not false sharing.
5. Run the threads on cores that are not SMT siblings (`thread_siblings_list`): siblings share L1 and one core.

Line spill: print `size_of` and `align_of` in a test. IF size is over 64, OR size is over `align_of`, THEN one access can touch two lines.

## Rules

Between cores:

- [ ] Apply only IF one core writes a line that another core reads or writes, and one of them is the hotpath.
- [ ] Wrap each variable that a different core writes in `crossbeam_utils::CachePadded<T>`. Per-core slots: `[CachePadded<AtomicU64>; N]`.
- [ ] Do not hardcode 64 for separation. `CachePadded` picks the boundary per target: x86_64 prefetches line pairs, Apple M has 128-byte lines.
- [ ] Separate with `#[repr(align(64))]` (or 128) only IF a peer fixes the value: C or C++ code, a NIC, shared memory.
- [ ] IF all cores write one variable (true sharing), THEN padding does not help. Give each core its own padded counter. Sum on read.
- [ ] A counter with one writer: `Relaxed` load + `Relaxed` store, no locked instruction. `fetch_add` only IF more than one thread writes it.

Inside one thread, for hotpath state:

- [ ] Keep data one thread reads together in one 64-byte line: 65 bytes = two fetches.
- [ ] Split hot from cold: hotpath fields in one `#[repr(align(64))]` struct, rare fields elsewhere. rustc sorts for padding, not heat.
- [ ] IF no process, device or C code reads the bytes, THEN keep the default repr: rustc reorders fields to cut padding. `#[repr(align(64))]` fixes the start and pads the size to 64.
- [ ] IF the bytes leave the process (FFI, wire, shared memory, DMA, file), THEN use `#[repr(C)]` and order fields largest alignment first. A `bool` before a `u32` costs 3 bytes.
- [ ] Pack flags into bits: `bitfield-struct`, `bitflags` 2.13.2, or a `u8` newtype with masks.
- [ ] IF the struct is viewed as bytes, THEN fill the line with an explicit `[u8; N]` field and derive `zerocopy::IntoBytes`. It fails the build on hidden padding.

Guards:

- [ ] Beside each hot struct: `const _: () = assert!(size_of::<T>() == 64 && align_of::<T>() == 64);`. A bad edit fails the build (E0080).
- [ ] Under `repr(C)`, also assert hot field offsets with `offset_of!`. Under the default repr, assert only size and alignment: its field order is not a promise.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Figure |
| --- | --- |
| `CachePadded` alignment | 128 on x86_64, aarch64, powerpc64. 32 on arm, mips. 256 on s390x. 64 elsewhere |
| Ten mixed fields: `repr(C)` as written / default repr / `repr(C)` sorted, flags as bits | 72 / 48 / 40 bytes |

## Rust tools

- `crossbeam-utils` 0.8.23, `default-features = false`: `CachePadded<T>`. Size = smallest multiple of the boundary that holds `T`.
- `#[repr(align(N))]`: N must be a literal. It applies to a type, not a field: wrap the field.
- `zerocopy` 0.8.63, `default-features = false`, `features = ["derive"]`: `IntoBytes`, `Immutable`, `KnownLayout`. `bytemuck` 1.25.2 `Pod` does the same padding check.
- `bitfield-struct` 0.13.0: `#[bitfield(u8)]` gives a `repr(transparent)` integer with `const fn` getters and setters.

## Traps

- The type system cannot see false sharing: two `Sync`, race-free `AtomicU64` fields still bounce the line.
- Filler `_pad: [u8; 56]` after a field fixes the gap, not the start address. Only `align` stops a neighbor from starting in the same line.
- CAUTION: Do not read a struct with hidden padding as bytes (transmute, raw slice, hash of raw memory). Padding bytes are uninitialized, and a read of them is undefined behavior.
- `#[repr(packed)]` is not a fix. Fields become unaligned, a reference to a field is error E0793, and one load can split across two lines.

## Code

`.rex/skills/rex-code-latency/cpu/cache-line-layout-code.md`:

- Padded counters for two writer cores, with build-time checks: read to separate per-core data.
- Hot state packed into one aligned line: read when a `repr(C)` struct must fill one line exactly.

## See also

- IF cores share data through atomics, THEN read `.rex/skills/rex-code-latency/threads/atomics.md`.
- IF the shared data is a queue between two cores, THEN read `.rex/skills/rex-code-latency/threads/ring-buffers.md`.
- IF one thread wastes cache lines in a scan, THEN read `.rex/skills/rex-code-latency/cpu/cache-hierarchy.md`.

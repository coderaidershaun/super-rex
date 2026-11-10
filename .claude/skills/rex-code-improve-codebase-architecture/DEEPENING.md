# Deepening

How to deepen cluster of shallow modules safely, given deps. Vocab from [LANGUAGE.md](LANGUAGE.md) — **module**, **interface**, **seam**, **adapter**.

## Dependency categories

Assess candidate for deepening -> classify deps. Category determines how deepened module tested across seam.

### 1. In-process

Pure compute, in-memory state, no I/O. Always deepenable — merge modules, test through new interface direct. No adapter.

### 2. Local-substitutable

Deps w/ local test stand-ins (PGLite for Postgres, in-memory FS). Deepenable if stand-in exists. Deepened module tested w/ stand-in in test suite. Seam internal. No port at module's external interface.

### 3. Remote but owned (Ports & Adapters)

Own services across network boundary (microservices, internal APIs). Define **port** (interface) at seam. Deep module owns logic. Transport injected as **adapter**. Tests use in-memory adapter. Prod uses HTTP/gRPC/queue adapter.

Recommendation shape: *"Define port at seam, implement HTTP adapter for prod + in-memory adapter for testing -> logic sits in one deep module even though deployed across network."*

### 4. True external (Mock)

Third-party services (Stripe, Twilio etc.) you don't control. Deepened module takes external dep as injected port. Tests provide mock adapter.

## Seam discipline

- **One adapter = hypothetical seam. Two adapters = real seam.** No port unless ≥2 adapters justified (typical: prod + test). Single-adapter seam = indirection.
- **Internal seams vs external seams.** Deep module can have internal seams (private to impl, used by own tests) + external seam at interface. No exposing internal seams through interface just because tests use them.

## Testing strategy: replace, don't layer

- Old unit tests on shallow modules = waste once tests at deepened module's interface exist -> delete.
- Write new tests at deepened module's interface. **Interface = test surface.**
- Tests assert on observable outcomes through interface, not internal state.
- Tests survive internal refactors — describe behaviour, not impl. Test must change when impl changes? -> testing past interface.

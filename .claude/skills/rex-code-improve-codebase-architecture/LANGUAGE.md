# Language

Shared vocab for every suggestion. Use exact — no sub "component", "service", "API", "boundary". Consistent language = whole point.

## Terms

**Module**
Anything with interface + impl. Scale-agnostic — applies to fn, class, package, tier-spanning slice.
_Avoid_: unit, component, service.

**Interface**
Everything caller must know to use module correct. Type signature + invariants, ordering, error modes, required config, perf characteristics.
_Avoid_: API, signature (too narrow — type-level only).

**Implementation**
Body of code inside module. Distinct from **Adapter**: thing can be small adapter w/ large impl (Postgres repo) or large adapter w/ small impl (in-memory fake). "Adapter" when seam = topic. "Implementation" otherwise.

**Depth**
Leverage at interface. Behaviour caller (or test) exercises per unit of interface learned. **Deep** = lots of behaviour behind small interface. **Shallow** = interface ≈ impl complexity.

**Seam** _(from Michael Feathers)_
Place where behaviour alters without edit in that place. *Location* of module's interface. Where to put seam = own design decision, distinct from what goes behind.
_Avoid_: boundary (overloaded with DDD bounded context).

**Adapter**
Concrete thing satisfies interface at seam. Describes *role* (slot it fills), not substance (what's inside).

**Leverage**
Caller payoff from depth. More capability per unit of interface learned. One impl pays back across N call sites + M tests.

**Locality**
Maintainer payoff from depth. Change/bugs/knowledge/verification concentrate at one place, not spread across callers. Fix once, fixed everywhere.

## Principles

- **Depth = property of interface, not impl.** Deep module can internally compose small mockable swappable parts — they aren't part of interface. Module can have **internal seams** (private to impl, used by own tests) + **external seam** at interface.
- **Deletion test.** Delete module. Complexity vanish? -> pass-through. Complexity reappear across N callers? -> earned its keep.
- **Interface = test surface.** Callers + tests cross same seam. Want to test *past* interface? Module probably wrong shape.
- **One adapter = hypothetical seam. Two adapters = real seam.** No new seam unless something varies across it.

## Relationships

- **Module** has exactly one **Interface** (surface to callers + tests).
- **Depth** = property of **Module**, measured against **Interface**.
- **Seam** = where **Module**'s **Interface** lives.
- **Adapter** sits at **Seam**, satisfies **Interface**.
- **Depth** -> **Leverage** for callers + **Locality** for maintainers.

## Rejected framings

- **Depth = ratio of impl-lines to interface-lines** (Ousterhout): rewards padding impl. Use depth-as-leverage.
- **"Interface" = TypeScript `interface` keyword or class public methods**: too narrow — interface here = every fact caller must know.
- **"Boundary"**: overloaded with DDD bounded context. Say **seam** or **interface**.

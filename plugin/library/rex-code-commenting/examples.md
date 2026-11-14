# Comment examples

Rules for each example are in `rex-code-commenting/SKILL.md`.

## Plain English (Iron law #3)

```rust
// bad — telegraphic
// mnl sort: stdlib O(n²) near-sorted. 4x per bench.

// bad — jargon
// Adapter at the ingest seam; deepens the module by hiding the transport.

// bad — plan reference
// Phase 2 of the refactor plan: temporary until the loader lands. See src/loader/mod.rs.

// good — short plain English
// Manual sort because the stdlib is O(n²) on near-sorted input; benchmarks show this is 4x faster.

// good — short plain English
// The exchange sends prices as strings, so parse before comparing.
```

## Quotables (carry these)

- "The best comment is the one you didn't need to write."
- "Comments rot faster than code."
- "Most comments don't improve quality. They create illusion of clarity."
- "Comment = duct tape for poor design."
- "Write code as if comments didn't exist. Then add the few that must exist."
- "Stale comment > silence in damage. Wrong worse than missing."

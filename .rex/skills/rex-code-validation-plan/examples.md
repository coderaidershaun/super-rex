# Bad trace / good trace

Words + trace rules: `.rex/skills/rex-code-validation-plan/SKILL.md`.

Bad: `V2: on gap, book recovers and works again.` No start state, no steps, no values. Test of it passes with wrong order.

Good. Trace in `plan.md`:

```text
V2 — sequence gap resets book (SyncError::SequenceGap, RESET)
Start: book at seq 10. Trigger: feed gives update seq 12.
1. book: Gap { expected: 11, got: 12 }
2. supervisor: reset book
3. book: snapshot request
4. book: snapshot applied (last_seq 15)
5. book: ready
End: book = snapshot at seq 15. No quote from book between 2 and 5.
Lane: contract. Test: sequence_gap_resets_book_from_snapshot.
```

Test of that trace:

```rust
#[test]
fn sequence_gap_resets_book_from_snapshot() {
    use Event::*;
    let mut sys = System::with_book_at(10);
    sys.feed(update(12));
    sys.run_until_idle();
    let want = [Gap { expected: 11, got: 12 }, Reset(Part::Book), SnapshotRequest,
                SnapshotApplied { last_seq: 15 }, Ready(Part::Book)];
    assert_eq!(sys.events(), want);
}
```

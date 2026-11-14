# AUDIT

Rules that each hit can break, and the words, are in `rex-code-folder-management/SKILL.md`.

Run from crate folder. Each hit = candidate. Read hit before you report it.

```bash
# size: files near 500-line limit
find src tests -name '*.rs' | xargs wc -l | sort -rn | head -8

# children: folder must hold mod.rs + min 2 children (asset folders are skipped)
for d in $(find src -mindepth 1 -type d); do
  find "$d" -name '*.rs' | grep -q . || continue
  files=$(find "$d" -maxdepth 1 -name '*.rs' ! -name mod.rs | wc -l)
  dirs=$(find "$d" -mindepth 1 -maxdepth 1 -type d -exec test -e {}/mod.rs \; -print | wc -l)
  echo "$((files + dirs)) children  $(test -e "$d/mod.rs" && echo mod.rs || echo NO-mod.rs)  $d"
done

# name.rs beside name/ : forbidden
for d in $(find src -mindepth 1 -type d); do test -e "$d.rs" && echo "FORBIDDEN $d.rs + $d/"; done

# module graph: who imports who
grep -rnE '^\s*(pub(\([a-z]+\))? )?mod |^\s*(pub(\([a-z]+\))? )?use (crate|super)' src | sort

# forbidden names
find src -name 'utils*' -o -name 'helpers*' -o -name 'common*' -o -name 'misc*' -o -name 'shared*'

# pub item that no code outside crate can reach (0 = good)
cargo rustc --lib -- -W unreachable_pub 2>&1 | grep -c unreachable
```

From module graph, find:

- [ ] Cycle between siblings.
- [ ] Shared data module that imports job module.
- [ ] Code that names child of other folder.
- [ ] Module with one user that lives outside folder of that user.
- [ ] `crate::` path where `super::` is correct, or reverse.

## Report

| Where | Rule broken | Evidence | Fix |
| --- | --- | --- | --- |
| `src/figure/` | 1 child | holds only `refine.rs` | extract second joint `sheet` |

- [ ] Give tree before + tree after.
- [ ] Give count of children for each folder.
- [ ] Give largest file + its line count.
- [ ] "No defects" = valid result. Do not invent defects.

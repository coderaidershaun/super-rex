# Mechanical sweep block

Method + scaling: `.claude/skills/rex-code-smells/SKILL.md`.

Copy-paste for module/crate scope (adjust path):

```bash
P=src/target/module
# long fns: lines from `fn` to the closing brace with the same indent (tests included) — confirm by reading
find "$P" -name '*.rs' -exec awk 'FNR==1{n=0} /^ *(pub(\([a-z]+\))? )?(const )?(async )?(unsafe )?fn / && $0 !~ /[;}] *$/ {n=FNR; f=$0; ind=match($0,/[^ ]/)} n && /^ *}$/ && match($0,/[^ ]/)==ind {if (FNR-n>50) print FILENAME":"n" ("FNR-n" lines) "f; n=0}' {} +
grep -rn '\.unwrap()\|\.expect(' $P --include='*.rs'
grep -rnE '\b(mgr|cnt|tmp|usr|obj|proc)\b' $P --include='*.rs'
grep -rn 'fn get_' $P --include='*.rs'
grep -rnE 'fn [a-z_]+\(.*bool.*\)' $P --include='*.rs'
grep -rnE '\)\.[a-z_]+\(\)\.[a-z_]+\(\)\.[a-z_]+\(' $P --include='*.rs'   # 3-hop chains
# rot
grep -rnE '\b(TODO|FIXME|XXX|HACK)\b' $P --include='*.rs'
grep -rniE 'temporary|for now|workaround|remove (after|once|when)' $P --include='*.rs'
grep -rnE '^\s*//\s*(let|fn|use|if|match|pub|impl|return)\b' $P --include='*.rs'   # commented-out code
grep -rnE '\b(todo|unimplemented)!\(' $P --include='*.rs'
grep -rnE '#!?\[allow\(' $P --include='*.rs'
grep -rn '#\[ignore' $P tests/ --include='*.rs'
grep -rn 'SMELL' $P --include='*.rs'
```

Rot sweep at crate scope. Run one time per crate, not one time per module:

```bash
cargo build --all-targets 2>&1 | grep -E 'deprecated|unexpected_cfgs|unfulfilled_lint_expectations'
cargo test --doc                    # lib crates only
cargo tree -d                       # duplicate versions of one dep
cargo machete                       # IF installed. ELSE write it in Not checked
rex codebase --only code --with-context   # IF rex on PATH
```

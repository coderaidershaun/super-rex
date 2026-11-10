Status: DRAFT

Reviewed at gate G1 and settled by Lane 1, 10 November 2026. Not frozen yet. It waits for: C1 (section 5: `StorePaths::locate`, `Store::create`, the text of a `StoreError`) and C3 (section 4: the `hooks` value, rule 3a and section 4.6: the hook command string). Every other part is stable: Lane 4 can build 4.3, 4.4, 4.8, 4.9, 4.10 and the settings merge of 4.5 without the hook group now.

# C6. What `rex init` writes

Owner: Lane 4. Readers: Lanes 1 and 8, and Lanes 2, 3, 9 and 10. Date: 10 November 2026.

## 0. Status and sources

This contract is a draft for the G1 review. Each point marked "Proposed" is for G1 to settle. Where a text is given in a fenced block, the code uses that text exactly.

Sources: the build request (§2.1, §4, §5.6, §5.7, §5.10, §5.11, §5.12, §5.16), the build plan (section 4.1 open points OP5, OP6 and OP14; Lane 4 items 4.1 to 4.11; P0.1, P0.7), challenge 1, checks 11 to 14, Task D §8 and §9, and the rex-skills installer at commit `d926ad7`. Section 13 maps each source to the section of C6 that covers it.

Other contracts, named here and not copied:

- C1 (Lane 2, `docs/contracts/c01-store.md`): the store schema and its code interface. It gives `StorePaths::locate` and `Store::create`, the way to find the store from a worktree, and the event-log path.
- C3 (Lane 3, `docs/contracts/c03-hook.md`): the hook registration. It gives the events, the matchers and the command, as the value of the `hooks` key.
- C7 (Lane 6): the skill header and the `rex skills` commands, including OP7 (settled: there is no `INDEX.md`).
- C11 (Lane 1, `docs/contracts/conventions.md`): the output style, the exit codes, the error text, and how a lane adds an error.

## 1. Scope and command line

```
rex init [--force] [--no-hooks]
```

`rex init` sets SuperRex up in a repo once. It is safe to run twice, and it never replaces what the user has. It runs in the current folder only.

| Option | Field in `InitOptions` | Effect |
| --- | --- | --- |
| `--force` | `is_forced: bool` | Overwrite installed skill and agent files that differ from this version of rex. It never touches a project file (`.claude/settings.local.json`, `AGENTS.md`, `.gitignore`, `.pre-commit-config.yaml`). |
| `--no-hooks` | `is_without_hooks: bool` | Leave the C3 hook registration out (section 4.6). Everything else is written. |

There is no `--vendor` option (§4 Vendors). `rex init` installs for Claude Code only.

`rex init` does not use the network, does not run `git`, does not edit the user-level Claude Code settings, does not write `CLAUDE.md`, and removes nothing.

## 2. Preflight

The preflight runs before anything is written. Its checks run in this order.

1. **Repo root.** The current folder must hold a `.git` directory. rex tests this with `cwd.join(".git").is_dir()` and runs no `git` command. `<cwd>` in every message is `std::env::current_dir()` made canonical.
   - `.git` is missing: blocking.
     `<cwd> is not the root of a Git repository; run rex init in the folder that holds .git, or run git init first`
   - `.git` is a file: blocking.
     `<cwd> is a linked worktree, a submodule or a --separate-git-dir checkout (.git is a file); run rex init in the main checkout of the repository`
2. **Claude Code on PATH.** rex runs `claude --version`. `claude --version` is the one command the preflight runs. IF the start fails with `NotFound` THEN blocking (OP14):
   `Claude Code was not found: claude is not on PATH; install Claude Code, then run rex init again`
   IF `claude --version` cannot be started for any other reason THEN blocking (the io error is the source):
   `could not run "claude --version"; check the Claude Code install, then run rex init again`
3. **Version readable.** rex takes the first whitespace-separated token of the first line of stdout (`2.1.322 (Claude Code)` gives `2.1.322`) and reads it as three integers `X.Y.Z`. The exit status of `claude --version` is not used; only its stdout is read. IF that fails THEN blocking:
   `could not read the Claude Code version from "claude --version" (it printed: <first line, or "nothing">); run claude update, then run rex init again`
4. **Version not too old.** The version is compared with `MIN_CLAUDE_CODE_VERSION = 2.1.322` as numbers, part by part, not as text. IF it is older THEN blocking (OP5):
   `Claude Code <found> is older than 2.1.322, the oldest version SuperRex works with; run claude update, then run rex init again`
5. **GitHub tool.** IF `gh` is not on PATH THEN a warning on stderr:
   `rex init: warning: gh not found on PATH. Delivery works without pull requests until it is installed.`
6. **Sandbox dependencies, Linux only.** IF `bwrap` (package `bubblewrap`) or `socat` is not on PATH THEN a reported failure. The install runs in full, and the message is given at the end (section 9). One message names each missing tool:
   `the Claude Code sandbox cannot start on this machine: <bwrap | socat | bwrap and socat> not found on PATH, so Claude Code will refuse to start in this repo until they are installed; install them with sudo apt-get install bubblewrap socat (Ubuntu or Debian), or sudo dnf install bubblewrap socat (Fedora)`

The seccomp filter of the sandbox is optional, so rex does not check it. `claude --version` is the one command the preflight runs. Every other tool the preflight looks for is found by a look at the folders on PATH, and it is not run.

## 3. Bundle layout

The bundle is embedded in the binary when it is built. It has two roots, each embedded with its own `include_dir!`: every file under `.claude/skills/` and every file under `.claude/agents/` of the super-rex repo. No other part of `.claude/` is embedded, so a stray file there (`settings.local.json`, `worktrees/`) cannot ship.

| Bundle path | Installed at |
| --- | --- |
| `skills/<skill>/<file>` | `<repo>/.claude/skills/<skill>/<file>` |
| `agents/<file>` | `<repo>/.claude/agents/<file>` |

- Which skills ship is decided by Lanes 8, 9 and 10 and by C7. Whatever is in the repo's `.claude/skills/` ships. The code holds no list. There is no `INDEX.md` (C7, OP7 settled).
- `bundle::walk` skips every file or folder whose name starts with `.` (for example `.DS_Store`). A test asserts that no path in the walk has such a part.
- `agents/` holds exactly the 20 grid files `model-<low|medium|high|xhigh>-effort-<low|medium|high|xhigh|max>.md`.

Install rules, carried from rex-skills:

- A missing file is written.
- A file equal to the bundle is skipped.
- A file that differs is skipped and counted. It is overwritten only with `--force`.
- A file that cannot be read is skipped, with a warning on stderr: `rex init: skipped <path>: <io error>`. It is not counted as differing.
- Nothing is written through a symbolic link at or below a skill folder, or at an agent file. Warning on stderr: `rex init: skipped <path>: it or a folder above it is a symbolic link`.
- The folders `.claude/skills` and `.claude/agents` themselves are followed when they are links, because people link a shared folder on purpose.
- A file in those folders that is not a bundle path is never read or touched. The skills of an older rex-skills install (`rex-code-lead`, the utilities) stay. An older install's `INDEX.md` is left alone.

## 4. Settings

File: `<repo>/.claude/settings.local.json`. The folder `.claude/` is created if it is absent.

What rex adds (R) is the safety settings of section 4.1, then the key `hooks` with the C3 hook registration as its value. With `--no-hooks`, R has no `hooks` key.

### 4.1 The safety settings

As §5.7, with `<parent>` from section 4.7:

```json
{
  "sandbox": {
    "enabled": true,
    "failIfUnavailable": true,
    "allowUnsandboxedCommands": false,
    "enableWeakerNetworkIsolation": true,
    "filesystem": {
      "allowWrite": ["~/.cargo/registry", "~/.cargo/git", "~/.cargo/.package-cache", "~/.cargo/.package-cache-mutate", "~/.cargo/.global-cache"],
      "denyRead": ["~/.aws"]
    }
  },
  "permissions": {
    "additionalDirectories": ["<parent>"],
    "blockWritesOutsideWorkingDirectories": true,
    "deny": ["Bash(sudo *)", "Bash(git push --force*)", "Bash(cargo publish*)", "Bash(gh pr merge*--admin*)"]
  }
}
```

The `~/` entries are written as `~/...`, not expanded. `<parent>` is an absolute path. This is Proposed (section 11).

The deny rule `Bash(gh pr merge*--admin*)` keeps a protected main from being bypassed (§4).

`rex init` writes no `env` key. From a sibling repo, a `rex` command sets `REX_STORE` itself (C2 2.7, C1 3.2). IF a live check shows that `env` in the settings file reaches the Bash tool and the hooks, THEN a change request to Lane 1 can add `env.REX_STORE`.

### 4.2 The merge rule

One rule over the whole JSON tree. U is the user's file, R is what rex adds.

1. IF U is absent THEN rex writes R. Output: `created`.
2. An object in R: for each key, IF the key is absent in U THEN it is added after U's keys, in R's order. ELSE the two values are merged by these rules.
3. An array in R: each element of R that has no deep-equal element in U is appended. U's elements and their order stay first. This covers `allowWrite`, `denyRead`, `additionalDirectories` and `deny`. Each `hooks.<Event>` array of matcher groups follows rule 3a.

   3a. For `hooks.<Event>`: IF any hook in any group of U's `hooks.<Event>` array has the C3 command string, THEN nothing is added for that event. ELSE R's group is appended. A rex group already in the file is never changed or removed.
4. A scalar in R, and the key in U with the same value: nothing.
5. A scalar in R and the key in U with a different value, or a value in U of another type (object, array, scalar): U's value is kept, with the warning of section 4.3.
6. IF nothing is to be added THEN the file is not written at all, and it is not reformatted. Output: nothing. This is the second-run guarantee.
7. IF something is to be added THEN rex writes the merged object: `serde_json` pretty JSON, 2-space indent, a trailing newline, U's key order kept. Output: `updated`.
8. The file is written in place, not through a temporary file and a rename.

`--force` never touches this file.

Worked example. The repo is `/Users/me/code/app`, and the user runs `rex init --no-hooks`. The file before:

```json
{
  "permissions": {
    "allow": ["Bash(npm test)"],
    "deny": ["Bash(rm -rf *)", "Bash(sudo *)"]
  },
  "sandbox": {
    "enabled": false
  },
  "model": "opus"
}
```

The file after:

```json
{
  "permissions": {
    "allow": [
      "Bash(npm test)"
    ],
    "deny": [
      "Bash(rm -rf *)",
      "Bash(sudo *)",
      "Bash(git push --force*)",
      "Bash(cargo publish*)",
      "Bash(gh pr merge*--admin*)"
    ],
    "additionalDirectories": [
      "/Users/me/code"
    ],
    "blockWritesOutsideWorkingDirectories": true
  },
  "sandbox": {
    "enabled": false,
    "failIfUnavailable": true,
    "allowUnsandboxedCommands": false,
    "enableWeakerNetworkIsolation": true,
    "filesystem": {
      "allowWrite": [
        "~/.cargo/registry",
        "~/.cargo/git",
        "~/.cargo/.package-cache",
        "~/.cargo/.package-cache-mutate",
        "~/.cargo/.global-cache"
      ],
      "denyRead": [
        "~/.aws"
      ]
    }
  },
  "model": "opus"
}
```

`Bash(sudo *)` is not added twice. `sandbox.enabled` keeps `false`, with the warning below. stdout gets `rex init: .claude/settings.local.json updated`. Without `--no-hooks`, a `hooks` key would be added after `model`.

### 4.3 Conflicts

When rule 5 keeps a value of the user's, stderr gets (values as compact JSON):

`rex init: warning: kept <dotted.key> = <U value> in .claude/settings.local.json; SuperRex sets <R value>.`

For a safety key (`sandbox` or any key below it, and `permissions.blockWritesOutsideWorkingDirectories`), the same line goes on:

` The protection this key gives is off until you change it.`

Example: `rex init: warning: kept sandbox.enabled = false in .claude/settings.local.json; SuperRex sets true. The protection this key gives is off until you change it.`

This is Proposed (section 11).

FOR THE OWNER: the reviewer proposes a reported failure (exit 1) for this case. The warning stays until the owner decides.

### 4.4 A file rex cannot merge

In each case the file is left as it is and nothing is written to it. The rest of the install runs, and the run ends as a reported failure (section 9).

- Not valid JSON (the serde error, with its line and column, is the source):
  `.claude/settings.local.json is not valid JSON and was left as it is, so the settings SuperRex needs are not set; correct it, then run rex init again`
- Valid JSON, but the top level is not an object:
  `.claude/settings.local.json is not a JSON object and was left as it is, so the settings SuperRex needs are not set; make its top level an object, then run rex init again`
- Cannot be read (any error other than `NotFound`; the io error is the source):
  `.claude/settings.local.json could not be read and was left as it is, so the settings SuperRex needs are not set; make it readable, then run rex init again`

### 4.5 Links

rex reads through a symbolic link, which is safe. IF something is to be added AND (the file is a link OR `.claude` is a link) THEN nothing is written, and the run ends as a reported failure:

`<link> is a symbolic link, and rex does not write through one, so the settings SuperRex needs are not set; add them to .claude/settings.local.json where the link points, then run rex init again`

`<link>` is `.claude/settings.local.json` or `.claude`, whichever is the link.

IF nothing is to be added THEN the link is fine, and there is no message. A user who merged by hand is not blocked.

### 4.6 `--no-hooks`

- R has no `hooks` key. The safety settings are written as usual.
- Hook entries already in the file, from an earlier run, are never removed.
- IF any matcher group of the C3 registration is already in the file (a hook with the C3 command string, in any `hooks.<Event>` array) THEN stderr gets:
  `rex init: warning: hooks left out, but hook entries from an earlier run are still in .claude/settings.local.json. Remove the entries that run rex hook to turn telemetry off.`

### 4.7 The parent folder

- `<parent>` is the parent folder of `fs::canonicalize(<repo>)`, as a string.
- Let `home` be `std::env::home_dir()` made canonical, or as it is if that fails. IF `home` is unknown, OR `home.starts_with(<parent>)` (a `Path` comparison, by component) THEN `<parent>` is not added. This covers `$HOME` itself, `/Users` or `/home` for a repo that is the home folder, and `/`. IF the repo has no parent THEN nothing is added.
- When it is not added, stderr gets:
  `rex init: warning: the parent folder <parent> holds your home folder, so it was not added as a working directory. Agents can write only inside this repo. Move the repo into a folder of its own to let agents edit sibling repos.`
- Known limit: when the repo is moved, the next run adds the new parent (rule 3). The old entry stays.

This is Proposed (section 11).

## 5. Store and library

- `rex init` calls `StorePaths::locate(cwd)`, then `Store::create(&paths)` (C1 sections 3.1 and 7.2). C1 owns `.rex/`, its file names, the schema and the version check.
- `create` makes `.rex/` and `.rex/orchestrators/` (the library folder of §5.5 and §5.12) if they are absent; `rex init` makes neither itself.
- rex checks for `.rex` before the call, and IF it was absent THEN stdout gets `rex init: .rex/ created`.
- A `StoreError` is a reported failure, with C1's text as its source.
- On a store at version 1, `create` writes nothing to `.rex/` (C1 4.3).

## 6. The ignore block

rex makes sure `.gitignore` holds this block. The file is created if it is absent. The block is found by its first line, the marker.

```
# rex:ignore superrex
.claude/skills/
.claude/agents/
.claude/settings.local.json
/.rex/
```

- Absent file: created with the block. Output `created`.
- File without the marker: rex adds a newline IF the last byte is not one, then one blank line, then the block. Output `updated`.
- File with the marker: left alone. Output nothing.
- Link: left alone, with the warning `rex init: skipped <path>: it is a symbolic link` on stderr.
- The rex-skills block (`# rex:ignore claude`) is a different marker. It is left as it is, and the new block is still added. A line in both blocks does no harm in `.gitignore`.
- Whole folders are ignored, not each installed path. A user's own untracked skill or agent in those folders is hidden too. A file that is already tracked stays tracked. To keep a new one in Git, run `git add -f <path>`, or add `!.claude/skills/<name>/` after the block.
- Not ignored: `AGENTS.md`, `.gitignore`, `.pre-commit-config.yaml` (OP6, settled), and `CODEBASE.md` (no evidence yet).
- C12 section 5 puts job worktrees under `.rex/worktrees/`, which `/.rex/` covers, and uses no `isolation: "worktree"`. `.claude/worktrees/` stays out of the block.

## 7. `AGENTS.md` sections

The marker is `<!-- rex:codebase -->`, as in rex-skills. The rex text is, in this order: the marker line, the codebase section, one blank line, the smells section. As bytes: `<!-- rex:codebase -->\n` + codebase section + `\n` + smells section. Each section ends with one newline.

The codebase section (Task D §9, byte for byte):

```markdown
## Codebase map

Run one of these commands before you search the tree by hand. Each one prints a tree of the working directory and skips the files that git ignores.

- `rex codebase`: every file.
- `rex codebase --only code`: only source files (Rust, Go, C, C++, Python, JavaScript, TypeScript, shell).
- `rex codebase --only go`: only one language. `--rust-only` is the same as `--only rust`.
- `rex codebase --with-context`: adds the first sentence of each source file's opening comment to its line.
- `rex codebase --only code --with-context`: the source files and what each one is for. Start here.

Add `--for-human` to write the tree to `CODEBASE.md` instead of printing it, for example `rex codebase --for-human --only code --with-context`.

Every source file opens with a comment of one or two lines that says what the file is for. A file with no sentence after its name in the tree is missing that comment. When you create or change a file, write the comment or bring it up to date.
```

The smells section (rex-skills `SMELLS_SECTION_BODY`, unchanged, because Task D §8 keeps it saying that `rex smells` reads Rust files):

```markdown
## Code smells

Run this command to find each known problem that was flagged and left in the code. It reads every `.rs` file and skips the files that git ignores.

- `rex smells`: one `path:line` for each `SMELL` flag comment in the Rust files, then the count of them all.
```

- Absent file: created with the rex text. Output `created`.
- File without the marker: the rex text goes on top, then one blank line, then the old bytes. Output `updated`.
- File with the marker: left alone, including an older rex-skills section. Output nothing.
- Link: left alone, with the warning `rex init: skipped <path>: it is a symbolic link` on stderr.

## 8. Pre-commit config

File: `.pre-commit-config.yaml`. Marker: the first line, `# rex:pre-commit`.

- Written only IF the file is absent. An existing file, with or without the marker, is never replaced, with or without `--force`.
- Link: left alone, with the warning `rex init: skipped <path>: it is a symbolic link` on stderr.
- On each run, IF `pre-commit` is on PATH THEN rex runs `pre-commit install --hook-type pre-commit --hook-type pre-push`. Its output is not shown.

The text is rex-skills `CONFIG` with two changes: the header says `rex init` in place of `rex skills`, and trivy's `--skip-dirs` is `target,node_modules,.claude,.rex` in place of `target,node_modules,.claude,.codex,.agents`. Reason: there are no vendor folders (§4 Vendors), and the store is not source to scan.

```yaml
# rex:pre-commit
# Installed by `rex init`. Needs pre-commit, gitleaks and trivy on PATH.
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v6.0.0
    hooks:
      - id: check-added-large-files
        args: [--maxkb=1024]
      - id: check-merge-conflict
      - id: check-toml
      - id: check-yaml
      - id: detect-private-key
      - id: end-of-file-fixer
      - id: trailing-whitespace

  - repo: local
    hooks:
      - id: gitleaks
        name: gitleaks
        entry: gitleaks git --pre-commit --staged --redact --verbose
        language: system
        pass_filenames: false
      - id: trivy
        name: trivy secrets, vulnerabilities and misconfigurations
        entry: trivy fs --timeout 15m --quiet --skip-version-check --scanners secret,vuln,misconfig --severity HIGH,CRITICAL --exit-code 1 --skip-dirs target,node_modules,.claude,.rex --skip-files .env .
        language: system
        pass_filenames: false
        stages: [pre-push]
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt --all -- --check
        language: system
        types: [rust]
        pass_filenames: false
      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy --workspace --all-targets -- -D warnings
        language: system
        types: [rust]
        pass_filenames: false
      - id: cargo-test
        name: cargo test
        entry: cargo test --workspace
        language: system
        types: [rust]
        pass_filenames: false
        stages: [pre-push]
```

When the config was created on this run, or holds the marker, rex warns on stderr for each hook tool not on PATH:

- `` rex init: warning: `gitleaks` not found on PATH. Its hook in .pre-commit-config.yaml fails every commit until it is installed ``
- `` rex init: warning: `trivy` not found on PATH. Its hook in .pre-commit-config.yaml fails every push until it is installed ``

An existing config of the user's (no marker) gets no tool warnings.

## 9. Output and exit

The order of the work is the order of the lines. Each line starts with `rex init:`. stdout:

1. The preflight (section 2). It prints nothing on stdout. A blocking failure stops here. A preflight warning (`gh`) goes to stderr at once, when its check runs. Reported failures go to stderr at the end, after the last stdout line, in the order they happened.
2. `rex init: .claude/settings.local.json created`, or `... updated`, or nothing IF unchanged. This comes first, straight after the preflight.
3. One line for skills and agents together: `rex init: <w> written, <s> skipped`, or `rex init: <w> written, <o> overwritten, <s> skipped` IF any file was overwritten. IF any file differs and was left alone, the line goes on: ` (1 differs from this version; use --force to overwrite)`, or ` (<d> differ from this version; use --force to overwrite)`.
4. `rex init: AGENTS.md created`, or `... updated`, or nothing.
5. `rex init: .gitignore created`, or `... updated`, or nothing.
6. `rex init: .rex/ created`, the first time only.
7. The config line: `rex init: .pre-commit-config.yaml created`, or `rex init: .pre-commit-config.yaml skipped (already exists)` for a user's own config, or nothing when it holds the marker or is a link.
8. The hooks line: `rex init: pre-commit hooks installed`, or `rex init: pre-commit hooks skipped (pre-commit not found; install it, then run: pre-commit install --hook-type pre-commit --hook-type pre-push)`.

The rex-skills line `pre-commit hooks skipped (no .git directory)` cannot occur, because the preflight requires a `.git` directory. A closed stdout does not stop the install (carried).

First run in a new repo, with `pre-commit` on PATH (`<n>` is the number of bundle files):

```
rex init: .claude/settings.local.json created
rex init: <n> written, 0 skipped
rex init: AGENTS.md created
rex init: .gitignore created
rex init: .rex/ created
rex init: .pre-commit-config.yaml created
rex init: pre-commit hooks installed
```

Second run in the same repo:

```
rex init: 0 written, <n> skipped
rex init: pre-commit hooks installed
```

The second run changes no file in the repo.

Exit codes are those of C11 part 4. Every failure of `rex init` exits 1. No `InitError` variant is a usage error (`is_usage` is false). `InitError` is Lane 4's own type in `src/commands/init/error.rs`, added by the rule of C11 part 6. A wrong flag is clap's own exit 2.
Each failure text in this contract is the `Display` of an `InitError` variant, written by C11 part 5: lower-case first letter, no `rex init:` prefix, no full stop at the end, the fix after `; `, and the io, serde or C1 error as `#[source]`.

- **Blocking** (section 2, checks 1 to 4): `run` returns the variant at once. Nothing is written. stderr gets `error: <text>[: <source>]`.
- **Reported** (2.6, 4.4, 4.5, a C1 error, a failed `pre-commit install`, and a stop of this section): the install goes on. Each failure is kept in order. At the end, `run` prints each one on stderr as `rex init: failed: <text>[: <source>]`, then returns `InitError::Incomplete { count }`. Its text is `rex init did not finish <count> step(s); each is named above`.
- **Warning**: stderr only, `rex init: warning: …`. Exit 0.

The `pre-commit install` failure text (its stderr, as one line, is the source): `` `pre-commit install` exited with <status>; fix what it names, then run rex init again ``.

Carried from rex-skills: any other I/O error (a folder or file that cannot be created or written, an `AGENTS.md` or `.gitignore` that cannot be read) stops the run at that step. It is added to the reported list as the last item, and `run` ends as for reported failures. The failures collected before the stop are always printed. Because the settings come first, such a stop never leaves the grid installed without the safety settings.

## 10. Open points

### OP5 — settled

Question (plan 4.1, point 5): the minimum Claude Code version. §4 says v2.1.293. §5.11 says the later of that and the version SuperRex is tested against, which is v2.1.322.

Answer: `MIN_CLAUDE_CODE_VERSION = 2.1.322`, one constant. IF the final gate runs on a later version THEN Lane 1 raises the constant to that version.

Reason: §5.11 is the later and more specific rule, and the checks ran on 2.1.322 (§0.6, §8). Every Claude Code behaviour SuperRex relies on was seen only on 2.1.322: the hook fields, `enableWeakerNetworkIsolation`, `blockWritesOutsideWorkingDirectories`, the deny rules and relative path resolution (checks 1 to 15). `failIfUnavailable` is from the docs and was not tested on any platform. P0.5 is its first run on macOS, and P0.7 on Linux. 2.1.293 only proves that the 20 grid pairs are valid. Inferred, not tested: an older version that does not know a setting such as `failIfUnavailable` may ignore it, which is the fail-open of challenge 1.

### OP6 — settled

Question (plan 4.1, point 6): the ignore block. §4 says nothing SuperRex adds goes into Git. A user may keep skills and agents of their own in the same folders. `rex init` also changes `AGENTS.md`, `.gitignore` and the pre-commit config, which are normally committed.

Answer: the block of section 6, with the new marker `# rex:ignore superrex`. It ignores whole folders. It does not ignore `AGENTS.md`, `.gitignore` or `.pre-commit-config.yaml`.

Reason: whole folders keep out of Git the skills the builder writes and anything an agent adds there, which are also "something SuperRex adds" (§4). A list per path would let those in, and it would go stale at each upgrade. The cost, a user's own untracked skill being hidden, has a one-line fix (section 6). A new marker lets a repo with the rex-skills block get the new entries (item 4.8). The three project files are the user's, normally committed: rex only adds a marked block to two of them and creates the third only IF it is absent. Ignoring a tracked file has no effect, and ignoring `.gitignore` would hide the block itself. Committing them is the user's choice.

### OP14 — settled

Question (plan 4.1, point 14): `claude` not on `PATH`. §5.11 compares `claude --version` with a minimum. The sources do not say what `rex init` does when the command is absent.

Answer: a blocking error, with the message of section 2, check 2. There is no option to skip it.

Reason: the version check is a safety check (OP5). Without `claude`, rex cannot know that the safety settings will be honoured. The fix is one install. Item 4.2 already assumes this: the tests put a stub `claude` on PATH because §5.11 makes it a requirement.

## 11. Other proposed decisions

- **Repo root only (section 2, check 1). Proposed.** Reason: the store, the settings and the skills belong to the main checkout (§5.12). A worktree is removed after a job (§5.16), with whatever was written in it. One rule with no search up the tree is predictable, and rex needs no `git` on PATH.
- **Linux sandbox tools missing is reported, not blocking (section 2, check 6). Proposed.** Reason: P0.7 needs both "rex init reports it" and "Claude Code refuses to start". Claude Code refuses only IF `failIfUnavailable: true` is written. A blocking check would leave the repo with no sandbox settings, so a session would start unsandboxed in bypass mode (challenge 1). That is the worst case.
- **A conflicting user value is kept, with a warning (section 4.3). Proposed.** Reason: the lane outcome "never replaces what the user has", and §5.7 "must merge into ... not replace". The warning makes a weaker setting visible.
- **`~/` paths are written as `~/...` (section 4.1). Proposed.** Item 4.5 and §5.7 say "absolute". Reason: `~` is the form checks 11 and 12 tested, and it does not depend on the launch folder. The problem of check 13 is with `..` and plain relative paths. `<parent>` is absolute.
- **A parent that holds the home folder is not added (section 4.7). Proposed.** Reason: §5.7 opens the parent so that agents can edit sibling repos. A repo directly in `~`, or the home folder itself, would make the whole home folder writable (`~/.ssh`, `~/.cargo/bin`, shell profiles). The owner asked for hard rules against catastrophic actions such as removing the user directory (§A.3). This case is inferred; the sources do not name it.
- **`--no-hooks` removes nothing (section 4.6). Proposed.** Reason: least code, and a removal is a delete in a file the user owns. §4 "Telemetry on or off" says only whether the hooks are installed.
- **`AGENTS.md`, `.gitignore` and the pre-commit config stay in Git (section 6). Settled (OP6).** This departs from the letter of §4 ("Nothing SuperRex adds"). Reason: see OP6.
- **Opening an existing store changes no file (section 5). Proposed.** Reason: without it, the second run changes files in `.rex/`, and P0.1 fails.
- **The settings come first (section 9). Proposed.** Reason: every later step can fail with an error. A stop before the settings would leave the grid installed with no sandbox settings: the fail-open of challenge 1.

Not tested: `~/` entries in a session started from a subfolder; an absolute `additionalDirectories` entry; `failIfUnavailable`; a deny rule with `*` in the middle. P0.5 starts one session from a subfolder.

P2 adds one case: `gh pr merge <n> --squash --admin` is refused in bypass mode.

## 12. What other lanes must know or provide

- **Lane 2 (C1).** Met by C1 4.3. The second-run test compares `.rex/` byte for byte and opens the store only through `rex`.
- **Lanes 2 and 9.** The orchestrator library folder is `.rex/orchestrators/`, made by `Store::create` when `rex init` runs. Use this one path.
- **Lane 3 (C3).** C3 gives the registration as one `serde_json::Value` and the command string as a constant.
- **Lane 10 (C12).** C12 section 5 puts job worktrees under `.rex/worktrees/`, which `/.rex/` covers, and uses no `isolation: "worktree"`. `.claude/worktrees/` stays out of the block.
- **Lane 1.**
  - `Cargo.toml` `include` lists `.claude/skills/**/*` and `.claude/agents/**/*` in place of `.claude/**/*`. `build.rs` needs no change: `rerun-if-changed=.claude` covers both folders.
  - `serde_json` with the feature `preserve_order`, so that the merge keeps the user's key order. The feature applies to the whole crate: every lane's JSON output then keeps insertion order in place of sorted keys (for example `rex jobs --json` and `rex job show`). Granted (C11 part 7).
  - `rex init` routes to `commands::init::run(&cwd, options)` with `InitOptions { is_forced, is_without_hooks }`. `rex skills` is Lane 6's group. It is no longer an alias of `rex init`.
- **Lanes 6 and 8 (C7).** The bundle ships whatever is in `.claude/skills/` and the 20 files in `.claude/agents/`. There is no `INDEX.md` (C7, OP7 settled).

## 13. Source map

| Source section | Where C6 covers it |
| --- | --- |
| §2.1 installer rules (embedded skills; never replaced without `--force`; nothing written through a link; three project files changed once, by a marker) | 1, 3, 4.5, 6, 7, 8 |
| §4 Vendors | 1, 8 |
| §4 Minimum Claude Code version | 2 (check 4), 10 OP5 |
| §4 What goes into Git in a target repo | 6, 10 OP6, 11 |
| §4 Telemetry on or off | 1, 4.6, 11 |
| §5.6 hook registration by `rex init` | 4 (C3 named), 12 |
| §5.7 safety settings, absolute paths, the parent folder | 4.1, 4.7, 11 |
| §5.7 merge into the user's file, not replace | 4.2, 4.3, 11 |
| §5.10 `rex init` row | 1, 3 to 9 |
| §5.11 Version | 2 (checks 2 to 4), 10 OP5, OP14 |
| §5.11 GitHub tool | 2 (check 5) |
| §5.11 Repo | 2 (check 1), 11 |
| §5.11 Sandbox (Linux dependencies) | 2 (check 6), 9, 11 |
| §5.12 storage and ignore entries | 5, 6, 12 |
| §5.16 a worktree is removed after a job | 11 (repo root only) |
| Task D §9 | 7 (codebase section) |
| Task D §8 | 7 (smells section unchanged) |
| Challenge 1 | 2 (check 6), 9, 10 OP5, 11 |
| Check 11 | 4.1 (Cargo folders, `~/` form) |
| Check 12 | 4.1, 11 (`~/` form, write block) |
| Check 13 | 4.1, 4.7 (absolute parent) |
| Check 14 | 6, 12 (`.claude/worktrees/`) |
| Build plan 4.3, 4.4 | 3 |
| Build plan 4.5 | 4.1 to 4.5 |
| Build plan 4.6 | 1, 4.6 |
| Build plan 4.7 | 5 |
| Build plan 4.8 | 6 |
| Build plan 4.9 | 7 |
| Build plan 4.10 | 2 |
| Build plan P0.1 | 4.2 (rule 6), 5, 9 |
| Build plan P0.7 | 2 (check 6), 11 |
| OP5, OP6, OP14 | 10 |

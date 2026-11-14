# Upgrade from 0.7 to 0.8

SuperRex 0.8.0 is a Claude Code plugin with a mod inside it. In 0.7, `rex init` copied the skills and agents into each repo and registered three `rex` hooks. In 0.8, the plugin carries the skills, the library and the grid agents, and the mod does the work of the hooks. This guide moves a repo that 0.7 set up to 0.8.

Do these steps once on your machine. Then, in each repo, `rex migrate` moves the store and one run of `rex init` does the rest. You do nothing by hand unless you changed a file that 0.7 installed.

## Before you start

Finish or close each job that a 0.7 session still runs. A 0.7 builder calls `rex job create`, `rex job plan` and `rex job close`, and 0.8 has none of these commands, so a 0.7 job cannot finish after the new binary is installed.

## On your machine

1. Install the new binary. `cargo install` replaces the 0.7 binary:

   ```sh
   cargo install super-rex
   ```

2. Make sure that the `rex` first on your `PATH` is the new one:

   ```console
   $ rex --version
   rex 0.8.0
   ```

   The mod calls `rex` by `PATH`, and the plugin and the binary must have the same version.

3. Make sure that Claude Code is 2.1.323 or later (`claude --version`). `rex init` refuses an older version.

You do not install the plugin by hand. `rex init` installs it in each repo.

## In each repo

Run each command yourself, in your own shell, in the root of the main checkout. Do not ask an agent to run them: the sandbox of a session cannot write the Claude Code settings files.

### 1. Move the store

0.7 kept a store in `.rex/rex.db` in every repo. Its form is version 1, and every 0.8 command except `rex init` and `rex migrate` refuses it with `error: store is version 1; run rex migrate`. Move it to version 2:

```console
$ rex migrate
store=v2 migrated=yes jobs=12
```

`rex migrate` runs in one transaction and keeps each job. A job that a person asked for gets the origin `composer`, any other the origin `sdk`, and a spend limit that you set moves into the job. A second run changes nothing and prints `store=v2 migrated=no`. A repo with no `.rex/rex.db` skips this step.

If you run `rex init --with-storage` before `rex migrate`, it does every other step, then ends with `rex init: failed: the store could not be set up: store is version 1; run rex migrate` and exit 1. Run `rex migrate`, then `rex init --with-storage` again.

### 2. Run `rex init`

0.7 always kept a store. In 0.8 the store is a choice. To keep recall, feedback, the cost of record and master runs, turn it on:

```sh
rex init --with-storage
```

Without `--with-storage`, the record of a job lives only in the session and ends with it, and the store from step 1 stays on disk unused. You can turn storage on later with another run of `rex init --with-storage`.

`rex init` does these things by itself:

- It installs the `super-rex` plugin at project scope, with `claude plugin marketplace add coderaidershaun/super-rex --scope project` and `claude plugin install super-rex@super-rex --scope project`.
- It removes the hook entries that 0.7 wrote in `.claude/settings.local.json` for `rex hook`, `rex gate` and `rex test-hook`. Those commands are gone, and each entry would fail on every event. Your own hooks stay.
- It adds the new deny rules, the sub-agent depth 4 and the rest of the 0.8 settings, and keeps every value of yours.
- It writes `.rex/config.json`, which the mod reads when a session starts.
- It removes the stopgap entries `git *` and `git:*` from `sandbox.excludedCommands`, and `Bash(git *)`, `Bash(git:*)` and `Bash(cd <parent>/*)` from `permissions.allow`, if you added them, with one line for each. They ran git and its hooks outside the sandbox. In their place it lists each job worktree in `<repo>.worktrees/` in `permissions.additionalDirectories`, so that `git commit` works in a worktree inside the sandbox with its hooks still sandboxed.
- It removes each copy that 0.7 put in `.rex/skills/` (the library), `.claude/skills/` (the 6 skills) and `.claude/agents/` (the 20 grid agents), if you never changed it. A copy in `.rex/skills/` would hide the plugin's skill, because `rex_skill` reads that folder first. A copy in `.claude/` would sit beside the plugin's in the session's list, and the 0.7 `/orchestration-builder` calls commands that 0.8 does not have. Skills of your own in these folders are left alone.
- It replaces the 0.7 block in `.gitignore` with the 0.8 block. The 0.7 block ignored `/.rex/*`, `.claude/skills/` and `.claude/agents/`, so Git ignored `.rex/config.json` and `.rex/skills/`, which 0.8 tracks.

Each removal of a stopgap entry, a copy or the block is one line on the output:

```console
rex init: removed "git *" from sandbox.excludedCommands in .claude/settings.local.json: it ran git and its hooks outside the sandbox, and rex now registers its worktrees, so git works inside it.
rex init: removed .rex/skills/rex-code-commenting/: it was an unchanged copy from an older rex, and the plugin ships rex-code-commenting now
rex init: removed .claude/skills/rex-grill-me/: it was an unchanged copy from an older rex, and the plugin ships rex-grill-me now
rex init: removed .claude/agents/model-low-effort-low.md: it was an unchanged copy from an older rex, and the plugin ships model-low-effort-low.md now
rex init: replaced the rex block in .gitignore with this version's: the old one also ignored .rex/config.json, .rex/skills/, .claude/skills/ and .claude/agents/
```

#### If you changed a copy or the block

`rex init` never deletes your changes. A copy that differs from the one 0.7 installed is kept, with a warning that says what it does and what to do:

```console
rex init: warning: kept .claude/skills/rex-rules/: it differs from the copy an older rex installed, so the session lists it beside the rex-rules skill that the plugin ships; to use the plugin's, move your changes out and delete that folder
```

To keep your change, move it into a skill of your own in `.rex/skills/<name>/`, under a name that the plugin does not ship. Then delete the folder or file that the warning names.

A 0.7 ignore block that you changed is kept as well. The warning lists each line to delete and each line to add:

```console
rex init: warning: kept the rex block in .gitignore, which differs from the block any rex wrote; to finish the upgrade, change it by hand:
rex init:   delete the line /.rex/*
rex init:   add the line /.rex/rex.db*
rex init:   add the line /.rex/events.jsonl
...
```

Make those changes in `.gitignore`. A warning never fails the run, and `rex init` exits 0. If a folder that it would look into is a symbolic link, it does not look into it, because the folder can be shared with other repos, and it says so.

### 3. Commit

Commit the files that `rex init` names in its commit line. A job works in the main checkout only when it is clean:

```sh
git add .rex/config.json .gitignore && git commit -m "chore: move SuperRex to 0.8"
```

Add `AGENTS.md` and `.pre-commit-config.yaml` too if `rex init` created or changed them. If `.rex/skills/` holds skills of your own, such as skills a job wrote, gap skills or forks that feedback made, commit them as well: 0.8 tracks them.

### 4. Start a new session

Claude Code loads a plugin when a session starts. Start a new session in the main checkout, then run:

```
/rex doctor
```

The answer starts with `doctor=ok` and ends with `mod=0.8.0`. A `problem=` line says what to fix, such as a binary and a plugin of different versions.

## What changes when you use it

| In 0.7 | In 0.8 |
| --- | --- |
| `/orchestration-builder <request>` | `/super-rex:orchestration-builder <request>` |
| `/rex-deliver`, `/rex-feedback`, `/rex-pr-review` | `/super-rex:rex-deliver`, `/super-rex:rex-feedback`, `/super-rex:rex-pr-review` |
| Grid agents `model-<tier>-effort-<level>` | `super-rex:model-<tier>-effort-<level>` |
| `rex job create`, `rex job plan`, `rex job close` | The builder calls the mod's tools `rex_job_open`, `rex_job_plan` and `rex_job_close`. `rex job show` stays. |
| `rex hook` and `.rex/events.jsonl` | The mod records each agent. With storage on, it sends the events to `rex ingest`. |
| `rex gate` | The mod's spend gate. Raise a limit with `/rex limit <job> <usd>` in the session, or `rex limit <job> --set <dollars>` at a shell. |
| `rex test-hook` | The mod cuts the passing lines of a test run. `rex test-filter` stays for a shell. |
| `rex init --force`, `rex init --no-hooks` | Gone: `rex init` copies no skill files and registers no hooks. |
| Skill files in `.claude/skills/` and `.rex/skills/` | The plugin's skills and library. `.rex/skills/` holds only the repo's own skills, and is in Git. |
| A skill file named by its path, such as `.rex/skills/rex-code-tdd/mocking.md` | Named `rex-code-tdd/mocking.md` and read with `rex_skill`. `rex skills check` reports the old form. |
| A merge with `gh pr merge` or a local squash merge | `rex_merge` only, as one squash commit pushed to the base. An agent's own `gh pr merge` and push to main are denied. |
| Job branch `<type>/<slug>-<job ID>` | `rex/<job ID>` |
| Job worktrees in `.rex/worktrees/<job ID>/` | `<repo>.worktrees/<job ID>`, beside the main checkout, made with `rex worktree add`. |
| The store, always | The store only with `rex init --with-storage`. |
| `rex watch` | `/rex watch` in the session. `rex watch` still works at a shell. |

New in 0.8: `/rex status`, `/rex jobs`, `/rex doctor`, `/rex prompts`, `/rex limit`, and master runs with `/rex master`, `/rex policy` and `/rex answer` (they need storage). <!-- T9: confirm after T7 --> The [README](../README.md) describes each one.

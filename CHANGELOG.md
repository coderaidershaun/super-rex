# Changelog

This file lists the changes to SuperRex that a user can see, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the version numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

What changes for you:

- **The skill library describes itself and answers as an MCP server.** Each of the 25 library skills declares what it is for in a `metadata` map of its header, the plugin starts a server that answers `head`, `search` and `get` over that library, and `rex skills check` checks the map. Another repo can make its own folder of skills searchable the same way: [docs/skill-library.md](docs/skill-library.md) walks through it.

### Added

- **A shared vocabulary, version 1.** `plugin/vocab/terms.json` lists the terms that a `metadata` map may use: 4 kinds, 8 categories, 6 facts, 3 scope keys, 8 activities, 13 touches, 2 qualities and 8 artefacts. `plugin/vocab/aliases.json` maps free words onto those terms, such as `p99` onto `latency`. The binary embeds both. `docs/skill-library-vocabulary.md` maps each criterion, row and file of the assignment table onto a term.
- **`rex skills check` reads the `metadata` map**, with rules M1 to M13: `kind` and `category` are terms, each claim and scope key is a term, `not_when` is there when a scope is, `intents` holds 3 to 5 phrases, `files` lists every file of the folder, each `open_when`, `requires` and `replaces` target exists, `max_topics` is a positive number, and the map holds at most 120 words. A wrong term is a finding that lists the allowed terms.
- **`rex skills check --pool <folder>` runs rule M14, the intent check.** Each intent phrase must find its own skill in the top five of a keyword search among the checked skills and the pool. Without `--pool`, a warning line says that the check was skipped. With fewer than 1,000 skills to search, a warning line says that the check is not a real test.
- **Every library skill has a `metadata` map**, with its kind, category, claims, placement, scope, `not_when`, intents, files and open-when items. They carry what the assignment table knew about each skill. Each instruction in a library file to open another skill, or a file of one, is declared as an `open_when` item of its skill, and a fitness test fails on one that is not.
- **The skill `rex-code-essentials`.** One page of the rules of `rex-code-philosophy`, `rex-code-commenting`, `rex-code-ergonomics` and `rex-code-error-writing` that every code task obeys, in any language, with the four rulebooks as its open-when items. The library now holds 25 skills.
- **The rules of the `metadata` map are written for skill writers.** `rex-skill-language` has a section `## METADATA`. A skill that a job's skill writer writes must carry the map, and a fork that `rex-feedback` makes keeps the map of the skill it forks.
- **`rex skills serve [--name <name>] [--library <folder>]...`.** An MCP server over stdin and stdout with three tools: `head` gives the library's name, version, number of skills and vocabulary version; `search` gives cards for request forms, never skill text; `get` gives one file and its SHA-256. A failed call answers with `isError` and a code, such as `not_found`, `bad_cursor` or `stale_cursor`. With several folders the first one wins for a skill name. With none, it serves the library built into the binary.
- **The plugin starts its library server.** `plugin.json` declares `mcpServers.library`, which runs `rex skills serve --name super-rex --library ${CLAUDE_PLUGIN_ROOT}/library`, so the server serves the installed plugin's own library.
- **Agents of a job cannot call the server's raw tools.** While a job is open, the mod refuses `mcp__plugin_super-rex_library__head`, `__search` and `__get` to the builder and to each sub-agent, with a refusal that starts `rex library guard:`. An agent reads a skill with `rex_skill`, as before. Outside a job, the tools work.
- **`rex skills search [--library <folder>]...`.** It reads the arguments of the `search` tool as JSON on stdin and prints the answer that the server gives. A search runs three lookups for each request form: A, the facts that a skill claims; B, the terms that a skill claims, with a check of its scope; C, a keyword search with SQLite FTS5 for free text and for words that are not terms. Each result gives the items with their tier, slot and reason, then `more` and a `cursor` for the rest, the `gaps`, the `unrecognised` words and the `dropped` skills. An answer is kept within a size limit.
- **`rex skills index [--library <folder>]...`** prints a static index of a library as one JSON document, with each skill's name, description, URL, SHA-256 digest and the fields that a search reads.
- **`rex doctor` checks the library server.** It prints a `library=` line with the name and version that the server answers to `head`, or `library=unreachable` and a `problem=library server: <cause>` line. `rex doctor --plugin <dir>` starts the server as the manifest of that plugin folder declares it, as Claude Code does, and also compares the manifest's version with the binary's in the `problem=version skew: …` line.
- **Guides.** `docs/skill-library.md` shows how to make a folder of skills searchable, with SuperRex's library as the worked example. `docs/skill-library-wp0.md` records the tests that this work rests on: FTS5 in the bundled SQLite, a nested `metadata` map that Claude Code loads with no warning, and a call from the mod to a server that the plugin declares.

### Unchanged

- Inside a job, the mod still assigns skills with the assignment table, the questions to the decision agent, and the cap of six read-first skills.
- `rex-code-essentials` is not yet assigned to any task.

### Known limits

- The plugin's server runs the first `rex` on your `PATH`. If that `rex` has no `skills serve`, the server does not start, and Claude Code shows only `status: failed` for it, with no message. `rex doctor --plugin <plugin folder>` reports it. `/rex doctor` does not pass `--plugin`, so it does not.
- `rex skills check` gives no finding for a library skill that has no `metadata` map.
- `head` ignores its `include` argument, so its answer never holds the vocabulary terms.

## 0.14.0 - 2026-11-25

Install the binary and the plugin of this version together, then start a new session.

What changes for you:

- **The skill `rex-simplify` makes a text clear and short for a person.** A small model that gets the text and nothing else reports each place that it did not understand. The session rewrites the text from that report, then cuts what the reader does not need.

### Added

- **The skill `/super-rex:rex-simplify [file or text] [for <reader>]`.** It takes an answer, a report, a document, an HTML page, a PDF or the comments of a code file. With no argument it takes the session's own draft, or its last reply to you. You start it, a session loads it by itself before it sends you a complex report, and an agent of a job reads it with `rex_skill`.
- **A test reader finds the gaps.** The session starts a Haiku agent, `super-rex:model-low-effort-low`, and gives it the text and one line about the reader: no request, no summary and no purpose, because each of those would hide a gap that the real reader still finds. The test reader answers in a fixed form: the main point in its own words, what the text asks of its reader, each gap with the exact words and its kind, and the parts it could skip. A gap is one of six kinds: a term that is not explained, a pointer to something outside the text, a missing step, a part that comes too late, a heavy sentence, or a vague claim.
- **Five stages.** The session writes the message and a word limit before the read, so that the test reader's report cannot bend them. The limit is half the words for a padded answer or report, and the same number of words for a document that readers look facts up in. Then the session keeps only the parts that the reader needs, fills each gap in them, and puts the message first and the steps in order. It cuts points, not grammar, so the result stays in full sentences. A new test reader checks a text that needed a large repair, for at most 2 rounds.
- **Shown in a real session.** A padded report of 371 words came back at 196. Its first four sentences give the cause, the fix, and the decision that the reader must make. The facts that the session could not find are listed as open. The two test reads and the rewrite cost $0.23 with the session on Sonnet.
- **`SKILL.md` is short, and `how-to.md` holds the method.** `SKILL.md` has the words, the inputs and the rules. `how-to.md` has the stages, the test reader's brief, the usual faults of each model tier, and the repair for each kind of gap. An agent reads it only when it runs the skill.

- **What the skill never does.** It never changes a fact, a number, a name, a command, a path, quoted text or what code does, and it never cuts a warning about a loss of data or funds. It only leaves out the parts that the reader does not need. A fact that it cannot find is listed as open, never invented. A file that Git does not track, or that has uncommitted changes, is not edited in place: the result goes to `<name>.simple.<ext>` beside it.

### Unchanged

- No row of the assignment table gives `rex-simplify` to a task, so no job starts a test reader unless you or a decision agent ask for one. The table names the skill under "Not in table", and a decision agent can give it by an override to a task that writes a document or a report for a person.

### Known limits

- Shown in a real session: the skill started by `/super-rex:rex-simplify` on a file, and loaded by a session for its own answer. Not shown: the skill inside a job, where the mod checks the test reader's brief. That brief passes the mod's check in a job with no plan; in a job with a plan the first start is refused once, and the second start passes.
- The test reader is a model, not a person. It knows more terms than most readers, so it finds missing steps and undefined project names better than it finds jargon.

## 0.13.0 - 2026-11-25

Install the binary and the plugin of this version together, then start a new session.

What changes for you:

- **`rex init --auto-merge` lets every job merge with no merge question.** A builder's own job and each job of a master tree merge to the base branch once their checks pass, so a run that nobody watches no longer waits on a merge answer.

### Added

- **`rex init --auto-merge` and `--no-auto-merge`.** The first writes `"auto_merge": true` to `.rex/config.json`, the second `false`. A run with neither flag keeps what the config holds, and a new config gets `false`. While auto-merge is on, each run of `rex init` prints a line that says so.
- **A builder's own job merges with no question.** In a repo with auto-merge, `rex_job_open` answers `merge=yes` for a job with no other answer, so the builder skill asks no merge question, also in a headless run.
- **Each job of a master tree merges with no question.** `rex_build` gives each job the merge answer `yes`, whatever its consequence. The policy's `merge: ask`, `merge_max_consequence` and `always_ask_paths` ask nothing. `merge_checks` still run before each merge, and `merge_after` still orders them. `rex_tree_open` answers `merge=auto`.
- **The first master tree of a repo with auto-merge asks no policy question.** The mod stores and confirms the default policy with `merge: allowed_if_checks_pass`.
- **A job opened before auto-merge was on takes the answer at its next `rex_merge`**, so a tree that stalled on merges goes on after `rex init --auto-merge` and a resume in a new session. A merge question that already waits stays open until you answer it.

### Unchanged

- Your own `merge: no`, the answer `Keep the branch`, and a policy with `merge: never` all hold over auto-merge.
- `rex_merge` is still the only path to the base branch, with every check it made before. A base branch that takes no direct push still ends as a kept branch.
- The store's form: an auto-merge answer is recorded as source `policy` with no policy hash.

## 0.12.0 - 2026-11-19

Install the binary and the plugin of this version together, then start a new session.

What changes for you:

- **A master tree asks for its budget and its time when it opens.** Two dialogs, four options each. The tree stops starting work at either, and asks you to raise it or stop.
- **The guard stops more than a recursive `rm`.** It now refuses the shell commands that could be a major disaster, such as a `DROP TABLE` through a database client, a disk wipe or a `terraform destroy`. No Claude Code sandbox is needed, and `rex init` turns none on.

### Added

- **The budget question.** Each time a master tree opens, new or resumed from the store, the mod asks `How much may this master tree spend?` with the options `Low ($10)`, `Medium ($250)`, `High ($5,000)` and `Extra high ($100,000)`. Under Other you can type another amount. The budget covers the master's own requests and every job of the tree. Before, a tree took `tree_usd` of the policy and asked nothing.
- **The time question.** The second dialog asks `How long may this master tree run?` with the options `Short term (1 hour)`, `Medium term (8 hours)`, `Long term (3 days)` and `Unlimited`, or a span you type, such as `90m`, `36h` or `2d`. The time counts from your answer. Before, a tree had no time limit.
- **A tree at its budget or at the end of its time starts nothing new.** `rex_build` opens no job and no job of the tree starts a new agent. The mod asks you once, `Raise it` or `Stop the tree` for the budget, `Give it more time` or `Stop the tree` for the time, and the watchdog wakes no builder while the question waits. `Raise it` takes the next of the four budgets; `Give it more time` gives the same span again. `Stop the tree` tells each builder to close its job `blocked` and the master to end the tree with its report; a job whose builder does not close it is closed by the mod after 2 watchdog ticks. A headless tree, with nobody to ask, stops.
- **`/rex budget [amount]` and `/rex time [span]`.** They show the budget, the spend, the time and the end of this session's tree, or set one of them from your own typing: `low`, `medium`, `high`, `xhigh` or an amount, and `short`, `medium`, `long`, `unlimited` or a span. A tree that stood at the limit goes on: its question is closed as `Not needed`, and its master and its stopped builders are woken.
- **The commands are shown at the limit.** When a tree reaches its budget or its time, the mod writes into the transcript each command that changes it: the two answers of its question, `/rex budget`, `/rex time`, `/rex limit` and `/rex policy show`.
- **The tool `rex_raise`: the master asks, you confirm.** Tell the master to raise the budget, the time or a job's limit, or let it ask when the work needs more. The mod shows you the request with the master's reason, and the options `Raise it to $<amount>` (or `Give it <span>`) and `Keep the budget`, `Keep the time` or `Keep the limit`. Only your own answer sets the value; a master cannot raise a limit itself, and a headless run cannot ask.
- **The master reads its limits.** The answer of `rex_tree_open` has the lines `budget_usd=`, `time=` and `ends_at=` and a `<limits>` block, and `rex_status` has `tree_time=` and `tree_ends_at=`. The master skill plans the steps to fit both, and has a section for a tree at its limit.
- **The guard stops the commands that could be a major disaster.** A Bash or Monitor call is refused with `rex guard: ALERT`, and you get a status line `rex: ALERT: stopped ...`, when it would:
  - send `DROP DATABASE`, `DROP SCHEMA`, `DROP TABLE`, `DROP KEYSPACE`, `TRUNCATE` or a `DELETE FROM` with no `WHERE` to a database client (`psql`, `mysql`, `sqlite3` and the like), in an argument, a here-document, a here-string or a pipe, or run `dropdb`, `dropuser`, `mysqladmin drop`, `redis-cli FLUSHALL` or a MongoDB drop;
  - delete with `find -delete` or `find -exec rm` from the root, the home folder, the main checkout (with no test that picks files) or outside the folders agents may clear, or run `git clean -x` or `-X` with force in the main checkout;
  - write over a disk: `dd of=/dev/...`, a redirect to a disk device, `mkfs`, `wipefs`, `diskutil eraseDisk` and others;
  - change the mode or owner of the root or the home folder recursively;
  - stop the machine, kill every process, delete the crontab, or start a fork bomb;
  - delete cloud resources: `terraform destroy`, `kubectl delete namespace`, `aws s3 rb`, `gcloud projects delete`, `gh repo delete` and the other rows of `ROWS` in `plugin/hooks/guard/disaster.ts`;
  - force-push or delete `main` or `master`.

  The same words where no such program runs are left alone: `grep -rn "DROP TABLE" migrations/`, a commit message, a file written with `cat > m.sql`. `sqlite3` and `duckdb` on a throwaway database (in memory, in a temp folder or in a job worktree) still run. If you mean the command, run it in your own shell. Before, the guard stopped only a recursive `rm`.

### Changed

- **`tree_usd` of the policy is the budget of a headless tree only.** With you at the terminal, your answer to the budget question sets the budget. `tree_usd` no longer shows in the `<policy>` block or in the `policy.` lines of `rex_status`.
- **`/rex policy confirm` keeps a tree's budget.** Before, it set the tree's cap to `tree_usd`.
- **Resume asks for the budget and the time again.** What the jobs spent before counts against the new budget. Before, a resumed tree took `tree_usd`.
- **The refusal of `rex_build` at the budget** now reads `rex gate: master tree <id> has spent $<spent> of its budget of $<budget>; nothing new starts until the person raises it (they answer the mod's question, or type /rex budget <usd>) or stops the tree; end your turn, and the mod wakes you`. Before, it told you to raise `tree_usd` and confirm the policy.
- **A wake that cannot reach a builder starts no new builder while the tree stands at its budget or its time.**
- **The guard's failure text.** A check of the guard that fails still refuses the command. The text is now `rex guard: the check of <a recursive rm | a database delete | a disaster command | a command> failed (<error>); refused`.
- **The shell text of each brief** and the builder skill say what the guard stops.
- **The notices of `rex init`** about a sandbox that an older run turned on now end `the plugin's guard replaces it`.

### Known limits

- The budget and the time stop new jobs and new agents only. Agents that already run finish their turn, so a tree can end a little over its budget.
- The end of the time is checked at each `rex_build`, at each new agent and at each watchdog tick, so you can be asked up to one watchdog period (5 minutes by default) after it.
- The store keeps the budget a tree opened with and no time. `rex tree show` after the session prints that first budget, and a resume asks you for both again.
- The guard is a tripwire that reads the command text, not a sandbox. It does not see SQL in a file (`psql -f`, `mysql < file`), tables dropped by an ORM, a migration tool or a test runner, commands inside a script file, `find ... | xargs rm`, `rsync --delete`, or a program named by a variable.

## 0.11.1 - 2026-11-18

Install the binary and the plugin of this version together, then start a new session. After Claude Code itself was closed, a tree comes back with `/super-rex:orchestration-master resume`: the mod's record of a session does not outlive its process.

What changes for you:

- **Spend limits no longer stop your work.** Every job gets a default limit of $100,000, and a master tree a default cap of $100,000. Before, a job got $10.50 to $96 by its size and budget, and a tree $150. A limit now stops a job only where you, the policy or the master set a lower one.
- **A master tree never waits in silence.** Raising a limit wakes the builder that stood at it. Each answer to a question of the mod now does something. A builder that is gone gets a new builder or one question. The master can give a job up.

**What to redo.** A policy that an older release stored holds its own `tree_usd` (150). In each repo with a stored policy, run `/rex policy show`, and when it says `tree_usd=150`, `/rex policy set tree_usd 100000`, then `/rex policy confirm`.

### Added

- **The tool `rex_cancel`.** The master closes one open job of its tree `cancelled`, with a reason. The job's builder is told to stop, and the worktree and branch stay. The master uses it when you stop the tree, and for a job that cannot go on and whose step is built again or left out. Before, only a builder could close a job, so a job whose builder was gone kept its tree open for ever.
- **`/super-rex:orchestration-master resume` in the session that still holds the tree.** It answers `existing=yes` and `resumed=yes`, and each open job whose builder no longer runs, such as one you stopped, gets a new builder or a question, by `on_dead_builder`. Before, it answered `existing=yes` and restarted nothing.
- **The answer `Not needed`.** The mod closes a question that nothing waits on any more: a question of a job that closed, and a spend question of a job that is under its limit again. Such a question no longer shows as waiting, no longer holds the watchdog off its job, and no longer keeps `rex_report` from closing the tree.

### Changed

- **The default spend limit of a job is $100,000**, the same for every size and budget, in the binary (`rex limit`) and in the mod. The default `tree_usd` of the policy is 100000.
- **A limit you set wakes the builder.** `/rex limit <job> <usd>` wakes the builder of a tree job that stood at its limit at once, and `rex limit <job> --set <usd>` at a shell does at the next watchdog tick. The spend question that waited is closed.
- **`Raise` and `Keep the limit` act at once**, from the dialog as from `/rex answer`. `Keep the limit`, `raise_limits: never`, a second raise under `allowed`, and a tree cap with no room left now tell the builder to close the job `blocked`, with one status line for you. Before, the job stayed open and nobody was told.
- **`Build it again`, `Start a new builder` and `Leave it blocked` act.** The first two start a new builder in the job's worktree, which goes on from the job's record. `Leave it blocked` closes the job `blocked`. A job the watchdog gave up on now stays open until you answer, with the line `rex: job <id> waits on you: builder <agent> did not recover`. Before, the job was closed `blocked` first and nothing acted on the answers.
- **A builder started after a dialog answer is linked to its job.** A spawn made from a timer does not pass the plugin's own spawn hooks, so the mod now records the run and the link itself. Found in a live run of this release.
- **`/rex policy confirm` wakes each builder that a paused tree had stopped.**
- **The watchdog counts wakes in a row.** A job that spends again starts its count of stuck wakes from nothing, and a turn that ends well sets the count of API-error tries back. Before, both counted over the whole life of a job, so a long job was blocked by its third quiet spell.
- **A wake that cannot reach a builder** starts a new builder (`on_dead_builder: continue`) or parks one question (`ask`). Before, it showed a status line only.
- **A question the master asks you in the conversation holds the watchdog off idle builders**, so the builder that waits for your answer is not told to go on without it.
- **`rex_continue` refuses a closed job**, and for an open job with no builder it runs the step of `on_dead_builder` and says what happened. Before, it told the master to call `rex_build`, which opened a second job.
- **`/rex policy confirm` gives a running tree the new `tree_usd` as its cap.** Before, the cap stayed as it was when the tree opened, though the refusal told you to raise it this way.
- **Resume** brings each open job back with what it has spent, takes the tree's cap from the policy you confirmed, and shows each question that waited again. Before, every job came back with a spend of $0, and a question that waited was shown nowhere.
- **Stopping a tree.** The master skill now closes each open job with `rex_cancel` and starts no fix job for a tree you stopped.

### Removed

- The table of default limits by size and budget, in `rex limit`, in `fixtures/shared/tables/limits.json` and in stage 3 of the builder skill. The agent caps of stage 3 stay.

### Known limits

- `Build it again` starts a second builder in the worktree of the first. When the first is only slow and not dead, both may write there.
- A resumed tree counts its master's own spend from nothing, because the store does not hold it.

## 0.11.0 - 2026-11-17

Install the binary and the plugin of this version together, run `rex init` again in each repo, then start a new session.

What changes for you:

- **The master is your own session.** `/rex master` is gone. Type `/super-rex:orchestration-master <goal>` and the session you are in becomes the master: it cuts the goal into jobs, starts a builder for each, and ends its turn. You talk to it as in any Claude Code session: ask where things stand, change the goal, or tell it to stop the tree. In 0.9 and 0.10 the master was a background agent that you could reach only through `/rex answer` and `/rex policy`.
- **`rex init` turns no sandbox on.** Agents run every Bash command with no prompt and with your full access. The deny rules still hold, and the mod stops a recursive `rm` that would delete the root, your home folder, the repo itself, or anything outside the folder that holds the repo.

**What to redo.** SuperRex keeps no backward compatibility yet:

- Run `rex init` again in each repo. It turns off the sandbox that an older run turned on in `.claude/settings.local.json` (`sandbox.enabled`, `permissions.blockWritesOutsideWorkingDirectories`), with a notice for each, and adds `"Bash"` to `permissions.allow`. Then start a new session: a running session keeps its old settings.
- A stored policy changed shape, because `master_model` is gone. The first `/super-rex:orchestration-master` in a repo with a stored policy says the policy changed: run `/rex policy confirm`.
- A tree that a master of 0.10 left open comes back with `/super-rex:orchestration-master resume`, with your session as its master.
- A script that ran `rex init --strict` fails: the flag is gone.

### Added

- **The skill `orchestration-master`.** `/super-rex:orchestration-master <goal>`, `/super-rex:orchestration-master @<charter file>` and `/super-rex:orchestration-master resume`. The skill tells the main session how to run a tree: open it, cut the goal into steps, write each builder's request, read each wake, ask you what the charter and the policy leave open, check each delivered job against its done check, and write the final report.
- **The tool `rex_tree_open`.** The main session opens its tree with it. It takes no argument: the charter is the text of your own prompt, so no agent, file or tool result can write one. Its answer is the charter, a marked reading of it, the policy, the repo's places and the code map. A master that lost its context calls it again and reads them again.
- **The rm guard.** In a repo that `rex init` set up, the mod reads each Bash and Monitor command of the main session and of every agent, in every permission mode. It refuses a recursive `rm` of the root folder, your home folder or a folder that holds it, the main checkout, its `.git`, a folder that holds the repo, and anything outside the folder that holds the repo, `<repo>.worktrees` and the temp folders. The refusal starts with `rex guard: ALERT`, and you get a status line. A variable that the command never set counts as empty, as the shell takes it, so `rm -rf "$DIR"/` is read as the root. It sees through `sh -c`, `eval`, `sudo`, `env`, subshells and a `cd` in the same command. It reads the command as text and follows no symbolic link: it is a tripwire for a wrong path, not a sandbox, and it does not see `find -delete`, `git clean` or what a script file does.

### Changed

- **The mod wakes the master with a prompt.** A wake enters the main session as a prompt of the plugin that starts with `rex: `. The model reads it as the plugin's message, never as your words, and no merge answer, limit, job or tree comes from it. The engine's own notice that a builder ended a turn still becomes one status line.
- **Questions.** With you at the terminal the master asks in the conversation. Headless, it parks a question with `rex_ask_owner`. The mod's own questions (a merge under `merge: ask`, a blocked job, a spend raise) still open a dialog, and `/rex answer` and the pane still answer them.
- **A tree ends only with `rex_report`.** The mod no longer wakes a master that stopped with nothing open, and no longer closes its tree after two such wakes: your session stops whenever it has nothing to do, and you are there to tell it to go on.
- **The master's own steps count toward the tree.** The main session's model requests go to the ledger of the tree it runs, as the master agent's did.
- **The headless permission check runs when a builder starts.** `rex_build` answers the refusal and cancels the job, where `/rex master` refused before the tree opened.
- **`rex init` writes permissions only.** `permissions.allow` gets `"Bash"`, `permissions.deny` keeps its ten rules, and `permissions.additionalDirectories` still opens the folder above the repo and the job worktrees to the file tools. No kept value of yours fails the run any more: a value that differs is a warning.
- **Each brief carries the shell text in place of the sandbox text.** It says that no sandbox holds a command and what the guard's alert means.
- `rex doctor` names `/super-rex:orchestration-master resume` for an open job of an older session.

### Removed

- `/rex master`, the agent type `super-rex:master`, and the policy key `master_model`. The master runs on the model of your session.
- `rex init --strict`, every `sandbox` setting that `rex init` wrote, `permissions.blockWritesOutsideWorkingDirectories`, the allow rules for `rex worktree`, and the check for `bwrap` and `socat` on Linux.
- The removal of the stopgap entries `git *` and `Bash(git *)` from the settings file: they are harmless now, and `rex init` leaves them.

A live interactive session on 17 November 2026 showed, with this release's plugin and binary in a repo with no remote:

- the rm guard refusing `rm -rf ~/<a folder>` in bypass mode, with its alert and its status line;
- a master tree of two jobs run by the main session: the one policy question, two builders in parallel, a status answer in plain words while they ran, the mod's wakes as prompts of the plugin, a builder's question asked in the conversation, both jobs merged with `rex_merge`, and the final report.

The README lists what is proven only in the tests.

## 0.10.0 - 2026-11-16

Install the binary and the plugin of this version together, then start a new session.

### Fixed

- **An agent's screen in `rex watch` keeps its facts once the run is read from the store.** With storage on, a run of a closed job showed `planned -`, `brief check -`, no skills and no report, and lost its task, so `engineer T1` became `engineer`. The screen now shows them from the store, as it does from the live view.
- **A run that starts while `rex watch` is open keeps its plan in the store.** A refresh of `rex watch` could record the run before the plugin's start event arrived, and the event then left the run's planned agent, brief check and given skills empty. The start event now fills them. A run already stored without them stays as it is.

## 0.9.0 - 2026-11-16

What works now that did not in 0.8:

- **A master tree runs to its end.** `/rex master` asks one policy question in plain words, starts the master and its builders itself, wakes the master when a builder delivers or stops, and the master ends the tree with a final report. In 0.8 a tree could close with nothing built, or stop after the master's first turn. A master also runs with storage off.
- **A repo with no remote, or with no commit, needs no git set-up.** The mod makes the first commit, a job starts from your local branch, and an agreed merge moves that branch, so the files appear in your main checkout. No question about branches, remotes or merging comes up beyond the one merge question.
- **The views walk a run level by level.** `rex watch`, the pane (`/rex watch`), the band and the new `/rex tree` name each level the same way (master, builder, lead, worker, helper) and say how deep you are.
- **`rex watch` needs no store.** It reads the live view that each session writes, in every storage mode.

Live interactive sessions showed these on 15 and 16 November 2026:

- a builder and a master in a repo with no commit and no remote;
- a master tree at every level, down to helpers at depth 4, merged to a remote through `rex_merge`, with the mod's clean-up, the done check and the cost of record;
- a master with storage off whose merge question you answer with `/rex answer 1`.

The README lists what is proven only in the tests. Version 0.8.3 was never published; its changes are in this release.

**What to redo.** SuperRex keeps no backward compatibility yet:

- Install the binary and the plugin of this version together, then start a new session. A session of 0.8 writes no live view, and `rex watch` reads only the views of its own version.
- A policy that a master of 0.8 stored keeps its old watchdog values (`watchdog_minutes` 30, `stuck_ticks` 3). For the new defaults, run `/rex policy set watchdog_minutes 5`, `/rex policy set stuck_ticks 18` and `/rex policy confirm`.
- A policy with `merge: allowed_if_checks_pass` and no `merge_checks` now merges on the builder's own checks, where 0.8 refused every merge. Set `merge_checks`, or another `merge`, if you relied on that refusal.
- A store made by 0.8 is still version 2 and needs nothing.

### Added

- `rex_report`, a new tool of the master: it ends the tree with the master's final report, written to `.rex/plans/<tree id>/final-report.md`, and is refused while a job of the tree is open or a question of it waits.
- Mode `none` for a repo with no remote `origin`, or one that holds no base yet. The base is the branch you have checked out (with HEAD detached, `main`, else `master`; a repo whose only branch is `master` gets `master`), a job starts from the local base, and `rex_merge` fast-forwards your local base branch, so the files appear in your main checkout and the delivery is `merged`. It moves a base that no worktree has checked out only while that base is still an ancestor of the job branch, and when your main checkout holds files in the way it refuses and names them. The builder tells you in one line about uncommitted changes it does not hold, and never asks about git set-up.
- The first commit of a repo with none: the mod makes it when a job opens (`chore: first commit, the base of rex jobs`, with your commit hooks). When a hook or git refuses it, the mod puts back what you had staged, opens no job, and names the failure and the two commands to run.
- `rex init` asks whether to keep a record of jobs in `.rex/rex.db` when you run it at a terminal with neither storage flag, and an empty answer is yes. `--without-storage` answers no without asking. A run with no terminal asks nothing, keeps storage off, and says what needs the store. A store that is already on stays on.
- The live view: the mod writes its view of the session to `.rex/live/<session id>.json` in every storage mode, after each change and at least every 10 seconds while a job or a tree is open. It holds no prompt, brief or ingest secret.
- `/rex tree`: the outline of the session, each master tree, job and agent by level, with tree guides, one line each cut to the terminal.
- `rex watch --once`, which prints the whole view once as a plain-text tree and needs no terminal.
- With no job of its own, `/rex status` and `rex_status` in the main session show each live tree: what it runs, in which folder, and what waits on you, in place of `job=none`.
- Each parked question shows at once as a status line with the command that answers it, and as a dialog in an interactive session. A dialog you dismiss leaves the plain line; once a question is answered, the line names the oldest question that still waits, or is cleared.
- The merge tripwire also refuses a `git merge` of the job branch or of its `--work` branch, a `git branch` that forces, renames, copies or deletes the base, `git update-ref refs/heads/<base>`, and a push or fetch from `.` into the base.

### Changed

- A master runs with storage off: its tree lives in the book of the session. Only `/rex master resume` needs storage.
- The first `/rex master` in a repo asks one question in your words, `Merge each job of a master to its base branch when its checks pass?`, with `Yes, merge`, `No, keep the branches` and `Ask me for each job`, in place of two questions in the policy's own terms.
- A master turn that ends never closes the tree. A master that stops with no job open and no question waiting is woken to report or go on, and after 2 such wakes in a row the mod closes the tree as `done` with the master's last message.
- The mod starts the master and every builder of a tree itself, and denies every other spawn of the master type, so the master can be woken after its first turn. A build refused at the rate limit wakes the master `retry.then_minutes` later, and `/rex policy confirm` wakes each master it resumes. The master's first prompt carries the policy, the repo and its mode, and the code map, and the master can read the code with `Glob` and `Grep`. It never asks you about git set-up or about what the policy settles.
- A master hears of a builder only when the builder ended its turn with no agent of its own running, or closed its job, with the end of the builder's last message. A builder that waits for its own agents wakes nobody.
- The watchdog ticks every 5 minutes (`watchdog_minutes` 5, `stuck_ticks` 18). A job with a run open, the builder's or an agent's below it, whose spend stays flat for 90 minutes is stuck, so an agent whose end never reached the mod no longer holds the tree for ever; a job whose agents spend is never woken. The builder of an idle job, with no agent running, is woken after 10 minutes with no spend, and the job reaches its blocked question in about 30 minutes. No job is woken while its question waits for you. A stopped master hears only of the first wake of a stopped builder. A quiet tick writes nothing.
- The answer to a merge question, given in the dialog, in the pane or with `/rex answer`, wakes the job's builder as well as the master.
- `/rex answer` takes the option's number or its text in any letter case, and needs no id when one question waits. `/rex master` answers how to watch the tree and how to answer its questions.
- The main session no longer answers the turn notices of a master or a tree builder: each reaches you as one line, and only the closing notice of a tree reaches the main session.
- The mod makes and removes every worktree of the job repo, so no agent runs a worktree command for it and the sandbox no longer refuses one. A job's worktree is made when the job opens, and a job whose worktree cannot be made, also when `rex worktree add` times out, is cancelled; a lead's parallel writers get theirs from the new tool `rex_worktree`. A fetch that fails starts the job from the `origin/<base>` the repo last fetched, and says the base may be stale. `rex_job_close` removes each worktree with no uncommitted work, and each branch of the job, here and on `origin`, whose work is on the base, and the builder tells you each one it kept.
- `rex watch` reads the live views and the store, when there is one, and shows them as one view. It walks a run level by level from home, with the path and the rail `level N of M` on top, and shows each waiting question on its tree and its job with numbered options and the `/rex answer` command. It fits any terminal and names an agent by its role, else its task, else its grid agent, saying a role once when it is the level's word (`worker T1`, not `worker worker`). A job whose session ended before the store had its cost of record shows the live tally, marked `≈`. It removes live views older than a week when it starts, and leaves every other file in `.rex/live/` alone.
- The pane walks the run level by level, with a breadcrumb, a level rail, the children of the node as rows (digits go down, `u` goes up, `n` pages past nine rows) and then its facts. Rows name agents as `rex watch` does. Its tabs are `tree`, `decisions` and `questions`; the Jobs, Tree and Master tabs are gone. A digit in the questions tab answers the oldest question.
- The band shows a master tree, each open job and what its builder is doing, and the oldest waiting question in the warning color with the command that answers it. Levels are named in place of `depth N`.
- Inside a job, an Agent call that gives `name`, `team_name`, `model`, `effort`, `mode`, `isolation` or `cwd` is refused before it starts, with the argument and what to do: a `name` had started a teammate that the mod did not record, gate or brief. A teammate or a workflow's agent that starts inside a job is refused at its spawn.
- `rex worktree add … --branch <name>` checks out a branch that exists already when the commit is its tip, and refuses one at another commit.

### Fixed

- The cost of record reaches the store in every session with storage on: the mod sends the session's transcript path once it knows it, and when the store holds none, `rex` finds the transcripts where Claude Code keeps them. A master run had lost the path.
- `/rex master resume` gives a restarted builder its worktree back on the job's branch, at its tip, when the worktree is gone. It failed because it tried to make the branch again. A git call that fails while a builder restarts leaves that job for you, with a status line, and the other jobs and the watchdog still start.
- Two agents of one role started at the same moment write two report files (`worker.md`, `worker-2.md`), not one.
- An agent's role is the role of the task its brief names, so the missed and extra skills of `rex_job_close` count a worker that shares its lead's grid agent as a worker, not as the lead.
- `rex_skill` reads a file named as `<skill>/<file>.md`, the way skill text names it, and refuses a file the skill does not hold with the list of the files it holds.
- `rex_job_open` and `rex_build` in a folder that is no git repo refuse with `git init` to run.
- Every git call of the mod runs with `GIT_TERMINAL_PROMPT=0` and a timeout, so a remote that asks for a password fails at once instead of hanging.
- `rex recall` failed with `Invalid column type Null` for a job stored without a work type, size or consequence.

## 0.8.2 - 2026-11-14

### Fixed

- The agents that start before a plan, the decision agent and the challenger, get the stage files of their fixed manifest in their brief, such as `orchestration-builder/stages/1-skills/stage.md`. Before, an entry that named a file and not only a skill attached nothing, and the agents had to read the stage files with `rex_skill`.
- The clean-up of `rex-deliver` deletes the job's remote branches. It ran `git push origin --delete "$B"` in a loop, and the mod refused that push because the shell fills in its target. The loop now prints each branch, and each push names its branch.
- A job's spend limit also counts the main session's own steps when the main session is the builder, not only its sub-agents.
- `rex_merge` refuses a job that has no plan, so a builder that skips the decision and `rex_job_plan` cannot merge.

### Changed

- `/rex limit` with no job id now says that it needs the id of a job, instead of naming a job that does not exist.
- The release check script's help says to commit renewed Claude Code declarations only when the minimum supported version rises.
- Inside the mod, one lookup finds the session's open jobs, where nine hand-written copies of the same filter stood, and the test-output trim splits its test commands once instead of on every shell command.

## 0.8.1 - 2026-11-14

### Added

- The master. `/rex master <goal or charter file>` starts a master that runs a tree of jobs, each with its own builder in its own worktree. It runs only from what you type yourself, after a pre-flight check, and needs `rex init --with-storage`. The charter is stored unchanged and goes in every brief.
- `rex_build`, `rex_continue`, `rex_note` and `rex_ask_owner`, the master's tools. `rex_build` opens a job under the policy and makes its worktree with `rex worktree add`.
- `/rex policy` and `/rex answer`: the policy file, pinned by its hash, and your answers to the questions a master parks. A changed policy pauses the tree until you confirm it.
- A watchdog that wakes a stuck builder and retries API errors on a schedule, a policy that can raise a job's spend limit once, the done check of each job, a final report that lists each job, and the `master` view in the pane.
- `/rex master resume` puts a tree back from the store after a restart and continues or asks, as the policy says.
- The gate checks `tree-ledger`, `rate-limit-pause`, `max-jobs` and `policy-merge`; each refuses when it fails.

## 0.8.0 - 2026-11-14

SuperRex is now a Claude Code plugin with a mod inside it. The skills, the skill library and the grid agents ship in the plugin, and the mod does the live work that the `rex` hooks did: it records each agent, checks and builds each brief, refuses a new agent over the spend limit, cuts test output, and merges. To move a repo from 0.7, read [docs/upgrade-0.8.md](docs/upgrade-0.8.md).

### Added

- The `super-rex` plugin, with its marketplace in `.claude-plugin/marketplace.json`. It holds the 6 skills that you start, now named `/super-rex:<skill>` (such as `/super-rex:orchestration-builder`), the library of 24 skills (with `rex-gap-ts`, a gap skill for TypeScript) and the 20 grid agents, now named `super-rex:model-<tier>-effort-<level>`. `rex init` installs it at project scope through `claude plugin marketplace add` and `claude plugin install`. `--plugin-source <path or owner/repo>` names another marketplace, such as `.` for a working copy.
- The mod, below `plugin/hooks/`. It keeps the live record of each job (the book), and marks a job `degraded` when a part of it fails, never blocking a prompt, a tool call or a new agent. It reads your prompts: `/rex prompts <off|observe|improve>` sets whether it records them and whether it adds a restatement of a request that starts a job as context, written by a Sonnet completion. Your text is never changed. In every mode it reads a merge answer (`merge: yes`) and a spend limit (`Set the spend limit of this job to <usd> dollars`) from your words.
- The mod checks each brief of a job once, for its `Task:` line and its five sections `Outcome:`, `Scope:`, `Context:`, `Acceptance:` and `Limits:`, and sends it back once with the findings. Then it builds the brief that the agent gets: the tag lines, a `<skills>` block with the full text and hash of each read-first skill and a line for each open-when skill, the author's text unchanged, the text for the agent's model and effort from `rex-agent-prompting`, the sandbox text and the `Return:` line.
- Tools for the skills, each named `mcp__super-rex__rex_<name>`: `rex_skill`, `rex_status`, `rex_job_open`, `rex_recall`, `rex_job_plan`, `rex_job_close`, `rex_merge` and `rex_assign`. The mod reads the caller from Claude Code, never from an argument, and a builder can act only on its own job.
- The assignment table is data, `plugin/assign/table.json`, and a repo can change it with an overlay, `.rex/assign.json`. `rex_assign` applies the rules that code can decide, asks the decision agent each criterion that needs judgement, and places at most 6 read-first skills of each task by priority, the rest as open-when skills with their case. A skill that a rule requires stays whatever the answers say. Each answer and each placed skill is recorded with its source, its reason and the hashes of the table and the overlay, and `rex_status` and the Decisions view of the pane show them. Stage 1 and the challenger work from these recorded answers.
- `rex_skill` gives a skill file by name, with its source and hash, from the repo's `.rex/skills/`, then a job's own skills, then the plugin, then `~/.claude/skills/`. Skill text names a file as `<skill>/<file>.md`, never by a folder, because the plugin's folder changes with each version.
- `rex_merge`, the only way a job merges. It reads the repo, the branch and the base from the caller's job, takes a merge lock for the repo, checks that the branch is one squash commit over `origin/<base>` and that the job has a merge answer from you (or from the master's policy), and pushes it to the base as a fast forward.
- The `/rex` command, which runs at once and costs no model call: `watch`, `status`, `jobs`, `doctor`, `prompts` and `limit`. A sub-command that changes something runs only from your own typing.
- A band line above the prompt while a job runs, with its status, agents, spend and limit, depth, questions and permission prompts, and a pane (`/rex watch`) with a header and the tabs Jobs, Tree, Decisions and Questions. A question has one Answer button for each option.
- Storage modes. Without `--with-storage`, the record of a job is the book, in memory, and ends with the session. `rex init --with-storage` writes `storage: on` in `.rex/config.json`, and the mod then sends each change to the store. Recall, feedback, the cost of record and master runs need it.
- `.rex/config.json`, which `rex init` writes and the mod reads: `storage`, `depth`, `jobs_from_sdk` and `min_claude_code`. It is in Git.
- `rex init --depth <levels>` and the setting `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`, 4 by default, which a master run needs. A higher value of yours is kept, and a lower one is kept with a warning. The mod lets an agent start at depth 4 only below a builder that a master started.
- Deny rules that keep merges and the record out of an agent's hands: `Bash(gh pr merge:*)`, `Bash(git push origin main:*)`, `Bash(git push origin HEAD:main:*)`, `Bash(rex ingest:*)`, `Bash(rex migrate:*)` and `Bash(rex limit:*)`.
- `rex migrate`, which moves a 0.7 store (version 1) to version 2 in one transaction and keeps each job. It is safe to run twice. Every other command refuses a version 1 store with `store is version 1; run rex migrate`.
- `rex ingest`, the mod's write path: one batch of events on stdin, a secret for each session, the same batch twice taken as one, and `last_sequence=<n>` as its answer. Agents may not run it.
- `rex doctor`, which reports whether the binary, the store, the config, the plugin and Claude Code fit, with one `problem=` line for each thing to fix.
- `rex tree show [<tree>] [--json]`, which prints a master tree with its jobs and its questions.
- `rex limit <job> --json`, and the limit source `policy`.
- `rex worktree add` and `rex worktree remove`, which make and remove a job's worktree in `<repo>.worktrees/` and list it in `permissions.additionalDirectories`, so that `git commit` works there inside the sandbox with its hooks still sandboxed. `rex init` excludes them from the sandbox and allows them with no prompt (not with `--strict`), lists the worktrees it finds and takes out those that are gone.
- The shared fixtures in `fixtures/shared/`: each call that the mod makes to `rex` with its expected output, and the tables that both sides use (the 44 `rex oms` cases, the limits, the prices, the hash vectors and the tools). A Rust test and a test of the mod check each one.

### Changed

- `rex init` installs no files of skills or agents. It writes the settings, `.rex/config.json`, the plugin, `AGENTS.md`, `.gitignore`, the pre-commit config, and with `--with-storage` the store. It needs Claude Code 2.1.323 or later. One run finishes the upgrade of a 0.7 repo. With `--with-storage`, it moves a 0.7 store (version 1) to version 2 itself, as `rex migrate` does, after a copy to `.rex/rex.db.v1-backup`; a store in which a row names a row that is not there is left as it is, and the failure gives the command that lists those rows. It removes the hook entries that rex 0.7 wrote for `rex hook`, `rex gate` and `rex test-hook`, which would fail on every event now. It removes the stopgap entries `git *`, `git:*`, `Bash(git *)`, `Bash(git:*)` and `Bash(cd <parent>/*)`, each with a notice. It removes each unchanged copy that rex 0.4 to 0.7 put in `.rex/skills/`, `.claude/skills/` or `.claude/agents/`, and replaces an unchanged 0.7 block in `.gitignore`, one line for each. A copy or a block that you changed is kept, with a warning that says what to change, and the run does not fail.
- The `.gitignore` block now tracks `.rex/config.json` and `.rex/skills/`, the skills of the repo itself, and ignores only the store, the plans, the worktrees, the orchestrators and the live files in `.rex/`.
- The builder is `/super-rex:orchestration-builder`. It stops at step 1 when the mod is not loaded, opens, plans and closes its job with the mod's tools, and asks the merge question in one fixed form, `Merge this job to main when its checks pass?`, with the options `Yes, merge` and `No, keep the branch`. A job's branch is `rex/<job ID>`.
- The spend gate is in the mod. It refuses a new agent at the Agent call and again at the spawn, and the refusal now names `/rex limit <job> <usd>`. Before it refuses, it reads a limit that you set with `rex limit <job> --set`.
- Test output is cut in the mod, from the result of the Bash tool, and a failed run reaches the agent whole. `rex test-filter` stays for a test run at a shell.
- `rex-deliver` makes one squash commit over `origin/<base>` and merges only through `rex_merge`, and only in the job's own repo. With no merge, it opens a pull request or keeps the branch. Job worktrees live in `<repo>.worktrees/<job ID>`, beside the main checkout, and are made and removed with `rex worktree`. Commit and push run inside the sandbox with an absolute `-C` path.
- `rex-feedback` never edits a shipped skill in the plugin. On its first change, the skill is copied into `.rex/skills/<name>/`, which `rex_skill` reads first.
- The store is version 2. A job has an origin (`composer`, `bridge`, `sdk` or `master`) in place of `asked_by`, a builder, its places, the source of its merge answer, its limit, its degraded marks and its tree. New tables hold the decisions, the skill manifests, the skills each run opened, the events the mod sent, the master trees and the questions. The status `lost` is new.
- `rex job show --json` and `rex jobs --json` give the fields of version 2. `rex job show` prints `origin`, `builder`, `tree`, `merge`, `merge_source`, the places, `limit_usd`, `limit_source` and `degraded`.
- `rex skills check` rules B3 and B4 check skill files named as `<skill>/<file>.md` against `plugin/skills`, `plugin/library` and the repo's `.rex/skills`, and a path that names a skill file by its folder is a finding. B3 keeps each link inside its own skill. `rex skills check` takes a folder of skills.
- Text that names the library count says 24.

### Removed

- `rex job create`, `rex job plan` and `rex job close`. The builder opens, plans and closes a job with `rex_job_open`, `rex_job_plan` and `rex_job_close`. `rex job` keeps only `rex job show`.
- `rex hook`, and with it the event log `.rex/events.jsonl`. The mod records each agent and sends the events to `rex ingest`.
- `rex gate`. The spend gate is in the mod.
- `rex test-hook`. The mod cuts test output.
- `rex init --force` and `rex init --no-hooks`. `rex init` copies no skill files that `--force` could overwrite, and registers no hooks that `--no-hooks` could leave out.
- The skill files `rex-deliver/github-merge.md` and `rex-deliver/no-remote-or-no-gh.md`: the merge with `gh pr merge` and the local squash merge are gone. `rex_merge` is the only way to merge.

## 0.7.0 - 2026-11-12

### Added

- The library skill `rex-agent-prompting`. An agent that starts another agent now writes the brief for that agent's model and effort: rules for every brief in `SKILL.md`, and one file for each grid tier (`haiku.md`, `sonnet.md`, `opus.md`, `fable.md`) with the text to add at each effort level, such as the finish text for a low effort and the stop text for Sonnet at a high one. The builder, each lead, the parallel-writer steps, the plan review and `rex-rules` point to it, and the assignment table gives it to each role that starts agents. `rex job close` does not count it as an extra skill. `rex init` installs 5 more files.

### Changed

- `rex init` writes `sandbox.failIfUnavailable: false`, not `true`, so Claude Code still starts on a system where its sandbox cannot, and runs commands with no sandbox there. Ubuntu 24.04 with `kernel.apparmor_restrict_unprivileged_userns = 1` is such a system: `bwrap` is installed but fails, and Claude Code would not start in the repo. `rex init --strict` still writes `true`. A settings file that already holds `true` keeps it, with a warning: set it to `false` by hand. The message for a missing `bwrap` or `socat` on Linux now says that commands run with no sandbox until they are installed.

## 0.6.0 - 2026-11-11

### Fixed

- A teammate of a Claude Code agent team no longer shows as finished while it works. Claude Code stops a teammate each time it goes idle and starts it again, under the same agent ID, when a message wakes it. `rex watch`, `rex status` and `rex job show` now show the run open from each new start until its next stop, keep its first start as `started_at`, and give it no `stopped_at` while it is open. Its duration counts the time it waited idle between messages. The store has two new columns: delete `.rex/rex.db` and run `rex init` again.

## 0.5.0 - 2026-11-11

### Changed

- Library paths moved in a code tidy: `commands::smells::{run, SmellsOptions}` is now `commands::codebase::{run_smells, SmellsOptions}`, `gate::run` takes `GateOptions` and `test_output::run_hook` takes `TestHookOptions`. Output of every command is the same. `rex job create`, `rex status` and `rex recall` share one open-job rule, and an unfinished last log line no longer makes a read take the write lock.
- `rex oms` is version 2: a role gets the `xhigh` tier only when its consequence is `high`. With a lower consequence the tier stops at `high` and rule `T3` shows in `rules`. `rex job show` prints `oms_version=2` on each role. The builder now starts its decision agent, challenger, skill writer and pull request reviewer on a grid agent from `rex oms`, not on the session's model or a fixed agent, and a plan job gets one adversarial review only. The role form `"session": true` is gone from `rex job plan` and `rex job show`, and the store schema changed with it: delete `.rex/rex.db` and run `rex init` again.
- `rex job close` prints `extra_skills=<n>` and `extra_skill.<i>=<role> <skill> skill_md_words=<words>` lines after the missed-skill lines: the skills a run opened that its role was not given, with the length of each `SKILL.md`. The key `missed_skills_incomplete` is now `skills_incomplete` (and `"skills_incomplete"` in `--json`, which also gains `"extra_skills"`). Change any script that reads the old key. The work breakdown and the role table have an `Open when` column: `Skills` holds at most 6 skills that an agent reads first, and `Open when` holds the ones it opens only when its work reaches their case.
- Stage 3 caps the sub-agents of a job by its size: 5 for a small job, 6 for a medium one, 10 for a large one, shown as `AGENTS: <count> of max <max>` in the decision. The challenger reports a plan over the cap as a `MAJOR` finding. Stage 2 builds each code feature in one task, and splits code only where files and outcomes are independent.
- `rex recall` lines gain `cost_usd` and `cache_hit` on each lesson, and `cost_per_job_usd` and `cache_hit` on each orchestrator (`--json` too). A cost with unpriced requests prints as `≥…`. Orchestrator lines sort by `best_match`, `accepted`, `missed`, the lowest `cost_per_job_usd`, then `runs`. The decision agent prefers the cheaper of two orchestrators with equal results.
- `rex job close` also prints `long_messages=<n>` and a `long_message.<i>=<role> <agent ID> chars=<n>` line for each run whose final message has more than 2,000 characters (`"long_messages"` in `--json`). Sub-agents now write their full output to `.rex/plans/<job>/<role>.md` and return the path with a short summary. Parallel writers write `<role>--<unit>.md`.
- `rex status` prints a `warning:` line after the header of a created or planned job from 80% of its spend limit.
- `rex init` writes five hook entries, not three, and the allow rule `Bash(rex test-filter)`; it counts 149 files, not 148. The `--no-hooks` warning names `rex hook`, `rex gate` and `rex test-hook`. A repo that was set up before gets the new entries by running `rex init` again.
- Tokens show as four counts and the cache hit %: `in`, `cache-w`, `cache-r`, `out` and `hit`, in `rex watch`, `rex status` and `rex job show`, instead of one sum of the four. Cache reads are counted again on each call, so the sum was mostly cache reads. The job list of `rex watch` shows the cost and `hit`. `rex job show` prints `tokens_input`, `tokens_cache_write`, `tokens_cache_read`, `tokens_output` and `tokens_cache_hit` in place of `tokens`, and the same keys without `tokens_` on each run line.

### Added

- `rex test-hook` also takes over a test command piped into `tail` with at most a line count, such as `cargo test 2>&1 | tail -50`, and puts `rex test-filter` in place of `tail`. Models add that `tail` by themselves, and it can cut a failure off.
- `rex init` writes `sandbox.network.allowLocalBinding: true`, and, except with `--strict`, `sandbox.excludedCommands` for `git worktree`, `git merge`, `git pull`, `git switch` and `rex init`. A git command leaves the sandbox only when its paths are relative, run from inside the repo. The sandbox never lets a command write `.claude/skills`, `.claude/agents` or the settings files, so these commands could not check out a repo that tracks those folders. A file that sets `sandbox.allowUnsandboxedCommands: false` gets no excluded commands. Run `rex init` again to add them.
- A spend limit for each job, in US dollars at list prices. `rex gate`, a `PreToolUse` hook on `Agent|Task` that `rex init` registers, refuses a new sub-agent once the job's spend (sub-agents and main session) has reached its limit, and tells the agent to report the job `blocked` with the command that raises it. The default limit comes from the job's budget and size, the agent cap of the job's size plus 2 for the main session, times $1.50, $4 or $8 for a low, medium or high budget, from $10.50 (low budget, small job) to $96 (high budget, large job); no stated budget uses the medium row (small $28, medium $32, large $48). A refusal keeps the job open: the builder asks the requester to run `rex limit <job> --set <dollars>`, then goes on with the same job; the job closes `blocked` only if the requester declines. `rex limit <job>` prints `limit_usd=`, `spent_usd=` and `source=default|set`, and `rex limit <job> --set <dollars>` sets a new limit. Run `rex init` again in a repo that was set up before to get the hook.
- A cut of test output for agents. `rex test-hook`, a `PreToolUse` hook on `Bash` that `rex init` registers, runs a plain test command (`cargo test`, `pytest`, `go test` and others) through `rex test-filter`. `rex test-filter` first prints the path of a file that holds every line of the run. The changed command is `set -o pipefail; <test command> 2>&1 | rex test-filter`, a form that Claude Code's check of a Bash command lets through in the default permission mode, with the allow rules `Bash(rex test-filter)` and `Bash(set -o pipefail)`. The filter shows every line except the lines in a runner's exact passing form, and always keeps the summary. A command that holds a `cd`, `&&`, `#`, a backslash or an output flag such as `--nocapture` is left unchanged. The file has mode 0600 in the temporary folder, and later runs remove files older than 24 hours. The command keeps its exit status. The grid agents still load `CLAUDE.md` and `AGENTS.md`. Run `rex init` again in a repo that was set up before to get the hook and the allow rule.
- The cost in US dollars of every run, of the main session and of each job, at Anthropic's list prices for the model each request ran on: a `cost` column in `rex watch`, `$…` in `rex status`, `cost_usd` and `unpriced_requests` in `rex job show`, and `cost` in the JSON of `rex job show` and `rex jobs`. Claude Code writes no cost per sub-agent, so `rex` works it out from each request's usage, with 5-minute and 1-hour cache writes, fast mode and the Claude Haiku 5.5 long-prompt rates. The store has two new columns: delete `.rex/rex.db` and run `rex init` again.
- The runs panel of `rex watch` names the job's orchestrator skill and version, such as `pipeline v1`.
- The main session's tokens: a `session` row first in the run tree of `rex watch`, a `session` line in `rex job show`, and the job total in the `rex status` header and the job panel of `rex watch`, so the rows add up to the job.

## 0.4.0 - 2026-11-11

### Added

- `rex-rules`, a router skill. A session that writes or reviews code without the builder reads the four core rulebooks through it, then the rows of the assignment table that fit its work.

### Changed

- The 22 skills that are only read by path, never started by a person, now live in `.rex/skills/`, the skill library: the 19 `rex-code-*` rulebooks, `rex-skill-language`, `rex-mermaid-diagrams` and `rex-codex-adversarial-review`. Claude Code no longer lists them in every session. `.claude/skills/` keeps the 6 that a person or a session starts: `orchestration-builder`, `rex-deliver`, `rex-feedback`, `rex-pr-review`, `rex-grill-me` and `rex-rules`. The rulebooks' `/name` commands are gone.
- `rex init` installs the library into `.rex/skills/`. `rex skills list` shows its skills as `source=library`. `rex skills check` B4 knows both folders and names the right one for a path to the wrong folder. A read below `.rex/skills/<name>/` counts as opening `<name>` in the job record. A new skill that stage 1 writes goes into the library.
- `rex-deliver` keeps the store out of Git with `/.rex/*` in `.git/info/exclude`, tested on `.rex/rex.db`, so a repo can still track `.rex/skills/`.

### Fixed

- `rex init` ignores `/.rex/*`, not `/.rex/`, like `rex-deliver`, so a repo can track `.rex/skills/` with `!/.rex/skills/`. This repo has its own ignore block, so `rex init` here no longer hides new skill and agent files from Git.
- A worker in a job worktree is told that skill paths start at the main checkout, and the parallel-writer brief lets it read its skill files outside its worktree.
- `skills_opened` is `[]` when the run's transcript was read and it opened no skill, and `null` when no transcript was read. `rex-feedback` compares it with each role's `skills`.
- `rex skills list` in a linked worktree also lists the main checkout's library. `rex skills check` takes the bundled skills as known only for a folder the repo does not have. `rex job plan` refuses a skill name that is not a folder name, such as `rex-code-tdd/mocking.md`.
- The skills: full paths in the assignment table, one home each for the core rulebooks, the `SLUG` form and the sandbox brief text, gap skills named `rex-gap-<language>`, the plan writer gets the skill list, a pull request review on request runs in a fresh agent, and text of earlier designs is gone (the builder branch for a sub-agent, "stage sub-agents", the unused mermaid `README.md`).

### Removed

- All backward compatibility, because no repo uses SuperRex yet. The store's schema is version 1 again, with `role.skills` part of it, and has no upgrade path: a store made by 0.3.0 is refused, so delete its `.rex/rex.db` and run `rex init`. `rex init` no longer names old stage-skill folders. `rex codebase --rust-only` is gone; use `--only rust`.

## 0.3.0 - 2026-11-11

### Added

- One assignment table, `.claude/skills/orchestration-builder/stages/1-skills/assign.md`, says which skills each kind of work gets. The decision agent starts from it and gives a reason for each skill it adds or leaves out, and the challenger checks the decision against it.
- `rex job plan`: a role in the plan file may give `"skills"`, the skills its tasks are assigned. Each one must also be in the job's `skills`. `rex job show` prints each role's skills.
- `rex job close` lists each skill that a role was assigned and that no run of that role opened (`missed_skill.<i>=`, `role_not_run.<i>=`; `missed_skills`, `roles_not_run` and `missed_skills_incomplete` in `--json`). The status and the exit code do not change. The builder tells the person about each one.
- `rex codebase --with-context` and `--only` support Java, Kotlin, Scala, C#, Swift, Objective-C, Dart, PHP, Ruby, Lua, Haskell, Elixir, R, SQL, Zig and Perl. `.m` is Objective-C and `.pl` is Perl.

### Changed

- `orchestration-builder` and `rex-deliver` start only from a person: `/orchestration-builder <request>`, `/orchestration-builder Execute plan <path>`, `/rex-deliver`. No agent can start them, and their descriptions no longer take up room in every session.
- The four stage skills are now files inside the builder, in `.claude/skills/orchestration-builder/stages/`: 31 skills become 27. `rex init` names any of the four old folders that an earlier install left in `.claude/skills/`; delete them by hand.
- The 20 model and effort agents have no Skill tool (`disallowedTools: Skill`). A worker reads only the files its brief names, and the list of every skill no longer fills its context.
- The Rust rulebooks apply to every language: an agent applies the intent of each rule in that language's idiom. The module comment and priority block rules are no longer tied to `//!`, and agents find the priority block with `git grep -n 'Priorities, in order:'`.
- The store's schema is now version 2 (`role.skills`). The first `rex` run in a version 1 store upgrades it in place. After that, an older `rex` refuses the store and asks for a newer one.

## 0.2.1 - 2026-11-11

### Changed

- Eleven places where two skills told an agent different things now agree, each settled by the skill whose whole subject it is. An error that is only logged is not handled (`rex-code-failure-policy`). Doc comments are for the public API of a library and never restate the name (`rex-code-commenting`). A file's top comment names no other file, and the crate root may add the priority block. A `SMELL` flag stays until the commit that fixes its problem. A `TODO` is fixed, deleted, or made a `SMELL` flag. "Seam" has one meaning. Prototype first when the domain is unknown, test first when the plan gives the interface. No mock of your own code. A sub-agent that runs TDD asks its caller, never the person. Pre-release crates are tried in a spike outside the repo. Unused crates are removed, found with `cargo machete`.
- "Core rules", which the orchestration skills used without defining, now names the skills: `rex-code-philosophy` and `rex-code-commenting` for every language, `rex-code-ergonomics` and `rex-code-error-writing` for Rust, and one skill for each kind of work. Stage 1 puts them in the skill list of a job that writes code.
- `rex-code-external-libraries` has the rule again that a crate's types stay out of your public API unless that crate is part of your contract.

## 0.2.0 - 2026-11-11

### Added

- The skill `rex-code-latency`, a reference for latency in Rust code that runs in nanoseconds to tens of microseconds. Its `SKILL.md` is a router: measure first, then a table from what you see to the one or two files to read. It covers the operating system, CPU caches and branches, allocation and reclamation, atomics and queues, the network receive path, and how to measure without changing the result. Each topic has a rules file of at most 900 words and a code file that an agent opens only when it writes that code.
- A paired challenger checks the builder's decision before any work starts. The new skill `orchestration-decision-challenge` has a fresh grid agent (`model-high-effort-high`, or `model-xhigh-effort-high` for a large or high-consequence job) look for important skills that were left out or not given to a role, and for gaps in the breakdown, the orchestrator and the ratings. The builder corrects what holds before `rex job plan`, and the work breakdown records the check.
- Plan jobs. A request for a plan writes `.rex/plans/<job-id>/plan.md` and changes no file in the repo. A later run of `orchestration-builder` with "Execute plan <path>" does the work. `orchestration-selection` now says how to build an orchestrator for planning.
- The skill `rex-codex-adversarial-review`. Every plan job runs it before its plan is final: the Codex CLI challenges the plan read-only, the runner checks each finding, and there is one rebuttal round. Without Codex, a grid agent of `model-high-effort-high` or above is the challenger.
- Two size rules in `rex skills check`. `H9`: `description` and `when_to_use` together have at most 60 words, because Claude Code lists every skill in every session. `B5`: the text of a `SKILL.md` after its header has at most 1,500 words, counted as `wc -w` counts them.

### Changed

- The builder's stage skills now carry the `orchestration-` prefix: `nuance-selection` is `orchestration-nuance-selection` and `work-breakdown` is `orchestration-work-breakdown`. A repo set up by an earlier `rex init` gets the new folders on its next run; delete the two old folders from `.claude/skills/` by hand.
- The three stage skills `orchestration-nuance-selection`, `orchestration-work-breakdown` and `orchestration-selection` no longer appear in the `/` menu (`user-invocable: false`). Only the builder runs them, because each stage needs the job's inputs; Claude can still load them.
- `rex watch` has a colour theme: coloured statuses and states with symbols, rounded panels, a top bar with counts, a details panel above the run tree, and tree lines.
- The bundled skills are split into a core and sibling files. Each `SKILL.md` is a short core that an agent always reads, with the rules it needs every time, and says when to read each sibling file in the skill's folder, which holds the text for one step or one case. A repo that already has the skills must run `rex init --force` to get the new `SKILL.md` files, because `rex init` does not overwrite a skill file that exists. Without `--force` it adds the sibling files and keeps the old, long `SKILL.md` files. `--force` replaces every skill and agent file that differs from this version, so a change you made to a bundled skill is lost.
- Rule `B4` of `rex skills check` also checks a path to a sibling file, such as `.claude/skills/rex-deliver/start.md`: the skill must be known, and the file must be in its folder or in a folder below it. A path with a placeholder, such as `.claude/skills/<name>/<file>.md`, is a template and is not checked.
- The builder hands its decision to a decision agent. A sub-agent with the role `decision-maker` runs at the session's model and effort, works through the three stages, and returns one decision block: the skills, the work breakdown, the orchestrator and the role table. The paired challenger checks that block, and the decision agent corrects it, so the session never reads the stage skills and does not carry them while it runs and delivers the job. Each task in the work breakdown now names the skills it needs, and a brief names only the skills of its own task, so a worker no longer reads every skill of its role. The four stage skills set `disable-model-invocation: true`, because only the builder flow reads them, by their paths.
- A read of any Markdown file in a skill's folder, at any depth, now counts as opening that skill in the job record, not only a read of its `SKILL.md`. That holds for the skills folders `.claude/skills/<name>/` and for the orchestrator library `.rex/orchestrators/<name>/`. A skill counts once however many of its files are read, and a Markdown file outside a skill's folder, such as `.claude/skills/README.md`, opens no skill.

## 0.1.0 - 2026-11-10

The first version. SuperRex supersedes rex-skills (Rex 2.0).

### Added

- The `rex` binary, with the commands `init`, `job` (`create`, `plan`, `close`, `show`), `hook`, `status`, `oms`, `skills` (`list`, `check`), `recall`, `jobs`, `watch`, `feedback`, `codebase`, `smells` and `commands`.
- `rex init`, which sets a repo up in one run that is safe to repeat: the skills and agents, the Claude Code sandbox settings and hooks, the store in `.rex/`, the rex sections of `AGENTS.md`, the block in `.gitignore`, and a pre-commit config.
- 28 skills: the builder `orchestration-builder` with its three stages, the delivery skills `rex-deliver`, `rex-pr-review` and `rex-feedback`, 18 `rex-code-*` rulebooks, `rex-grill-me`, `rex-mermaid-diagrams` and `rex-skill-language`.
- The grid of 20 agents, one for each pair of model tier and effort level, and `rex oms`, the fixed function that picks one from a role's ratings.
- A local record of each job in `.rex/rex.db`: the request, the plan, the roles, the sub-agent runs with their tokens, the deliveries and the feedback.
- Telemetry through three Claude Code hooks, with tokens summed from the transcripts. A stop event that Claude Code sends for a call of its own, such as the reply it suggests after a turn, gives no run.
- `rex watch`, a live view of jobs and their sub-agent runs.
- Lessons: `rex feedback` stores the cause and the lesson of a job that missed, and `rex recall` gives them to later jobs that are similar.
- Commands outside the sandbox. A sandboxed command reaches the network only through the sandbox's proxy, so a program that opens its own connection, such as a WebSocket client or a database driver, cannot connect. `rex init` writes `sandbox.allowUnsandboxedCommands: true`, so that an agent can run such a command again outside the sandbox, and the `orchestration-builder` skill says when an agent does that. `rex init --strict` writes `false`, so that no command leaves the sandbox.

### Changed

- The installer of rex-skills, `rex skills`, is now `rex init`. `rex skills` now lists and checks skills.

### If you used a build from before this release

- A settings file that such a build wrote holds `sandbox.allowUnsandboxedCommands: false`. `rex init` keeps that value and warns. Set it to `true` by hand to let agents run a command outside the sandbox.
- Run `rex init --force` in a repo that is already set up, to get the skill text of this version.
- Such a build recorded a run for each reply that Claude Code suggests after a turn. Those runs stay in the store. This version records no new ones.

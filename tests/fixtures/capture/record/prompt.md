# Prompt and command line of the recorded session

Started with the working folder = the throwaway repo, all `CLAUDE*` variables of any outer
Claude Code session unset, and the prompt on standard input (because `--add-dir` and
`--mcp-config` take several values and would swallow a prompt given as an argument):

```sh
for v in $(compgen -e | grep '^CLAUDE'); do unset "$v"; done   # bash
cd <throwaway repo>
claude -p --dangerously-skip-permissions --model sonnet --effort low \
  --output-format stream-json --verbose --include-hook-events \
  --setting-sources project,local --strict-mcp-config --mcp-config '{"mcpServers":{}}' \
  --max-budget-usd 2 < prompt.txt > tree.stream.jsonl
```

The prompt, exactly (everything between the two lines of `=`):

```text
================================================================================
This is a test of agent tooling. Do exactly the steps below, in order, and nothing else. Do no work yourself except starting agents. Keep every reply short.

Each step starts one agent with the Agent tool. Use exactly the subagent_type and description given. The prompt of each agent must be EXACTLY the text between the lines `----- BRIEF -----` and `----- END -----`, character for character, including its first lines and the blank line after them. Do not add to it or change it.

Step 1. subagent_type "model-low-effort-medium", description "bg", run_in_background true.
----- BRIEF -----
Rex-Job: k3x9p2m7qd
Rex-Role: background-worker

You are a test agent. Do these steps in order, then reply with the one line BG DONE.
1. Bash tool, in the foreground (do NOT set run_in_background): sleep 5
2. Edit tool on the file bg.txt in your working folder: replace the line "one" with the two lines "one" and "two".
3. Agent tool: subagent_type "model-low-effort-low", description "bg-child", foreground. Its prompt must be exactly these four lines (the third line is empty):
Rex-Job: k3x9p2m7qd
Rex-Role: worker

Read the file bg.txt in your working folder with the Read tool, then reply with the one line BG-CHILD DONE.
----- END -----

Step 2. Wait for step 1's agent to be launched, then: subagent_type "model-medium-effort-low", description "lead", foreground.
----- BRIEF -----
Rex-Job: k3x9p2m7qd
Rex-Role: lead

You are a test agent. Do these steps in order, then reply with the one line LEAD DONE.
1. Read tool: read the file notes.txt in your working folder.
2. Bash tool: wc -l notes.txt
3. Edit tool, one call, on notes.txt: old_string is the three lines "alpha", "beta", "gamma" (joined by newlines, no newline at the end); new_string is the four lines "alpha", "BETA", "gamma", "delta" (joined by newlines, no newline at the end).
4. Agent tool: subagent_type "model-low-effort-low", description "worker", foreground. Its prompt must be exactly these four lines (the third line is empty):
Rex-Job: k3x9p2m7qd
Rex-Role: worker

Write tool: create the file worker.txt in your working folder with the two lines "w1" and "w2". Then reply with the one line WORKER DONE.
----- END -----

Step 3. subagent_type "model-high-effort-low", description "reviewer", foreground.
----- BRIEF -----
Rex-Job: k3x9p2m7qd
Rex-Role: reviewer

You are a test agent. Read the file notes.txt in your working folder with the Read tool, then reply with the one line REVIEWER DONE.
----- END -----

Step 4. subagent_type "general-purpose", description "plain", foreground.
----- BRIEF -----
You are a test agent. Do not use any tool. Reply with the one line PLAIN DONE.
----- END -----

Step 5. Wait until the background agent of step 1 has sent its completion notice. Do not end before it has. Then reply with one line for each agent: its description and its final line.
================================================================================
```

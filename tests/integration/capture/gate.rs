//! `rex gate` and `rex limit` on a store that holds the recorded session: a new
//! sub-agent starts under the limit, is refused over it, starts again once the
//! limit is raised, and starts whenever the gate cannot tell.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use serde_json::{Value, json};
use super_rex::store::StoreError;

use super::recorded::{AFTER_SESSION, BEFORE_SESSION, Fixture, JOB_A, SESSION};

fn rex(cwd: &Path, args: &[&str]) -> Command {
    let mut command = Command::cargo_bin("rex").unwrap();
    command
        .args(args)
        .current_dir(cwd)
        .env_remove("REX_STORE")
        .env_remove("CLAUDE_PROJECT_DIR");
    command
}

/// A PreToolUse event of the recorded session, for a call of the Agent tool.
fn agent_call(session: &str) -> String {
    json!({
        "session_id": session,
        "cwd": "/home/fixture/work/repo",
        "permission_mode": "default",
        "hook_event_name": "PreToolUse",
        "tool_name": "Agent",
        "tool_input": { "description": "worker", "prompt": "Rex-Job: k3x9p2m7qd\nRex-Role: worker\n\nDo it.", "subagent_type": "model-low-effort-low" },
        "tool_use_id": "toolu_01ABC"
    })
    .to_string()
}

/// Runs `rex gate` with `input` and returns its stdout, after checking that it
/// exits 0 and prints nothing on stderr.
fn gate(cwd: &Path, input: &str) -> String {
    let output = rex(cwd, &["gate"]).write_stdin(input).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "exit code");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "",
        "standard error"
    );
    String::from_utf8(output.stdout).unwrap()
}

fn limit(cwd: &Path, args: &[&str]) -> String {
    let mut all = vec!["limit", JOB_A];
    all.extend_from_slice(args);
    let output = rex(cwd, &all).assert().success().get_output().clone();
    assert!(output.stderr.is_empty(), "something on standard error");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn gate_refuses_only_over_the_limit() {
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    fixture.append(&fixture.lines.clone());
    let root = fixture.store.paths().root.clone();
    let spent = fixture.read(JOB_A, AFTER_SESSION).cost.unwrap();
    let spent_usd = format!(
        "{}.{:06}",
        spent.nanodollars / 1_000_000_000,
        spent.nanodollars / 1_000 % 1_000_000
    );

    // A small job that stated no budget: 5 agents and the main session, which
    // counts as 2, at the medium $4 each.
    assert_eq!(
        limit(&root, &[]),
        format!("limit_usd=28.000000\nspent_usd={spent_usd}\nsource=default\n")
    );
    assert_eq!(gate(&root, &agent_call(SESSION)), "", "under the limit");

    // The limit of a job that no person set: agent cap of its size plus 2 for the
    // main session, times the dollars for one agent of its budget.
    let table = [
        (Some("low"), ["10.500000", "12.000000", "18.000000"]),
        (Some("medium"), ["28.000000", "32.000000", "48.000000"]),
        (Some("high"), ["56.000000", "64.000000", "96.000000"]),
        (None, ["28.000000", "32.000000", "48.000000"]),
    ];
    for (budget, limits) in table {
        for (size, expected) in ["small", "medium", "large"].into_iter().zip(limits) {
            fixture
                .store
                .write(|tx| {
                    tx.execute(
                        "UPDATE job SET size = ?2, pref_budget = ?3 WHERE id = ?1",
                        rusqlite::params![JOB_A, size, budget],
                    )?;
                    Ok::<_, StoreError>(())
                })
                .unwrap();
            let shown = limit(&root, &[]);
            assert_eq!(
                shown.lines().next().unwrap(),
                format!("limit_usd={expected}"),
                "{size} job, budget {budget:?}"
            );
        }
    }

    assert_eq!(
        limit(&root, &["--set", "0.10"]),
        format!("limit_usd=0.100000\nspent_usd={spent_usd}\nsource=set\n")
    );
    let refused: Value = serde_json::from_str(&gate(&root, &agent_call(SESSION))).unwrap();
    assert_eq!(
        refused,
        json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason":
                    "SuperRex spend limit reached: job k3x9p2m7qd has spent $0.186 of its $0.100 limit, \
                     so this agent was not started. Start no more agents. If you are a sub-agent, report \
                     the state blocked to the agent that started you, with the spend ($0.186), the limit \
                     ($0.100) and the raise command below. If you are the main session, keep the job \
                     open: tell the requester the spend, the limit and the raise command, and go on with \
                     the same job once they have run it. Close the job blocked only if they decline. \
                     Raise command: rex limit k3x9p2m7qd --set 0.20"
            }
        })
    );

    limit(&root, &["--set", "1"]);
    assert_eq!(gate(&root, &agent_call(SESSION)), "", "a raised limit");

    let help = rex(&root, &["limit", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(
        String::from_utf8(help)
            .unwrap()
            .contains("The job whose spend and limit to print"),
        "the job argument has its help line"
    );

    for bad in ["0", "-1", "ten", "1.2.3", ""] {
        rex(&root, &["limit", JOB_A, "--set", bad])
            .assert()
            .code(2)
            .stdout("");
    }

    // Every case the gate cannot judge lets the agent start, with nothing printed.
    limit(&root, &["--set", "0.10"]);
    let other_event = agent_call(SESSION).replace("PreToolUse", "PostToolUse");
    let other_tool = agent_call(SESSION).replace("\"Agent\"", "\"Bash\"");
    let failures = [
        "".to_owned(),
        "not json {".to_owned(),
        json!({ "hook_event_name": "PreToolUse", "tool_name": "Agent" }).to_string(),
        other_event,
        other_tool,
        agent_call("a-session-with-no-open-job"),
    ];
    for input in &failures {
        assert_eq!(gate(&root, input), "", "input {input:?}");
    }
    let no_store = tempfile::tempdir().unwrap();
    fs::create_dir(no_store.path().join(".git")).unwrap();
    assert_eq!(gate(no_store.path(), &agent_call(SESSION)), "", "no store");
    let no_repo = tempfile::tempdir().unwrap();
    assert_eq!(gate(no_repo.path(), &agent_call(SESSION)), "", "no repo");
}

/// The builder plans each job inside the agent cap and the dollars for one agent
/// that its stage 3 text states, so `rex limit` must give the limit those numbers
/// make: (cap + 2 for the main session) times the dollars of the budget.
#[test]
fn the_default_limits_agree_with_the_builder_skill() {
    let stage = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".claude/skills/orchestration-builder/stages/3-orchestration/stage.md"),
    )
    .unwrap();
    // A row such as `| small | 5 |`; the size may also be in backticks.
    let cap = |size: &str| -> u64 {
        stage
            .lines()
            .find_map(|line| {
                let cells: Vec<&str> = line.split('|').map(str::trim).collect();
                match cells.as_slice() {
                    ["", name, count, ""] if name.trim_matches('`') == size => count.parse().ok(),
                    _ => None,
                }
            })
            .unwrap_or_else(|| panic!("no row for {size} in the agent cap table"))
    };
    let dollars = |budget: &str| -> f64 {
        let mark = format!("`{budget}` $");
        let line = stage
            .lines()
            .find(|line| line.contains("Spend limit") && line.contains(&mark))
            .unwrap_or_else(|| panic!("no dollars for {budget} in the spend limit line"));
        let after = &line[line.find(&mark).unwrap() + mark.len()..];
        let amount: String = after
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        amount.trim_end_matches('.').parse().unwrap()
    };

    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let root = fixture.store.paths().root.clone();
    for budget in ["low", "medium", "high"] {
        for size in ["small", "medium", "large"] {
            fixture
                .store
                .write(|tx| {
                    tx.execute(
                        "UPDATE job SET size = ?2, pref_budget = ?3 WHERE id = ?1",
                        rusqlite::params![JOB_A, size, budget],
                    )?;
                    Ok::<_, StoreError>(())
                })
                .unwrap();
            let expected = (cap(size) + 2) as f64 * dollars(budget);
            assert_eq!(
                limit(&root, &[]).lines().next().unwrap(),
                format!("limit_usd={expected:.6}"),
                "{size} job, budget {budget}: the code and the builder skill disagree"
            );
        }
    }
}

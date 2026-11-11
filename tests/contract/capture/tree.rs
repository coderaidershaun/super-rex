//! The recorded session, taken in whole, gives the tree and the totals that the
//! fixture's own script worked out; the order of its lines does not matter.

use serde_json::{Value, json};
use super_rex::capture::{RunState, TokenTotal, take_in};
use super_rex::store::StoreError;

use super::fixture::*;

/// WORKER's brief, as JSON text in its completed call.
const WORKER_BRIEF: &str = r"Rex-Job: k3x9p2m7qd\nRex-Role: worker\n\nWrite tool";

/// REVIEWER opens one skill by the Skill tool and reads the `SKILL.md` of a library
/// skill. It reads files beside a `SKILL.md`: of a skill it did not open otherwise, of a
/// library skill one folder deeper, of a library skill it already opened, and of an
/// orchestrator in the library. Its read of another `SKILL.md` and of another skill's
/// file fail. Its reads of a Markdown file in the skill library itself, of one outside
/// any skill, and of a skill's file that is not Markdown open no skill. The calls are in
/// a record of a request already counted.
const SKILL_RECORDS: &str = r#"{"type":"assistant","timestamp":"2026-11-10T13:10:39.890Z","message":{"id":"msg_fixture0012","model":"claude-opus-5-5","content":[{"type":"tool_use","id":"toolu_skill","name":"Skill","input":{"skill":"rex-code-tdd"}},{"type":"tool_use","id":"toolu_read","name":"Read","input":{"file_path":"/repo/.rex/skills/rex-code-commenting/SKILL.md"}},{"type":"tool_use","id":"toolu_blocked","name":"Read","input":{"file_path":"/repo/.rex/skills/rex-code-smells/SKILL.md"}},{"type":"tool_use","id":"toolu_sibling","name":"Read","input":{"file_path":"/repo/.claude/skills/rex-deliver/parallel-writers.md"}},{"type":"tool_use","id":"toolu_deeper","name":"Read","input":{"file_path":"/repo/.rex/skills/rex-code-latency/os/jitter.md"}},{"type":"tool_use","id":"toolu_again","name":"Read","input":{"file_path":"/repo/.rex/skills/rex-code-commenting/examples.md"}},{"type":"tool_use","id":"toolu_library","name":"Read","input":{"file_path":"/repo/.rex/orchestrators/solo-engineer/roles.md"}},{"type":"tool_use","id":"toolu_blocked_sibling","name":"Read","input":{"file_path":"/repo/.claude/skills/rex-pr-review/end.md"}},{"type":"tool_use","id":"toolu_skills_folder","name":"Read","input":{"file_path":"/repo/.rex/skills/README.md"}},{"type":"tool_use","id":"toolu_docs","name":"Read","input":{"file_path":"/repo/docs/notes.md"}},{"type":"tool_use","id":"toolu_not_markdown","name":"Read","input":{"file_path":"/repo/.claude/skills/rex-grill-me/notes.txt"}}]}}
{"type":"user","timestamp":"2026-11-10T13:10:39.891Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_read","content":"skill text"},{"type":"tool_result","tool_use_id":"toolu_blocked","is_error":true,"content":"blocked"},{"type":"tool_result","tool_use_id":"toolu_blocked_sibling","is_error":true,"content":"blocked"}]}}
"#;

#[test]
fn recorded_session_gives_correct_tree() {
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    // WORKER's brief names another job. The tag never moves the run.
    let other_job = WORKER_BRIEF.replace(JOB_A, JOB_B);
    let lines: Vec<String> = fixture
        .lines
        .iter()
        .map(|line| line.replace(WORKER_BRIEF, &other_job))
        .collect();
    assert_ne!(lines, fixture.lines, "WORKER's brief not changed");
    fixture.append(&lines);
    append_text(&fixture.transcript(REVIEWER), SKILL_RECORDS);

    let runs = fixture.read(JOB_A, AFTER_SESSION);

    let expected = expected();
    let mut all_tokens = expected_tokens(&expected["main_session"]["tokens"]);
    for agent in expected["agents"].as_array().unwrap() {
        let id = agent["agent_id"].as_str().unwrap();
        let run = run(&runs, id);
        let text = |key: &str| agent[key].as_str().map(str::to_owned);
        let counts = |key: &str| agent["transcript_counts"][key].as_u64();
        assert_eq!(run.parent_agent_id, text("parent"), "{id} parent");
        assert_eq!(
            run.depth,
            agent["depth"].as_u64().map(|depth| depth as u32),
            "{id} depth"
        );
        assert_eq!(run.agent_type, text("agent_type"), "{id} agent type");
        assert_eq!(run.model_ran, text("resolved_model"), "{id} model");
        assert_eq!(run.effort_ran, text("effort"), "{id} effort");
        assert_eq!(
            run.background,
            agent["background"].as_bool(),
            "{id} background"
        );
        assert_eq!(run.role, text("role"), "{id} role");
        let tag_job = (id == WORKER).then(|| JOB_B.to_owned());
        assert_eq!(run.tag_job, tag_job, "{id} tag job");
        assert_eq!(run.tool_uses, counts("tool_uses"), "{id} tool uses");
        assert_eq!(run.lines_added, counts("lines_added"), "{id} lines added");
        assert_eq!(
            run.lines_removed,
            counts("lines_removed"),
            "{id} lines removed"
        );
        let tokens = expected_tokens(&agent["tokens"]);
        assert_eq!(run.tokens, TokenTotal::Provisional(tokens), "{id} tokens");
        assert_eq!(run.state, RunState::Finished, "{id} state");
        let skills: &[&str] = if id == REVIEWER {
            &[
                "rex-code-commenting",
                "rex-code-latency",
                "rex-code-tdd",
                "rex-deliver",
                "solo-engineer",
            ]
        } else {
            &[]
        };
        // Every run's transcript was read, so a run that opened no skill has
        // an empty list, not `None`.
        let skills: Vec<String> = skills.iter().map(|&skill| skill.to_owned()).collect();
        assert_eq!(run.skills_opened, Some(skills), "{id} skills");
        all_tokens += tokens;
    }
    assert_eq!(
        agent_ids(&runs),
        [BG, LEAD, BG_CHILD, WORKER, REVIEWER, PLAIN]
    );
    assert_eq!((runs.open, runs.finished, runs.unconfirmed), (0, 6, 0));

    let main_session = expected_tokens(&expected["main_session"]["tokens"]);
    assert_eq!(runs.session_tokens, TokenTotal::Provisional(main_session));
    assert_eq!(runs.tokens, TokenTotal::Provisional(all_tokens));
    assert_eq!(all_tokens, expected_tokens(&expected["modelUsage"]));
    assert!(!runs.incomplete);
    assert_eq!(runs.damaged_log_lines, 0);
}

#[test]
fn lines_twice_or_in_any_order_change_nothing() {
    let recorded = {
        let mut fixture = Fixture::new();
        fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
        fixture.append(&fixture.lines.clone());
        fixture.read(JOB_A, AFTER_SESSION).runs
    };

    let mut twice = Fixture::new();
    twice.open_job(JOB_A, BEFORE_SESSION).unwrap();
    twice.append(&twice.lines.clone());
    take_in(&mut twice.store).unwrap();
    forget_offset(&mut twice);
    take_in(&mut twice.store).unwrap();
    assert_eq!(
        twice.read(JOB_A, AFTER_SESSION).runs,
        recorded,
        "taken in twice"
    );

    let mut reversed = Fixture::new();
    reversed.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let mut lines = reversed.lines.clone();
    lines.reverse();
    for line in &lines {
        reversed.append(&[line]);
        take_in(&mut reversed.store).unwrap();
    }
    assert_eq!(
        reversed.read(JOB_A, AFTER_SESSION).runs,
        recorded,
        "reversed"
    );

    let mut named_task = Fixture::new();
    named_task.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let lines: Vec<String> = named_task
        .lines
        .iter()
        .map(|line| line.replace(r#""tool_name":"Agent""#, r#""tool_name":"Task""#))
        .collect();
    assert_ne!(lines, named_task.lines, "no tool name changed");
    named_task.append(&lines);
    assert_eq!(
        named_task.read(JOB_A, AFTER_SESSION).runs,
        recorded,
        "sub-agent tool named Task"
    );
}

/// Claude Code also sends a stop line for a call it makes on its own, such as the
/// reply it suggests after a turn. The agent type is empty, and no start line, call
/// or transcript goes with it.
#[test]
fn stop_line_of_claude_codes_own_call_makes_no_run() {
    let recorded = {
        let mut fixture = Fixture::new();
        fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
        fixture.append(&fixture.lines.clone());
        fixture.read(JOB_A, AFTER_SESSION)
    };

    let own_call = "a1f16da89dcf482e4";
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let mut line: Value = serde_json::from_str(&fixture.lines_of(PLAIN)[1]).unwrap();
    let event = &mut line["event"];
    assert_eq!(event["hook_event_name"], "SubagentStop");
    event["agent_id"] = json!(own_call);
    event["agent_type"] = json!("");
    event["last_assistant_message"] = json!("yes, run a review before merging");
    event["agent_transcript_path"] = json!(fixture.transcript(own_call).to_str().unwrap());
    let mut lines = fixture.lines.clone();
    lines.push(line.to_string());
    fixture.append(&lines);

    let runs = fixture.read(JOB_A, AFTER_SESSION);

    assert_eq!(runs.runs, recorded.runs);
    assert_eq!((runs.open, runs.finished, runs.unconfirmed), (0, 6, 0));
    assert!(!runs.incomplete);
}

/// Makes the next intake read the whole log again, as if the offset were lost.
fn forget_offset(fixture: &mut Fixture) {
    fixture
        .store
        .write(|tx| {
            tx.execute(
                "UPDATE meta SET value = '0' WHERE key = 'intake.offset'",
                [],
            )?;
            Ok::<_, StoreError>(())
        })
        .unwrap();
}

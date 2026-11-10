//! The recorded session, taken in whole, gives the tree and the totals that the
//! fixture's own script worked out; the order of its lines does not matter.

use super_rex::capture::{RunState, TokenTotal, take_in};
use super_rex::store::StoreError;

use super::fixture::*;

/// WORKER's brief, as JSON text in its completed call.
const WORKER_BRIEF: &str = r"Rex-Job: k3x9p2m7qd\nRex-Role: worker\n\nWrite tool";

/// REVIEWER opens one skill by the Skill tool and reads the `SKILL.md` of another;
/// its read of a third fails. The calls are in a record of a request already counted.
const SKILL_RECORDS: &str = r#"{"type":"assistant","timestamp":"2026-11-10T13:10:39.890Z","message":{"id":"msg_fixture0012","model":"claude-opus-5-5","content":[{"type":"tool_use","id":"toolu_skill","name":"Skill","input":{"skill":"rex-code-tdd"}},{"type":"tool_use","id":"toolu_read","name":"Read","input":{"file_path":"/repo/.claude/skills/rex-code-commenting/SKILL.md"}},{"type":"tool_use","id":"toolu_blocked","name":"Read","input":{"file_path":"/repo/.claude/skills/rex-code-smells/SKILL.md"}}]}}
{"type":"user","timestamp":"2026-11-10T13:10:39.891Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_read","content":"skill text"},{"type":"tool_result","tool_use_id":"toolu_blocked","is_error":true,"content":"blocked"}]}}
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
            &["rex-code-commenting", "rex-code-tdd"]
        } else {
            &[]
        };
        assert_eq!(run.skills_opened, skills, "{id} skills");
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

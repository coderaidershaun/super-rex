//! What `rex job plan` promises about the selection function: each role's
//! stored record is exactly what the function returns for the job's and the
//! role's inputs, so the record and the function never disagree.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use rusqlite::Row;
use super_rex::oms::{self, Inputs, Level};
use super_rex::store::{Store, StorePaths};

use super::store::git;

fn rex(cwd: &Path, args: &[&str], stdin: &str) -> String {
    let output = Command::cargo_bin("rex")
        .unwrap()
        .current_dir(cwd)
        .env_remove("REX_STORE")
        .env_remove("CLAUDE_PROJECT_DIR")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_EFFORT")
        .args(args)
        .write_stdin(stdin.to_owned())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "rex {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn level(row: &Row<'_>, column: &str) -> Option<Level> {
    let text: Option<String> = row.get(column).unwrap();
    match text.as_deref() {
        None => None,
        Some("low") => Some(Level::Low),
        Some("medium") => Some(Level::Medium),
        Some("high") => Some(Level::High),
        Some(other) => panic!("{column} holds {other}"),
    }
}

/// What a role row holds after the plan.
#[derive(Debug, PartialEq, Eq)]
struct Record {
    agent: Option<String>,
    demand: Option<String>,
    rules: Option<String>,
    conflict: Option<String>,
    oms_version: Option<u32>,
}

#[test]
fn plan_records_what_oms_returns() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("shop");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    let paths = StorePaths::locate(&repo).unwrap();
    Store::create(&paths).unwrap();
    fs::create_dir_all(paths.orchestrators.join("pipeline")).unwrap();

    let created = rex(
        &repo,
        &[
            "job",
            "create",
            "--session",
            "s1",
            "--request-file",
            "-",
            "--asked-by",
            "human",
            "--merge",
            "yes",
            "--type",
            "feature",
            "--size",
            "large",
            "--consequence",
            "high",
            "--lang",
            "rust",
            "--duration",
            "long",
            "--budget",
            "low",
            "--autonomy",
            "high",
        ],
        "Add a search page.",
    );
    let job = created
        .lines()
        .next()
        .unwrap()
        .strip_prefix("job=")
        .unwrap();
    let plan = r#"{"orchestrator": "pipeline", "skills": [], "breakdown": "Build it.", "roles": [
        {"role": "architect", "intelligence": "high", "reliability": "high", "consequence": "high", "verification": "high"},
        {"role": "engineer", "intelligence": "medium", "reliability": "none", "consequence": "low", "verification": "low"},
        {"role": "scout", "intelligence": "none", "reliability": "none", "consequence": "none", "verification": "none"}]}"#;
    rex(&repo, &["job", "plan", job, "--plan-file", "-"], plan);

    let store = Store::open(&paths).unwrap();
    let mut statement = store
        .conn()
        .prepare("SELECT * FROM role WHERE job_id = ?1 ORDER BY rowid")
        .unwrap();
    let rows: Vec<(String, Inputs, Record)> = statement
        .query_map([job], |row| {
            let inputs = Inputs {
                duration: level(row, "in_duration"),
                budget: level(row, "in_budget"),
                intelligence: level(row, "in_intelligence"),
                autonomy: level(row, "in_autonomy"),
                reliability: level(row, "in_reliability"),
                consequence: level(row, "in_consequence"),
                verification: level(row, "in_verification"),
            };
            let record = Record {
                agent: row.get("agent")?,
                demand: row.get("demand")?,
                rules: row.get("rules")?,
                conflict: row.get("conflict")?,
                oms_version: row.get("oms_version")?,
            };
            Ok((row.get("role")?, inputs, record))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    let names: Vec<&str> = rows.iter().map(|(name, ..)| name.as_str()).collect();
    assert_eq!(names, ["architect", "engineer", "scout"]);
    let job_inputs = Inputs {
        duration: Some(Level::High),
        budget: Some(Level::Low),
        autonomy: Some(Level::High),
        ..Inputs::default()
    };
    let role_inputs = [
        (
            Some(Level::High),
            Some(Level::High),
            Some(Level::High),
            Some(Level::High),
        ),
        (
            Some(Level::Medium),
            None,
            Some(Level::Low),
            Some(Level::Low),
        ),
        (None, None, None, None),
    ];
    for ((name, inputs, record), role) in rows.iter().zip(role_inputs) {
        let (intelligence, reliability, consequence, verification) = role;
        let expected_inputs = Inputs {
            intelligence,
            reliability,
            consequence,
            verification,
            ..job_inputs
        };
        assert_eq!(*inputs, expected_inputs, "{name}");

        let selection = oms::select(expected_inputs);
        let rules: Vec<&str> = selection.rules.iter().map(|rule| rule.id()).collect();
        let conflict: Vec<&str> = selection.conflict.iter().map(|cap| cap.name()).collect();
        let expected = Record {
            agent: Some(selection.agent.to_string()),
            demand: Some(selection.demand.to_string()),
            rules: Some(rules.join(",")),
            conflict: Some(if conflict.is_empty() {
                "none".to_owned()
            } else {
                conflict.join(",")
            }),
            oms_version: Some(oms::VERSION),
        };
        assert_eq!(*record, expected, "{name}");
    }
}

//! `rex limit` on a store that holds the recorded session: the default limit of
//! each size and budget, a limit a person sets, and an amount it refuses.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use super_rex::store::StoreError;

use super::recorded::{AFTER_SESSION, BEFORE_SESSION, Fixture, JOB_A};

fn rex(cwd: &Path, args: &[&str]) -> Command {
    let mut command = Command::cargo_bin("rex").unwrap();
    command
        .args(args)
        .current_dir(cwd)
        .env_remove("REX_STORE")
        .env_remove("CLAUDE_PROJECT_DIR");
    command
}

fn limit(cwd: &Path, args: &[&str]) -> String {
    let mut all = vec!["limit", JOB_A];
    all.extend_from_slice(args);
    let output = rex(cwd, &all).assert().success().get_output().clone();
    assert!(output.stderr.is_empty(), "something on standard error");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn limit_shows_the_default_and_the_set_limit() {
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
}

/// The limit table the plugin gives a new job, which it reads from the shared
/// fixtures, is the binary's own default for every size and budget.
#[test]
fn the_shared_limit_table_is_the_default_of_rex_limit() {
    let table: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/shared/tables/limits.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let root = fixture.store.paths().root.clone();
    for (size, row) in table["default_limit_nano"].as_object().unwrap() {
        for (budget, nano) in row.as_object().unwrap() {
            let budget = (budget != "none").then_some(budget.as_str());
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
            let nano = nano.as_u64().unwrap();
            assert_eq!(
                limit(&root, &[]).lines().next().unwrap(),
                format!(
                    "limit_usd={}.{:06}",
                    nano / 1_000_000_000,
                    nano / 1_000 % 1_000_000
                ),
                "{size} job, budget {budget:?}"
            );
        }
    }
}

/// The builder plans each job inside the agent cap and the dollars for one agent
/// that its stage 3 text states, so `rex limit` must give the limit those numbers
/// make: (cap + 2 for the main session) times the dollars of the budget.
#[test]
fn the_default_limits_agree_with_the_builder_skill() {
    let stage = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugin/skills/orchestration-builder/stages/3-orchestration/stage.md"),
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

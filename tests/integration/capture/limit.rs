//! `rex limit` on a store that holds the recorded session: the default limit,
//! which is the same for every size and budget, a limit a person sets, and an
//! amount it refuses.

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

    // A job with no limit of its own gets the default of every job.
    assert_eq!(
        limit(&root, &[]),
        format!("limit_usd=100000.000000\nspent_usd={spent_usd}\nsource=default\n")
    );

    // The default is the same for every size and budget.
    for budget in [Some("low"), Some("medium"), Some("high"), None] {
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
            let shown = limit(&root, &[]);
            assert_eq!(
                shown.lines().next().unwrap(),
                "limit_usd=100000.000000",
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

/// The default limit the plugin gives a new job, which it reads from the shared
/// fixtures, is the binary's own default.
#[test]
fn the_shared_default_limit_is_the_default_of_rex_limit() {
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
    let nano = table["default_limit_nano"].as_u64().unwrap();
    assert_eq!(
        limit(&root, &[]).lines().next().unwrap(),
        format!(
            "limit_usd={}.{:06}",
            nano / 1_000_000_000,
            nano / 1_000 % 1_000_000
        ),
    );
}

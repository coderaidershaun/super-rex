//! What the orchestrator index promises to `rex job plan` and `rex recall`: a
//! version number that changes when, and only when, a folder's content changes.

use std::fs;
use std::path::Path;

use super_rex::store::orchestrator::{self, OrchestratorVersion};
use super_rex::store::{Store, StorePaths};

fn describe(skill_file: &Path) -> Option<String> {
    let text = fs::read_to_string(skill_file).ok()?;
    Some(text.trim().to_owned())
}

fn index_all(store: &mut Store) -> Vec<OrchestratorVersion> {
    let library = store.paths().orchestrators.clone();
    store
        .write(|tx| orchestrator::index_all(tx, &library, &describe))
        .unwrap()
}

fn version(name: &str, version: i64, description: &str) -> OrchestratorVersion {
    OrchestratorVersion {
        name: name.to_owned(),
        version,
        description: description.to_owned(),
    }
}

fn row_count(store: &Store) -> i64 {
    store
        .conn()
        .query_row("SELECT count(*) FROM orchestrator", [], |row| row.get(0))
        .unwrap()
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn index_gives_a_new_version_only_when_content_changes() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("repo").join(".rex");
    let paths = StorePaths {
        root: temp.path().join("repo"),
        db: dir.join("rex.db"),
        event_log: dir.join("events.jsonl"),
        orchestrators: dir.join("orchestrators"),
        dir,
    };
    let mut store = Store::create(&paths).unwrap();
    let library = &paths.orchestrators;
    write(&library.join("pipeline/SKILL.md"), "Plan, build, review.");
    write(
        &library.join("pipeline/leads/engineer.md"),
        "Write the code.",
    );
    write(&library.join("research/notes.md"), "Read and report.");

    let first = index_all(&mut store);
    assert_eq!(
        first,
        [
            version("pipeline", 1, "Plan, build, review."),
            version("research", 1, ""),
        ]
    );

    assert_eq!(
        index_all(&mut store),
        first,
        "no change gives the same versions"
    );
    assert_eq!(row_count(&store), 2, "no change writes no row");

    write(
        &library.join("pipeline/leads/engineer.md"),
        "Write the code first.",
    );
    assert_eq!(
        index_all(&mut store),
        [
            version("pipeline", 2, "Plan, build, review."),
            version("research", 1, ""),
        ],
        "a changed file in a sub-folder gives that orchestrator a new version"
    );

    write(&library.join("research/.notes.md.swp"), "editor state");
    write(&library.join("pipeline/.cache/state"), "anything");
    let after_hidden = index_all(&mut store);
    assert_eq!(
        after_hidden[0].version, 2,
        "hidden files give no new version"
    );
    assert_eq!(
        after_hidden[1].version, 1,
        "hidden files give no new version"
    );
    assert_eq!(row_count(&store), 3);
}

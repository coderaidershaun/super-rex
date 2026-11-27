//! Every skill of the plugin's library has a `metadata` map that a search reads,
//! so no library skill is left out of search without a trace.

use std::fs;
use std::path::{Path, PathBuf};

use super_rex::commands::skills::has_metadata;

/// The skill folders of `library`, sorted, and the names of those whose
/// `SKILL.md` has no `metadata` map that a search reads.
fn skills_without_metadata(library: &Path) -> (Vec<String>, Vec<String>) {
    let mut skills: Vec<String> = fs::read_dir(library)
        .unwrap_or_else(|error| panic!("read {}: {error}", library.display()))
        .map(|entry| entry.expect("folder entry"))
        .filter(|entry| entry.path().join("SKILL.md").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    skills.sort();
    let missing = skills
        .iter()
        .filter(|skill| !has_metadata(&library.join(skill).join("SKILL.md")))
        .cloned()
        .collect();
    (skills, missing)
}

fn crate_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

// A library skill with no metadata map is never found by a search, and the
// agent that needed it never learns that it exists.
#[test]
fn every_plugin_library_skill_has_a_metadata_map() {
    let (skills, missing) = skills_without_metadata(&crate_path("plugin/library"));

    assert_eq!(skills.len(), 25, "{skills:?}");
    assert!(
        missing.is_empty(),
        "these library skills have no metadata map that a search reads: {missing:?}"
    );
}

#[test]
fn the_check_names_a_skill_with_no_metadata_map() {
    let (skills, missing) =
        skills_without_metadata(&crate_path("tests/fixtures/library-metadata/library"));

    assert_eq!(skills, ["author-only", "no-metadata", "with-metadata"]);
    assert_eq!(missing, ["author-only", "no-metadata"]);
}

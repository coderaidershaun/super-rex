//! Contract test of what `rex skills` gives to the store: an orchestrator's
//! one-line description.

use std::fs;
use std::path::Path;

use super_rex::commands::skills::describe;

/// The store reads an orchestrator's description through this function when
/// it indexes the library, so it must fit the store's `describe` argument and
/// give the one-line text.
#[test]
fn describe_gives_the_one_line_description_or_none() {
    let describe: &dyn Fn(&Path) -> Option<String> = &describe;
    let folder = tempfile::tempdir().unwrap();

    let folded = folder.path().join("folded.md");
    fs::write(
        &folded,
        "---\nname: a\ndescription: >\n  First part\n  second part.\n---\nBody\n",
    )
    .unwrap();
    let headerless = folder.path().join("headerless.md");
    fs::write(&headerless, "Just text.\n").unwrap();

    assert_eq!(
        describe(&folded).as_deref(),
        Some("First part second part.")
    );
    assert_eq!(describe(&headerless), None);
    assert_eq!(describe(&folder.path().join("missing.md")), None);
}

//! The master's procedure is the whole prompt of `super-rex:master`, so each
//! duty the mod leaves to the master's judgement must stand in it as a rule.

use std::fs;
use std::path::Path;

const PROCEDURE: &str = "plugin/skills/orchestration-builder/master/procedure.md";
const BUILDER_BRIEF: &str = "plugin/skills/orchestration-builder/master/builder.md";

fn read(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// The text of the section under `## <heading>`, to the next `## ` heading.
fn section<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let marker = format!("\n## {heading}\n");
    let start = text.find(&marker)? + marker.len();
    let rest = &text[start..];
    Some(rest.find("\n## ").map_or(rest, |end| &rest[..end]))
}

/// Each section the procedure must hold, the duty of the duty map it states,
/// and the words that show the rule is there, in any case.
const RULES: [(&str, &str, &[&str]); 6] = [
    (
        "Steps",
        "P20",
        &["cut", "divide", "join", "fix job", "plan job"],
    ),
    (
        "Sub-agent models",
        "P4",
        &["default", "top tier", "named risk"],
    ),
    ("Hand-off", "P10", &["Read", "output file", "done."]),
    (
        "Project matters",
        "P15, P16",
        &[
            "charter",
            "git stash",
            "git reset --hard",
            "force push",
            "admin merge",
        ],
    ),
    ("Notifications", "R2", &["main loop", "task notification"]),
    (
        "Final report",
        "P19",
        &["result", "evidence", "not run", "not proven", "not built"],
    ),
];

// The kit of `claude plugin test` reads no file, so this test is where the
// procedure's rules are held: a master whose prompt lost one of them would
// still pass every mod test.
#[test]
fn procedure_file_states_step_rules() {
    let text = read(PROCEDURE);
    let mut missing = Vec::new();
    for (heading, duty, words) in RULES {
        match section(&text, heading) {
            None => missing.push(format!("section `## {heading}` ({duty})")),
            Some(body) => missing.extend(
                words
                    .iter()
                    .filter(|word| !body.to_lowercase().contains(&word.to_lowercase()))
                    .map(|word| format!("`{word}` in `## {heading}` ({duty})")),
            ),
        }
    }
    assert!(
        missing.is_empty(),
        "{PROCEDURE} must state each rule the master obeys; it lacks:\n{}",
        missing.join("\n")
    );
}

// The builder brief is the only instruction a builder gets that no model
// wrote: it must send the builder into the builder skill and back to its master.
#[test]
fn builder_brief_sends_the_builder_to_its_skill_and_its_master() {
    let text = read(BUILDER_BRIEF);
    let missing: Vec<&str> = [
        "rex_skill",
        "orchestration-builder",
        "awaiting decision",
        "rex_job_open",
    ]
    .into_iter()
    .filter(|word| !text.contains(word))
    .collect();
    assert!(missing.is_empty(), "{BUILDER_BRIEF} lacks {missing:?}");
}

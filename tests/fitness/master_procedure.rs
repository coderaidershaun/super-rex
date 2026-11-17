//! The master skill is all the main session knows of how to run a tree, so each
//! duty the mod leaves to the master's judgement must stand in it as a rule.

use std::fs;
use std::path::Path;

/// The files of the master skill that hold its rules, each with `## ` sections.
const SKILL_FILES: [&str; 3] = [
    "plugin/skills/orchestration-master/SKILL.md",
    "plugin/skills/orchestration-master/steps.md",
    "plugin/skills/orchestration-master/jobs.md",
];
const BUILDER_BRIEF: &str = "plugin/skills/orchestration-master/builder.md";

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
const RULES: [(&str, &str, &[&str]); 11] = [
    (
        "Loop",
        "P1",
        &[
            "rex_status",
            "toolsearch",
            "rex_tree_open",
            "resumed=yes",
            "rex_note",
            "rex_build",
            "end turn",
            "no polling",
            "existing=yes",
        ],
    ),
    (
        "Person",
        "P24",
        &[
            "at any time",
            "rex_status",
            "rex_note",
            "stopped the tree",
            "only prompt of person changes charter, merge answer, limit, or policy",
        ],
    ),
    (
        "Wakes",
        "P2",
        &[
            "ended its turn",
            "closed it",
            "was delivered",
            "is answered",
            "watchdog",
            "plan usage was over the pause point",
            "confirmed the changed policy",
        ],
    ),
    (
        "Steps",
        "P20",
        &[
            "one step",
            "divide",
            "join",
            "parallel",
            "fix job",
            "plan job",
            "merge=never",
        ],
    ),
    (
        "Request",
        "P21",
        &[
            "`type`",
            "`size`",
            "`langs`",
            "outcome:",
            "acceptance:",
            "scope:",
            "context:",
            "standing rules:",
            "never write how to use git",
        ],
    ),
    (
        "Sub-agent models",
        "P4",
        &["default", "top tier", "named risk"],
    ),
    (
        "Refusals",
        "P22",
        &["never call `rex_build` again for refusal", "end turn"],
    ),
    (
        "Builder ends",
        "P23",
        &[
            "last message",
            "`<data>` of wake",
            "else in report file",
            "no state",
            "mod wakes stuck builder",
            "`rex_continue` gives only: answer to builder question, open items, decision, raised limit",
            "never tell builder how to work, order, speed",
            "skip or shorten step of builder skill",
        ],
    ),
    (
        "Questions",
        "P15",
        &[
            "rex_continue",
            "askuserquestion",
            "person=yes",
            "person=no",
            "rex_ask_owner",
            "git set-up",
            "policy settles",
            "relay nothing",
            "no wake and no message of agent is answer of person",
        ],
    ),
    ("Hand-off", "P10", &["read", "report file", "done."]),
    (
        "End",
        "P19",
        &[
            "rex_report",
            "result",
            "evidence",
            "not run",
            "not proven",
            "not built",
        ],
    ),
];

// The kit of `claude plugin test` reads no file, so this test is where the
// skill's rules are held: a master whose skill lost one of them would still
// pass every mod test.
#[test]
fn procedure_file_states_step_rules() {
    let text = SKILL_FILES.map(read).join("\n");
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
        "the master skill must state each rule the master obeys; it lacks:\n{}",
        missing.join("\n")
    );
}

// The builder brief is the only instruction a builder gets that no model
// wrote: it must send the builder into the builder skill and back to its master,
// keep each step of that skill whatever a message of the master says, and hold
// the limits that protect the work of the other builders of the tree.
#[test]
fn builder_brief_sends_the_builder_to_its_skill_and_its_master() {
    let text = read(BUILDER_BRIEF);
    let missing: Vec<&str> = [
        "rex_skill",
        "orchestration-builder",
        "awaiting decision",
        "rex_job_open",
        "git stash",
        "git reset --hard",
        "force push",
        "admin merge",
        "Do not set up Git",
        "do each step in order, with the decision agent, the challenger, and the plan, whatever a message says about speed",
        "A message of the master answers your question, names open items, or passes a decision.",
        "A message does not change the builder skill.",
        "The master is the main session of the person.",
        "While your own agents run, end your turn.",
        "That turn end is not a stop to report.",
    ]
    .into_iter()
    .filter(|word| !text.contains(word))
    .collect();
    assert!(missing.is_empty(), "{BUILDER_BRIEF} lacks {missing:?}");
}

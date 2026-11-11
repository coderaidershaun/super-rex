//! Fitness tests of `rex init`: what the binary embeds and so installs into
//! every repo, and what the hooks it installs may answer.

use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};
use super_rex::commands::test_output::{OUTPUT_FLAGS, TEST_COMMANDS, hook_output};

/// Each model tier of the grid, with the Claude Code model alias it runs.
const MODEL_TIERS: [(&str, &str); 4] = [
    ("low", "haiku"),
    ("medium", "sonnet"),
    ("high", "opus"),
    ("xhigh", "fable"),
];
const EFFORT_LEVELS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// One agent of the grid: its bundle path, and the header lines that make it
/// run the model and effort its name gives.
struct GridAgent {
    path: PathBuf,
    header_lines: [String; 3],
}

/// Each agent of the grid, one for each model tier and effort level.
fn grid_agents() -> Vec<GridAgent> {
    MODEL_TIERS
        .iter()
        .flat_map(|(tier, model)| {
            EFFORT_LEVELS.iter().map(move |level| {
                let name = format!("model-{tier}-effort-{level}");
                GridAgent {
                    path: PathBuf::from(format!(".claude/agents/{name}.md")),
                    header_lines: [
                        format!("name: {name}"),
                        format!("model: {model}"),
                        format!("effort: {level}"),
                    ],
                }
            })
        })
        .collect()
}

// This test must never be deleted. A file like .claude/settings.local.json
// or the store in .rex can be hidden from `git status` by the user's global
// git ignore rules, so it could end up embedded in the published crate
// without ever showing up as a change. This is the only check that would
// catch it.
#[test]
fn bundle_contains_only_skill_and_agent_paths() {
    let entries = super_rex::bundle::walk();
    assert!(!entries.is_empty(), "embedded bundle must not be empty");

    let markers = [
        ".claude/skills/rex-deliver/SKILL.md",
        ".rex/skills/rex-skill-language/SKILL.md",
    ];
    for marker in markers {
        assert!(
            entries.iter().any(|(path, _)| path == Path::new(marker)),
            "embedded bundle must contain {marker}"
        );
    }

    // An agent whose file runs another model or effort than its name gives
    // is picked by the selection function for a pair it does not run.
    let agents: Vec<PathBuf> = grid_agents()
        .into_iter()
        .map(|agent| {
            let (_, bytes) = entries
                .iter()
                .find(|(path, _)| *path == agent.path)
                .unwrap_or_else(|| {
                    panic!(
                        "embedded bundle must contain every grid agent, missing {}",
                        agent.path.display()
                    )
                });
            let text = String::from_utf8_lossy(bytes);
            let header: Vec<&str> = text
                .lines()
                .skip(1)
                .take_while(|line| *line != "---")
                .collect();
            for line in &agent.header_lines {
                assert!(
                    header.contains(&line.as_str()),
                    "{} must have the header line `{line}`",
                    agent.path.display()
                );
            }
            agent.path
        })
        .collect();

    let skill_roots = [".claude/skills", ".rex/skills"];
    for (path, _) in &entries {
        let skill_root = skill_roots.iter().find(|root| path.starts_with(root));
        assert!(
            skill_root.is_some() || agents.contains(path),
            "every embedded path must start with .claude/skills/ or .rex/skills/, or be a grid agent, found {}",
            path.display()
        );

        let below_root = match skill_root {
            Some(root) => path.strip_prefix(root).expect("path is below its root"),
            None => path.strip_prefix(".claude/agents").expect("agent path"),
        };
        if skill_root.is_some() {
            assert!(
                below_root.components().count() >= 2,
                "every skill file must sit in a skill folder (<root>/<skill>/<file>), found {}",
                path.display()
            );
        }

        let hidden = below_root.components().find(|component| {
            matches!(component, Component::Normal(name) if name.to_string_lossy().starts_with('.'))
        });
        assert!(
            hidden.is_none(),
            "no embedded path may hold a hidden file or folder, found {}",
            path.display()
        );
    }
}

// This test must never be deleted. The test hook runs before every shell
// command in every repo. A permission decision in its answer would skip or
// change the person's permission prompt for that command, and nothing would
// show that it had.
#[test]
fn test_hook_never_decides_a_permission() {
    let forms = [
        "{test}",
        "{test} --quiet",
        "{test} 2>&1",
        "RUST_LOG=debug {test}",
        "{test} | tail -5",
        "cd src && {test}",
        "{test} | grep x",
        "{test}; rm -rf x",
        "{test} && exit 0",
        "{test} > out.txt",
        "echo $({test})",
        "{test} \\",
    ];
    let mut commands: Vec<String> = TEST_COMMANDS
        .iter()
        .flat_map(|test| forms.iter().map(move |form| form.replace("{test}", test)))
        .collect();
    // A run that asks for its output keeps it, so it is never changed.
    for test in TEST_COMMANDS {
        commands.extend(OUTPUT_FLAGS.iter().map(|flag| format!("{test} {flag}")));
    }
    commands.extend(["ls -la", "git status", "rm -rf build", ""].map(String::from));

    let mut changed = 0;
    for command in &commands {
        let tool_input = json!({ "command": command, "description": "d", "timeout": 5, "run_in_background": true });
        let event = json!({ "hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": tool_input });
        let Some(output) = hook_output(&event) else {
            continue;
        };
        changed += 1;
        let answer = &output["hookSpecificOutput"];
        let keys: Vec<&String> = answer.as_object().expect("an object").keys().collect();
        assert_eq!(keys, ["hookEventName", "updatedInput"], "{command}");
        let mut kept = answer["updatedInput"].clone();
        kept["command"] = Value::String(command.clone());
        assert_eq!(kept, tool_input, "every other field stays: {command}");
    }
    // The first five forms of each test command are changed; the rest are not.
    assert_eq!(changed, TEST_COMMANDS.len() * 5);
}

//! Fitness tests of `rex init`: what the binary embeds and so installs into
//! every repo.

use std::path::{Component, Path, PathBuf};

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

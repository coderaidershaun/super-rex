//! Fitness tests of `rex init`: what the binary embeds and so installs into
//! every repo.

use std::path::{Component, Path, PathBuf};

const MODEL_TIERS: [&str; 4] = ["low", "medium", "high", "xhigh"];
const EFFORT_LEVELS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// The bundle path of each agent in the grid, one for each model tier and
/// effort level.
fn grid_agent_paths() -> Vec<PathBuf> {
    MODEL_TIERS
        .iter()
        .flat_map(|tier| {
            EFFORT_LEVELS.iter().map(move |level| {
                PathBuf::from(format!(".claude/agents/model-{tier}-effort-{level}.md"))
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

    let agents = grid_agent_paths();
    for agent in &agents {
        assert!(
            entries.iter().any(|(path, _)| path == agent),
            "embedded bundle must contain every grid agent, missing {}",
            agent.display()
        );
    }

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

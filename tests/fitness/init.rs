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
            EFFORT_LEVELS
                .iter()
                .map(move |level| PathBuf::from(format!("agents/model-{tier}-effort-{level}.md")))
        })
        .collect()
}

// This test must never be deleted. A file like .claude/settings.local.json
// can be hidden from `git status` by the user's global git ignore rules,
// so it could end up embedded in the published crate without ever
// showing up as a change. This is the only check that would catch it.
#[test]
fn bundle_contains_only_skill_and_agent_paths() {
    let entries = super_rex::bundle::walk();
    assert!(!entries.is_empty(), "embedded bundle must not be empty");

    let has_marker = entries
        .iter()
        .any(|(path, _)| path == Path::new("skills/rex-skill-language/SKILL.md"));
    assert!(
        has_marker,
        "embedded bundle must contain skills/rex-skill-language/SKILL.md"
    );

    let agents = grid_agent_paths();
    for (path, _) in &entries {
        assert!(
            path.starts_with("skills") || agents.contains(path),
            "every embedded path must start with skills/ or be a grid agent, found {}",
            path.display()
        );

        let hidden = path.components().find(|component| {
            matches!(component, Component::Normal(name) if name.to_string_lossy().starts_with('.'))
        });
        assert!(
            hidden.is_none(),
            "no embedded path may hold a hidden file or folder, found {}",
            path.display()
        );
    }
}

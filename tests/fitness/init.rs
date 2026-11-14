//! Fitness tests of what the plugin ships and what `rex init` writes: the files
//! the binary embeds, and the settings hooks no run may leave behind.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};

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
                    path: PathBuf::from(format!("plugin/agents/{name}.md")),
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

// This test must never be deleted. A file like the types folder the engine
// writes below plugin/.claude-plugin can be hidden from `git status` by the user's global
// git ignore rules, so it could end up embedded in the published crate
// without ever showing up as a change. This is the only check that would
// catch it.
#[test]
fn bundle_contains_only_skill_and_agent_paths() {
    let entries = super_rex::bundle::walk();
    assert!(!entries.is_empty(), "embedded bundle must not be empty");

    let markers = [
        "plugin/skills/rex-deliver/SKILL.md",
        "plugin/library/rex-skill-language/SKILL.md",
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

    let skill_roots = ["plugin/skills", "plugin/library"];
    for (path, _) in &entries {
        let skill_root = skill_roots.iter().find(|root| path.starts_with(root));
        assert!(
            skill_root.is_some() || agents.contains(path),
            "every embedded path must start with plugin/skills/ or plugin/library/, or be a grid agent, found {}",
            path.display()
        );

        let below_root = match skill_root {
            Some(root) => path.strip_prefix(root).expect("path is below its root"),
            None => path.strip_prefix("plugin/agents").expect("agent path"),
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

/// Runs `rex init` in a new repo whose settings file holds `settings`, with
/// stand-in tools on PATH, and gives the settings file after it.
fn settings_after_init(settings: Option<&Value>) -> Value {
    let repo = tempfile::tempdir().expect("create repo");
    let bin = tempfile::tempdir().expect("create tools folder");
    fs::create_dir(repo.path().join(".git")).expect("create .git");
    if let Some(settings) = settings {
        fs::create_dir(repo.path().join(".claude")).expect("create .claude");
        fs::write(
            repo.path().join(".claude/settings.local.json"),
            settings.to_string(),
        )
        .expect("write settings");
    }
    for (tool, body) in [
        (
            "claude",
            "if [ \"$1\" = plugin ]; then exit 0; fi\necho '2.1.323 (Claude Code)'",
        ),
        ("bwrap", "exit 0"),
        ("socat", "exit 0"),
    ] {
        let path = bin.path().join(tool);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write tool");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        }
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rex"))
        .arg("init")
        .current_dir(repo.path())
        .env("PATH", bin.path())
        .env_remove("REX_STORE")
        .output()
        .expect("run rex init");
    assert!(
        output.status.success(),
        "rex init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text =
        fs::read_to_string(repo.path().join(".claude/settings.local.json")).expect("read settings");
    serde_json::from_str(&text).expect("settings JSON")
}

/// The command of each hook of any event in `settings`.
fn hook_commands(settings: &Value) -> Vec<String> {
    let Some(events) = settings.get("hooks").and_then(Value::as_object) else {
        return Vec::new();
    };
    events
        .values()
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|group| group.get("hooks").and_then(Value::as_array))
        .flatten()
        .filter_map(|hook| hook.get("command").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

// This test must never be deleted. A settings hook runs on every event of
// every session in the repo, outside the plugin's own failure handling, so a
// hook that fails or prints can stop an agent. The plugin's mod does that work
// now, so no run of `rex init` may leave a hook that runs rex.
#[test]
fn init_registers_no_hook() {
    let old_entry = |command: &str| json!([{ "hooks": [{ "type": "command", "command": command, "timeout": 10 }] }]);
    let starting_files = [
        None,
        Some(json!({})),
        Some(json!({ "hooks": {} })),
        Some(json!({ "hooks": {
            "SubagentStart": old_entry("rex hook >/dev/null 2>&1 || true"),
            "SubagentStop": old_entry("rex hook >/dev/null 2>&1 || true"),
            "PostToolUse": old_entry("rex hook >/dev/null 2>&1 || true"),
            "PreToolUse": [
                { "matcher": "Agent|Task", "hooks": [{ "type": "command", "command": "rex gate 2>/dev/null || true" }] },
                { "matcher": "Bash", "hooks": [
                    { "type": "command", "command": "rex test-hook 2>/dev/null || true" },
                    { "type": "command", "command": "my-linter" }
                ] }
            ]
        } })),
    ];
    for settings in &starting_files {
        let after = settings_after_init(settings.as_ref());

        let commands = hook_commands(&after);
        assert!(
            commands.iter().all(|command| !command.starts_with("rex ")),
            "a hook runs rex after rex init on {settings:?}: {commands:?}"
        );
        let kept_own = settings
            .as_ref()
            .is_some_and(|settings| settings.to_string().contains("my-linter"));
        assert_eq!(
            commands.contains(&"my-linter".to_owned()),
            kept_own,
            "the person's hook stays"
        );
    }
}

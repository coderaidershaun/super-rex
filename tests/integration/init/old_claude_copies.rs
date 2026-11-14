//! The skills and grid agents that rex 0.4 to 0.7 copied into `.claude/skills/`
//! and `.claude/agents/`: `rex init` removes a copy the person never changed,
//! so the session no longer lists an old skill beside the plugin's own.

use std::fs;
use std::path::{Path, PathBuf};

use super::support::{Repo, repo, rex_init_ok};

/// Files exactly as rex 0.7.0 wrote them, from `git show 7deda22:.claude/…`.
fn fixture(part: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/old-copies")
        .join(part)
}

fn install_old_skill(repo: &Repo, skill: &str) -> PathBuf {
    let dest = repo.path().join(".claude/skills").join(skill);
    fs::create_dir_all(&dest).expect("create skill folder");
    fs::copy(
        fixture("claude-skills").join(skill).join("SKILL.md"),
        dest.join("SKILL.md"),
    )
    .expect("copy skill");
    dest
}

fn install_old_agent(repo: &Repo, agent: &str) -> PathBuf {
    let folder = repo.path().join(".claude/agents");
    fs::create_dir_all(&folder).expect("create agents folder");
    let dest = folder.join(agent);
    fs::copy(fixture("claude-agents").join(agent), &dest).expect("copy agent");
    dest
}

#[test]
fn unchanged_skills_and_agents_are_removed_with_one_notice_each() {
    let repo = repo();
    let grill = install_old_skill(&repo, "rex-grill-me");
    let rules = install_old_skill(&repo, "rex-rules");
    let low = install_old_agent(&repo, "model-low-effort-low.md");
    let high = install_old_agent(&repo, "model-high-effort-high.md");

    let printed = rex_init_ok(&repo, &[]);

    for gone in [&grill, &rules, &low, &high] {
        assert!(!gone.exists(), "{} is still there", gone.display());
    }
    for line in [
        "rex init: removed .claude/skills/rex-grill-me/: it was an unchanged copy from an older rex, and the plugin ships rex-grill-me now\n",
        "rex init: removed .claude/skills/rex-rules/: it was an unchanged copy from an older rex, and the plugin ships rex-rules now\n",
        "rex init: removed .claude/agents/model-high-effort-high.md: it was an unchanged copy from an older rex, and the plugin ships model-high-effort-high.md now\n",
        "rex init: removed .claude/agents/model-low-effort-low.md: it was an unchanged copy from an older rex, and the plugin ships model-low-effort-low.md now\n",
    ] {
        assert_eq!(
            printed.stdout.matches(line).count(),
            1,
            "got: {}",
            printed.stdout
        );
    }
    assert_eq!(printed.stderr, "");
}

#[test]
fn changed_copies_are_kept_with_a_warning_each() {
    let repo = repo();
    let skill = install_old_skill(&repo, "rex-grill-me");
    let mut text = fs::read_to_string(skill.join("SKILL.md")).expect("read skill");
    text.push_str("\nAsk about the budget too.\n");
    fs::write(skill.join("SKILL.md"), &text).expect("change skill");
    let agent = install_old_agent(&repo, "model-low-effort-low.md");
    fs::write(
        &agent,
        "---\nname: model-low-effort-low\nmodel: haiku\n---\nOurs.\n",
    )
    .expect("change agent");

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        fs::read_to_string(skill.join("SKILL.md")).expect("read"),
        text
    );
    assert!(agent.exists());
    assert!(
        !printed.stdout.contains("removed"),
        "got: {}",
        printed.stdout
    );
    assert_eq!(
        printed.stderr,
        "rex init: warning: kept .claude/skills/rex-grill-me/: it differs from the copy an older rex installed, so the session lists it beside the rex-grill-me skill that the plugin ships; to use the plugin's, move your changes out and delete that folder\n\
         rex init: warning: kept .claude/agents/model-low-effort-low.md: it differs from the copy an older rex installed, so the session lists it beside the model-low-effort-low.md agent that the plugin ships; to use the plugin's, move your changes out and delete that file\n"
    );
}

#[test]
fn skills_and_agents_of_the_person_are_kept_with_no_notice() {
    let repo = repo();
    let skills = repo.path().join(".claude/skills/shop-release");
    fs::create_dir_all(&skills).expect("create skill");
    fs::write(
        skills.join("SKILL.md"),
        "---\nname: shop-release\n---\nOurs.\n",
    )
    .expect("write");
    let agents = repo.path().join(".claude/agents");
    fs::create_dir_all(&agents).expect("create agents");
    fs::write(
        agents.join("reviewer.md"),
        "---\nname: reviewer\n---\nOurs.\n",
    )
    .expect("write");

    let printed = rex_init_ok(&repo, &[]);

    assert!(skills.join("SKILL.md").exists());
    assert!(agents.join("reviewer.md").exists());
    assert!(
        !printed.stdout.contains("removed"),
        "got: {}",
        printed.stdout
    );
    assert_eq!(printed.stderr, "");
}

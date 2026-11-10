//! Contract tests of what `rex oms` gives to `rex hook` and what `rex skills`
//! gives to the store.

use std::fs;
use std::path::Path;

use super_rex::commands::skills::describe;
use super_rex::oms::{Effort, GridAgent, ParseGridAgentError, Tier};

/// `rex hook` reads grid agent names back from what Claude Code reports, so
/// each name must give back the pair it was written from.
#[test]
fn grid_agent_names_round_trip_and_bad_names_are_typed() {
    let tiers = [
        (Tier::Low, "low"),
        (Tier::Medium, "medium"),
        (Tier::High, "high"),
        (Tier::Xhigh, "xhigh"),
    ];
    let efforts = [
        (Effort::Low, "low"),
        (Effort::Medium, "medium"),
        (Effort::High, "high"),
        (Effort::Xhigh, "xhigh"),
        (Effort::Max, "max"),
    ];
    for (tier, tier_name) in tiers {
        for (effort, effort_name) in efforts {
            let agent = GridAgent { tier, effort };
            let name = format!("model-{tier_name}-effort-{effort_name}");

            assert_eq!(agent.to_string(), name);
            assert_eq!(name.parse::<GridAgent>(), Ok(agent), "{name}");
        }
    }

    assert!(matches!(
        "model-high".parse::<GridAgent>(),
        Err(ParseGridAgentError::Shape(name)) if name == "model-high"
    ));
    assert!(matches!(
        "model-huge-effort-low".parse::<GridAgent>(),
        Err(ParseGridAgentError::Tier(name)) if name == "huge"
    ));
    assert!(matches!(
        "model-high-effort-extreme".parse::<GridAgent>(),
        Err(ParseGridAgentError::Effort(name)) if name == "extreme"
    ));
}

/// The store indexes an orchestrator through this function (C1 7.4), so it
/// must fit the store's `describe` argument and give the one-line text.
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

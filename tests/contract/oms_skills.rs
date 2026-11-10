//! Contract tests of `rex oms` and the `rex skills` commands.

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

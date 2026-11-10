//! Fitness tests of `rex oms` and the `rex skills` commands.
//!
//! The selection rules must hold for every one of the 16,384 combinations of
//! inputs. A wrong agent fails silently: the job still runs, on the wrong
//! model, so only these tests notice.

use sha2::{Digest, Sha256};
use super_rex::oms::{
    Effort, GridAgent, Inputs, Level, Overridden, Rule, Selection, Tier, VERSION, select,
};

const RATINGS: [Option<Level>; 4] = [
    None,
    Some(Level::Low),
    Some(Level::Medium),
    Some(Level::High),
];

/// Every combination of inputs, each rating in the order `None`, Low, Medium,
/// High, with duration changing slowest and verification fastest.
fn all_inputs() -> Vec<Inputs> {
    let mut all = Vec::with_capacity(RATINGS.len().pow(7));
    for duration in RATINGS {
        for budget in RATINGS {
            for intelligence in RATINGS {
                for autonomy in RATINGS {
                    for reliability in RATINGS {
                        for consequence in RATINGS {
                            for verification in RATINGS {
                                all.push(Inputs {
                                    duration,
                                    budget,
                                    intelligence,
                                    autonomy,
                                    reliability,
                                    consequence,
                                    verification,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    all
}

/// Names one rating of `Inputs`, so that a test can step it.
type Field = fn(&mut Inputs) -> &mut Option<Level>;

const RAISING_FIELDS: [(&str, Field); 6] = [
    ("intelligence", |inputs| &mut inputs.intelligence),
    ("reliability", |inputs| &mut inputs.reliability),
    ("consequence", |inputs| &mut inputs.consequence),
    ("autonomy", |inputs| &mut inputs.autonomy),
    ("budget", |inputs| &mut inputs.budget),
    ("duration", |inputs| &mut inputs.duration),
];

/// The inputs with one rating moved one step up, or `None` when that rating
/// is High or `None` and so has no step up.
fn one_step_up(inputs: Inputs, field: Field) -> Option<Inputs> {
    let mut raised = inputs;
    let rating = field(&mut raised);
    *rating = match *rating {
        Some(Level::Low) => Some(Level::Medium),
        Some(Level::Medium) => Some(Level::High),
        Some(Level::High) | None => return None,
    };
    Some(raised)
}

fn is_weaker(agent: GridAgent, than: GridAgent) -> bool {
    agent.tier < than.tier || agent.effort < than.effort
}

fn has_high_stakes(inputs: Inputs) -> bool {
    inputs.reliability == Some(Level::High) || inputs.consequence == Some(Level::High)
}

fn has_cap_rule(selection: &Selection) -> bool {
    selection.rules.iter().any(|rule| {
        matches!(
            rule,
            Rule::CapBudgetTier | Rule::CapBudgetEffort | Rule::CapDurationEffort
        )
    })
}

fn joined<T>(items: &[T], name: fn(&T) -> &'static str) -> String {
    let names: Vec<&str> = items.iter().map(name).collect();
    names.join(",")
}

#[test]
fn every_combination_returns_a_supported_agent() {
    let tiers = [Tier::Low, Tier::Medium, Tier::High, Tier::Xhigh];
    let efforts = [
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::Xhigh,
        Effort::Max,
    ];

    for inputs in all_inputs() {
        let agent = select(inputs).agent;
        let read_back = agent.to_string().parse::<GridAgent>();

        // Version 1 supports all twenty pairs, so every pair of a known tier
        // and a known effort whose name reads back is in the supported table.
        assert!(
            tiers.contains(&agent.tier) && efforts.contains(&agent.effort),
            "total: {} gave {agent:?}",
            inputs.code()
        );
        assert_eq!(read_back, Ok(agent), "total: {}", inputs.code());
    }
}

#[test]
fn raising_an_input_never_weakens_the_agent() {
    for inputs in all_inputs() {
        let agent = select(inputs).agent;
        for (name, field) in RAISING_FIELDS {
            let Some(raised) = one_step_up(inputs, field) else {
                continue;
            };
            let raised_agent = select(raised).agent;
            assert!(
                !is_weaker(raised_agent, agent),
                "raising never weakens: {name} up from {} to {} gave {raised_agent} after {agent}",
                inputs.code(),
                raised.code()
            );
        }
    }
}

#[test]
fn easier_checking_never_strengthens_the_agent() {
    for inputs in all_inputs() {
        let agent = select(inputs).agent;
        let Some(easier) = one_step_up(inputs, |inputs| &mut inputs.verification) else {
            continue;
        };
        let easier_agent = select(easier).agent;
        assert!(
            !is_weaker(agent, easier_agent),
            "easier checking never strengthens: {} to {} gave {easier_agent} after {agent}",
            inputs.code(),
            easier.code()
        );
    }
}

#[test]
fn caps_only_lower() {
    let no_cap = [None, Some(Level::High)];
    for inputs in all_inputs() {
        let selection = select(inputs);
        let (agent, demand) = (selection.agent, selection.demand);
        let has_no_limits = no_cap.contains(&inputs.budget) && no_cap.contains(&inputs.duration);

        assert!(
            agent.tier <= demand.tier && agent.effort <= demand.effort,
            "caps only lower: {} gave {agent} above its demand {demand}",
            inputs.code()
        );
        if !has_cap_rule(&selection) || has_no_limits {
            assert_eq!(agent, demand, "caps only lower: {}", inputs.code());
        }
    }
}

#[test]
fn the_floor_holds() {
    for inputs in all_inputs()
        .into_iter()
        .filter(|inputs| has_high_stakes(*inputs))
    {
        let agent = select(inputs).agent;
        assert!(
            agent.tier >= Tier::Medium && agent.effort >= Effort::High,
            "the floor holds: {} gave {agent}",
            inputs.code()
        );
    }
}

#[test]
fn the_budget_holds() {
    let medium_medium = GridAgent {
        tier: Tier::Medium,
        effort: Effort::Medium,
    };
    let medium_high = GridAgent {
        tier: Tier::Medium,
        effort: Effort::High,
    };
    for inputs in all_inputs() {
        let agent = select(inputs).agent;
        let code = inputs.code();
        match (inputs.budget, has_high_stakes(inputs)) {
            (Some(Level::Low), false) => assert!(
                agent.tier <= Tier::Medium && agent.effort <= Effort::Medium,
                "the budget holds: {code} gave {agent}, above {medium_medium}"
            ),
            (Some(Level::Low), true) => {
                assert_eq!(agent, medium_high, "the budget holds: {code}");
            }
            (Some(Level::Medium), _) => assert!(
                agent.tier <= Tier::High,
                "the budget holds: {code} gave {agent}, above the high tier"
            ),
            _ => {}
        }
    }
}

#[test]
fn conflict_is_reported_only_when_the_floor_overrode_a_cap() {
    for inputs in all_inputs() {
        let selection = select(inputs);
        if selection.conflict.is_empty() {
            continue;
        }
        assert!(
            has_high_stakes(inputs) && selection.rules.contains(&Rule::FloorStakes),
            "conflict is honest: {} reported {:?} with rules {:?}",
            inputs.code(),
            selection.conflict,
            selection.rules
        );
    }
}

#[test]
fn reliability_and_consequence_are_symmetric() {
    for inputs in all_inputs() {
        let swapped = Inputs {
            reliability: inputs.consequence,
            consequence: inputs.reliability,
            ..inputs
        };
        let selection = select(inputs);
        let swapped_selection = select(swapped);
        assert_eq!(
            (
                selection.agent,
                selection.demand,
                selection.rules,
                selection.conflict
            ),
            (
                swapped_selection.agent,
                swapped_selection.demand,
                swapped_selection.rules,
                swapped_selection.conflict
            ),
            "symmetry: {} against {}",
            inputs.code(),
            swapped.code()
        );
    }
}

#[test]
fn none_matches_its_neutral_level() {
    let neutral_levels: [(&str, Field, Level); 5] = [
        (
            "intelligence",
            |inputs| &mut inputs.intelligence,
            Level::Medium,
        ),
        (
            "verification",
            |inputs| &mut inputs.verification,
            Level::Medium,
        ),
        ("autonomy", |inputs| &mut inputs.autonomy, Level::Medium),
        ("budget", |inputs| &mut inputs.budget, Level::High),
        ("duration", |inputs| &mut inputs.duration, Level::High),
    ];
    for inputs in all_inputs() {
        for (name, field, neutral) in neutral_levels {
            let mut filled = inputs;
            let rating = field(&mut filled);
            if rating.is_some() {
                continue;
            }
            *rating = Some(neutral);
            assert_eq!(
                select(inputs).agent,
                select(filled).agent,
                "none matches {neutral:?} for {name}: {} against {}",
                inputs.code(),
                filled.code()
            );
        }
    }
}

#[test]
fn the_extremes_are_guarded() {
    for inputs in all_inputs() {
        let selection = select(inputs);
        let agent = selection.agent;
        let code = inputs.code();
        let has_binding_effort_cap = selection
            .rules
            .iter()
            .any(|rule| matches!(rule, Rule::CapBudgetEffort | Rule::CapDurationEffort));

        if agent.effort == Effort::Max {
            assert!(
                has_high_stakes(inputs)
                    && inputs.verification == Some(Level::Low)
                    && inputs.autonomy == Some(Level::High)
                    && !has_binding_effort_cap,
                "max effort is guarded: {code} gave {agent}"
            );
        }
        if agent.tier == Tier::Xhigh {
            assert!(
                inputs.intelligence == Some(Level::High) && has_high_stakes(inputs),
                "the top tier is guarded: {code} gave {agent}"
            );
        }
        if agent.tier == Tier::Low {
            assert!(
                inputs.intelligence == Some(Level::Low) && !has_high_stakes(inputs),
                "the lowest tier is guarded: {code} gave {agent}"
            );
        }
    }
}

// Warning: never delete this test. It pins the whole function to its version
// number, so a change in behaviour without a version bump fails here.
#[test]
fn the_checksum_pins_the_function_at_its_version() {
    let lines: Vec<String> = all_inputs()
        .into_iter()
        .map(|inputs| {
            let selection = select(inputs);
            let conflict = match joined(&selection.conflict, |over| Overridden::name(*over)) {
                none if none.is_empty() => "none".to_owned(),
                names => names,
            };
            format!(
                "{} {} {} {} {conflict}",
                inputs.code(),
                selection.agent,
                selection.demand,
                joined(&selection.rules, |rule| Rule::id(*rule))
            )
        })
        .collect();
    let text = lines.join("\n");
    let digest = Sha256::digest(text.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();

    assert_eq!(
        lines[0],
        "------- model-medium-effort-medium model-medium-effort-medium T0,E0 none"
    );
    assert_eq!(
        (VERSION, hex.as_str()),
        (
            1,
            "baf26ba9c8d654b46318d8b6cc2e18b639e3bed245eafa9f0cd30f11e47a6c99"
        )
    );
}

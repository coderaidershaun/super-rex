//! Fitness tests of `rex oms`.
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

/// One pass over every input. Each failure names the invariant and the input.
#[test]
fn selection_invariants_hold_for_every_combination() {
    let tiers = [Tier::Low, Tier::Medium, Tier::High, Tier::Xhigh];
    let efforts = [
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::Xhigh,
        Effort::Max,
    ];
    let medium_medium = GridAgent {
        tier: Tier::Medium,
        effort: Effort::Medium,
    };
    let medium_high = GridAgent {
        tier: Tier::Medium,
        effort: Effort::High,
    };
    let no_cap = [None, Some(Level::High)];
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
        let selection = select(inputs);
        let (agent, demand) = (selection.agent, selection.demand);
        let code = inputs.code();
        let high_stakes = has_high_stakes(inputs);

        // Version 2 supports all twenty pairs, so every pair of a known tier
        // and a known effort whose name reads back is in the supported table.
        assert!(
            tiers.contains(&agent.tier) && efforts.contains(&agent.effort),
            "total: {code} gave {agent:?}"
        );
        assert_eq!(
            agent.to_string().parse::<GridAgent>(),
            Ok(agent),
            "total: {code}"
        );

        for (name, field) in RAISING_FIELDS {
            let Some(raised) = one_step_up(inputs, field) else {
                continue;
            };
            let raised_agent = select(raised).agent;
            assert!(
                !is_weaker(raised_agent, agent),
                "raising never weakens: {name} up from {code} to {} gave {raised_agent} after {agent}",
                raised.code()
            );
        }

        if let Some(easier) = one_step_up(inputs, |inputs| &mut inputs.verification) {
            let easier_agent = select(easier).agent;
            assert!(
                !is_weaker(agent, easier_agent),
                "easier checking never strengthens: {code} to {} gave {easier_agent} after {agent}",
                easier.code()
            );
        }

        let has_no_limits = no_cap.contains(&inputs.budget) && no_cap.contains(&inputs.duration);
        assert!(
            agent.tier <= demand.tier && agent.effort <= demand.effort,
            "caps only lower: {code} gave {agent} above its demand {demand}"
        );
        if !has_cap_rule(&selection) || has_no_limits {
            assert_eq!(agent, demand, "caps only lower: {code}");
        }

        if high_stakes {
            assert!(
                agent.tier >= Tier::Medium && agent.effort >= Effort::High,
                "the floor holds: {code} gave {agent}"
            );
        }

        match (inputs.budget, high_stakes) {
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

        if !selection.conflict.is_empty() {
            assert!(
                high_stakes && selection.rules.contains(&Rule::FloorStakes),
                "conflict is honest: {code} reported {:?} with rules {:?}",
                selection.conflict,
                selection.rules
            );
        }

        // Reliability and consequence weigh the same, except that only
        // consequence opens the top tier. So the swap must give the same
        // selection unless one side fired the rule that closes the top tier.
        // Then only the tier between `high` and `xhigh`, and with it the
        // budget cap on the tier, may differ.
        let swapped = Inputs {
            reliability: inputs.consequence,
            consequence: inputs.reliability,
            ..inputs
        };
        let swapped_selection = select(swapped);
        let fired_top_tier_rule =
            |selection: &Selection| selection.rules.contains(&Rule::TierTopNeedsConsequence);
        if fired_top_tier_rule(&selection) || fired_top_tier_rule(&swapped_selection) {
            let without_top_tier = |selection: &Selection| {
                let below_top = |agent: GridAgent| GridAgent {
                    tier: agent.tier.min(Tier::High),
                    ..agent
                };
                let rules: Vec<Rule> = selection
                    .rules
                    .iter()
                    .copied()
                    .filter(|rule| {
                        !matches!(rule, Rule::TierTopNeedsConsequence | Rule::CapBudgetTier)
                    })
                    .collect();
                (
                    below_top(selection.agent),
                    below_top(selection.demand),
                    rules,
                    selection.conflict.clone(),
                )
            };
            assert_eq!(
                without_top_tier(&selection),
                without_top_tier(&swapped_selection),
                "symmetry below the top tier: {code} against {}",
                swapped.code()
            );
            let (with_rule, other) = if fired_top_tier_rule(&selection) {
                (&selection, &swapped_selection)
            } else {
                (&swapped_selection, &selection)
            };
            assert!(
                !fired_top_tier_rule(other)
                    && with_rule.demand.tier == Tier::High
                    && other.demand.tier == Tier::Xhigh,
                "the top tier rule fires on one side only and moves the demand from xhigh to high: {code} against {}",
                swapped.code()
            );
        } else {
            assert_eq!(
                (
                    &selection.agent,
                    &selection.demand,
                    &selection.rules,
                    &selection.conflict
                ),
                (
                    &swapped_selection.agent,
                    &swapped_selection.demand,
                    &swapped_selection.rules,
                    &swapped_selection.conflict
                ),
                "symmetry: {code} against {}",
                swapped.code()
            );
        }

        for (name, field, neutral) in neutral_levels {
            let mut filled = inputs;
            let rating = field(&mut filled);
            if rating.is_some() {
                continue;
            }
            *rating = Some(neutral);
            assert_eq!(
                agent,
                select(filled).agent,
                "none matches {neutral:?} for {name}: {code} against {}",
                filled.code()
            );
        }

        let has_binding_effort_cap = selection
            .rules
            .iter()
            .any(|rule| matches!(rule, Rule::CapBudgetEffort | Rule::CapDurationEffort));
        if agent.effort == Effort::Max {
            assert!(
                high_stakes
                    && inputs.verification == Some(Level::Low)
                    && inputs.autonomy == Some(Level::High)
                    && !has_binding_effort_cap,
                "max effort is guarded: {code} gave {agent}"
            );
        }
        if agent.tier == Tier::Xhigh || demand.tier == Tier::Xhigh {
            assert!(
                inputs.intelligence == Some(Level::High) && inputs.consequence == Some(Level::High),
                "the top tier needs high consequence: {code} gave {agent}, demand {demand}"
            );
        }
        if agent.tier == Tier::Low {
            assert!(
                inputs.intelligence == Some(Level::Low) && !high_stakes,
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
            2,
            "783662252c3f2bc38e0799d903e403141869d27a366a413eeab49c53ca5f0f17"
        )
    );
}

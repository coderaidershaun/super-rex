//! The selection function: six steps from a role's seven ratings to one grid
//! agent. Each step is one function, so each rule fires in one place.

use std::collections::BTreeSet;

use super::{Effort, GridAgent, Inputs, Level, Overridden, Rule, Selection, Tier};

/// One value for each level of an input.
struct ByLevel<T> {
    low: T,
    medium: T,
    high: T,
}

impl<T: Copy> ByLevel<T> {
    fn at(&self, level: Level) -> T {
        match level {
            Level::Low => self.low,
            Level::Medium => self.medium,
            Level::High => self.high,
        }
    }
}

/// The upper limits that a budget puts on the pair.
#[derive(Clone, Copy)]
struct BudgetCap {
    tier: Option<Tier>,
    effort: Option<Effort>,
}

const NO_BUDGET_CAP: BudgetCap = BudgetCap {
    tier: None,
    effort: None,
};

/// Effort levels each tier accepts, indexed by tier position minus one.
type SupportedEfforts = [&'static [Effort]; 4];

// Warning: the ten constants below are the whole mapping. A change to any of
// them changes some selection, so it needs a new VERSION and a new checksum.

const TIER_WHEN_INTELLIGENCE_IS_NONE: Tier = Tier::Medium;

const EFFORT_WHEN_STAKES_ARE_NONE: Effort = Effort::Medium;

const TIER_LIFT_FOR_HIGH_STAKES: i8 = 1;

/// The most expensive tier is kept for roles whose mistakes a revert does not
/// undo, so without high consequence the tier stops one step below it.
const TIER_CAP_WITHOUT_HIGH_CONSEQUENCE: Tier = Tier::High;

const EFFORT_SHIFT_FOR_VERIFICATION: ByLevel<i8> = ByLevel {
    low: 1,
    medium: 0,
    high: -1,
};

const EFFORT_SHIFT_FOR_AUTONOMY: ByLevel<i8> = ByLevel {
    low: -1,
    medium: 0,
    high: 1,
};

const BUDGET_CAPS: ByLevel<BudgetCap> = ByLevel {
    low: BudgetCap {
        tier: Some(Tier::Medium),
        effort: Some(Effort::Medium),
    },
    medium: BudgetCap {
        tier: Some(Tier::High),
        effort: Some(Effort::High),
    },
    high: NO_BUDGET_CAP,
};

/// Duration limits effort only, because effort is the documented speed
/// control and nothing documents how the tier affects speed.
const DURATION_EFFORT_CAPS: ByLevel<Option<Effort>> = ByLevel {
    low: Some(Effort::Medium),
    medium: Some(Effort::High),
    high: None,
};

const EFFORT_FLOOR_FOR_HIGH_STAKES: Effort = Effort::High;

/// From the Claude Code model configuration page, read on 2026-11-10: every
/// tier's model accepts all five levels. Claude Code quietly runs a lower
/// level when asked for one that a model does not accept.
const SUPPORTED_EFFORTS: SupportedEfforts = [&Effort::ALL; 4];

/// Pick the grid agent for one role. Every combination of inputs has an
/// answer, so this cannot fail.
#[must_use]
pub fn select(inputs: Inputs) -> Selection {
    select_with(inputs, &SUPPORTED_EFFORTS)
}

fn select_with(inputs: Inputs, supported: &SupportedEfforts) -> Selection {
    let mut fired = BTreeSet::new();
    let stakes = stakes(inputs);
    let demand = GridAgent {
        tier: tier_demand(inputs, stakes, &mut fired),
        effort: effort_demand(inputs, stakes, &mut fired),
    };

    let budget_cap = inputs
        .budget
        .map_or(NO_BUDGET_CAP, |level| BUDGET_CAPS.at(level));
    let duration_cap = inputs
        .duration
        .and_then(|level| DURATION_EFFORT_CAPS.at(level));

    let capped = apply_caps(demand, budget_cap, duration_cap, &mut fired);
    let floored = GridAgent {
        effort: apply_floor(capped.effort, stakes, &mut fired),
        ..capped
    };
    let agent = snap_to_supported(floored, supported, &mut fired);

    Selection {
        agent,
        demand,
        rules: fired.into_iter().collect(),
        conflict: conflict(agent.effort, budget_cap.effort, duration_cap),
        inputs,
    }
}

/// The higher of reliability and consequence. `None` sorts below every
/// level, so one `None` rating is ignored, and two `None` ratings give `None`.
fn stakes(inputs: Inputs) -> Option<Level> {
    inputs.reliability.max(inputs.consequence)
}

fn tier_demand(inputs: Inputs, stakes: Option<Level>, fired: &mut BTreeSet<Rule>) -> Tier {
    let start = match inputs.intelligence {
        None => {
            fired.insert(Rule::TierDefault);
            TIER_WHEN_INTELLIGENCE_IS_NONE
        }
        Some(level) => {
            fired.insert(Rule::TierIntelligence);
            Tier::at_position(level.position())
        }
    };
    if stakes != Some(Level::High) {
        return start;
    }
    fired.insert(Rule::TierStakesLift);
    let lifted = Tier::at_position(start.position() + TIER_LIFT_FOR_HIGH_STAKES);
    if inputs.consequence != Some(Level::High) && lifted > TIER_CAP_WITHOUT_HIGH_CONSEQUENCE {
        fired.insert(Rule::TierTopNeedsConsequence);
        return TIER_CAP_WITHOUT_HIGH_CONSEQUENCE;
    }
    lifted
}

fn effort_demand(inputs: Inputs, stakes: Option<Level>, fired: &mut BTreeSet<Rule>) -> Effort {
    let mut position = match stakes {
        None => {
            fired.insert(Rule::EffortDefault);
            EFFORT_WHEN_STAKES_ARE_NONE.position()
        }
        Some(level) => {
            fired.insert(Rule::EffortStakes);
            level.position()
        }
    };
    if let Some(verification) = inputs.verification {
        match verification {
            Level::Low => {
                fired.insert(Rule::EffortHardToVerify);
            }
            Level::Medium => {}
            Level::High => {
                fired.insert(Rule::EffortEasyToVerify);
            }
        }
        position += EFFORT_SHIFT_FOR_VERIFICATION.at(verification);
    }
    if let Some(autonomy) = inputs.autonomy {
        match autonomy {
            Level::Low => {
                fired.insert(Rule::EffortAutonomyLow);
            }
            Level::Medium => {}
            Level::High => {
                fired.insert(Rule::EffortAutonomyHigh);
            }
        }
        position += EFFORT_SHIFT_FOR_AUTONOMY.at(autonomy);
    }
    let in_range = position.clamp(Effort::Low.position(), Effort::Max.position());
    apply_floor(Effort::at_position(in_range), stakes, fired)
}

/// Runs twice: once on the demand, and again after the caps, because the
/// floor beats the caps.
fn apply_floor(effort: Effort, stakes: Option<Level>, fired: &mut BTreeSet<Rule>) -> Effort {
    if stakes == Some(Level::High) && effort < EFFORT_FLOOR_FOR_HIGH_STAKES {
        fired.insert(Rule::FloorStakes);
        return EFFORT_FLOOR_FOR_HIGH_STAKES;
    }
    effort
}

fn apply_caps(
    demand: GridAgent,
    budget: BudgetCap,
    duration_effort: Option<Effort>,
    fired: &mut BTreeSet<Rule>,
) -> GridAgent {
    if budget.tier.is_some_and(|cap| cap < demand.tier) {
        fired.insert(Rule::CapBudgetTier);
    }
    if budget.effort.is_some_and(|cap| cap < demand.effort) {
        fired.insert(Rule::CapBudgetEffort);
    }
    if duration_effort.is_some_and(|cap| cap < demand.effort) {
        fired.insert(Rule::CapDurationEffort);
    }
    let tier = budget.tier.map_or(demand.tier, |cap| cap.min(demand.tier));
    let effort = [budget.effort, duration_effort]
        .into_iter()
        .flatten()
        .fold(demand.effort, Effort::min);
    GridAgent { tier, effort }
}

/// Moves an effort the tier does not accept to the highest accepted level at
/// or below it, else the lowest above it, as Claude Code itself does. The
/// tier never changes, because it was chosen for capability.
fn snap_to_supported(
    pair: GridAgent,
    supported: &SupportedEfforts,
    fired: &mut BTreeSet<Rule>,
) -> GridAgent {
    let accepted = supported[pair.tier as usize - 1];
    if accepted.contains(&pair.effort) {
        return pair;
    }
    fired.insert(Rule::SnapEffort);
    let below = accepted
        .iter()
        .copied()
        .filter(|effort| *effort < pair.effort)
        .max();
    let above = accepted
        .iter()
        .copied()
        .filter(|effort| *effort > pair.effort)
        .min();
    let effort = below
        .or(above)
        .expect("every tier accepts at least one effort level");
    GridAgent { effort, ..pair }
}

/// The caps that the final effort is above. Only the floor can put it there.
fn conflict(
    effort: Effort,
    budget_effort: Option<Effort>,
    duration_effort: Option<Effort>,
) -> Vec<Overridden> {
    let mut overridden = Vec::new();
    if budget_effort.is_some_and(|cap| effort > cap) {
        overridden.push(Overridden::Budget);
    }
    if duration_effort.is_some_and(|cap| effort > cap) {
        overridden.push(Overridden::Duration);
    }
    overridden
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs_from_code(code: &str) -> Inputs {
        let ratings: Vec<Option<Level>> = code
            .chars()
            .map(|letter| match letter {
                'L' => Some(Level::Low),
                'M' => Some(Level::Medium),
                'H' => Some(Level::High),
                '-' => None,
                other => panic!("`{other}` is not a rating letter in `{code}`"),
            })
            .collect();
        let [
            duration,
            budget,
            intelligence,
            autonomy,
            reliability,
            consequence,
            verification,
        ] = ratings[..]
        else {
            panic!("`{code}` does not have seven ratings");
        };
        Inputs {
            duration,
            budget,
            intelligence,
            autonomy,
            reliability,
            consequence,
            verification,
        }
    }

    fn rule_ids(rules: &[Rule]) -> String {
        rules
            .iter()
            .map(|rule| rule.id())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn conflict_names(selection: &Selection) -> String {
        if selection.conflict.is_empty() {
            return "none".to_owned();
        }
        let names: Vec<&str> = selection.conflict.iter().map(|over| over.name()).collect();
        names.join(",")
    }

    fn assert_case(number: usize, selection: &Selection, agent: &str, rules: &str, conflict: &str) {
        let (tier, effort) = agent
            .split_once(" / ")
            .expect("expected agent is written as `tier / effort`");
        let expected = (
            format!("model-{tier}-effort-{effort}"),
            rules.to_owned(),
            conflict.to_owned(),
        );
        let actual = (
            selection.agent.to_string(),
            rule_ids(&selection.rules),
            conflict_names(selection),
        );
        assert_eq!(actual, expected, "case {number}");
    }

    #[test]
    fn the_cases_of_the_specification() {
        // Each case: inputs code, expected agent, rules, conflict.
        let cases = [
            ("-------", "medium / medium", "T0,E0", "none"),
            ("--M----", "medium / medium", "T1,E0", "none"),
            ("--L----", "low / medium", "T1,E0", "none"),
            ("--H----", "high / medium", "T1,E0", "none"),
            ("----L--", "medium / low", "T0,E1", "none"),
            ("-----H-", "high / high", "T0,T2,E1", "none"),
            ("----HL-", "high / high", "T0,T2,E1", "none"),
            ("----LH-", "high / high", "T0,T2,E1", "none"),
            ("--L--H-", "medium / high", "T1,T2,E1", "none"),
            ("--H--H-", "xhigh / high", "T1,T2,E1", "none"),
            ("------L", "medium / high", "T0,E0,E2", "none"),
            ("------H", "medium / low", "T0,E0,E3", "none"),
            ("---H---", "medium / high", "T0,E0,E4", "none"),
            ("---L---", "medium / low", "T0,E0,E5", "none"),
            ("---LLLH", "medium / low", "T0,E1,E3,E5", "none"),
            ("--HHHHL", "xhigh / max", "T1,T2,E1,E2,E4", "none"),
            ("--H-HHL", "xhigh / xhigh", "T1,T2,E1,E2", "none"),
            ("--M--HH", "high / high", "T1,T2,E1,E3,F1", "none"),
            ("--ML-HH", "high / high", "T1,T2,E1,E3,E5,F1", "none"),
            ("--MH-HH", "high / high", "T1,T2,E1,E3,E4", "none"),
            ("-L-----", "medium / medium", "T0,E0", "none"),
            ("-LH----", "medium / medium", "T1,E0,C1", "none"),
            (
                "-LHHHHL",
                "medium / high",
                "T1,T2,E1,E2,E4,C1,C2,F1",
                "budget",
            ),
            ("-LL--HH", "medium / high", "T1,T2,E1,E3,C2,F1", "budget"),
            ("-MHHHHL", "high / high", "T1,T2,E1,E2,E4,C1,C2", "none"),
            ("-MHHMML", "high / high", "T1,E1,E2,E4,C2", "none"),
            ("-HHHHHL", "xhigh / max", "T1,T2,E1,E2,E4", "none"),
            (
                "L-HHHHL",
                "xhigh / high",
                "T1,T2,E1,E2,E4,C3,F1",
                "duration",
            ),
            ("L-MHMM-", "medium / medium", "T1,E1,E4,C3", "none"),
            ("M-HHHHL", "xhigh / high", "T1,T2,E1,E2,E4,C3", "none"),
            (
                "LLH--H-",
                "medium / high",
                "T1,T2,E1,C1,C2,C3,F1",
                "budget,duration",
            ),
            ("HLM-M-L", "medium / medium", "T1,E1,E2,C2", "none"),
            ("--LHMML", "low / xhigh", "T1,E1,E2,E4", "none"),
            ("--LHLLH", "low / low", "T1,E1,E3,E4", "none"),
            ("--MHMMH", "medium / medium", "T1,E1,E3,E4", "none"),
            ("--HHMML", "high / xhigh", "T1,E1,E2,E4", "none"),
            ("--LHHMH", "medium / high", "T1,T2,E1,E3,E4", "none"),
            ("MMMMMMM", "medium / medium", "T1,E1", "none"),
            ("HHHHHHH", "xhigh / high", "T1,T2,E1,E3,E4", "none"),
            ("LLLLLLL", "low / low", "T1,E1,E2,E5", "none"),
            ("-LLHM-L", "low / medium", "T1,E1,E2,E4,C2", "none"),
            ("M--H--L", "medium / high", "T0,E0,E2,E4,C3", "none"),
            ("--HHHML", "high / max", "T1,T2,T3,E1,E2,E4", "none"),
            ("--H-H--", "high / high", "T1,T2,T3,E1", "none"),
        ];
        for (index, (code, agent, rules, conflict)) in cases.into_iter().enumerate() {
            let selection = select(inputs_from_code(code));
            assert_case(index + 1, &selection, agent, rules, conflict);
        }

        // Case 45: case 36 with a table in which the `high` tier lacks `xhigh`.
        let high_without_xhigh: &[Effort] =
            &[Effort::Low, Effort::Medium, Effort::High, Effort::Max];
        let gapped = [
            &Effort::ALL[..],
            &Effort::ALL[..],
            high_without_xhigh,
            &Effort::ALL[..],
        ];
        let selection = select_with(inputs_from_code("--HHMML"), &gapped);
        assert_case(45, &selection, "high / high", "T1,E1,E2,E4,V1", "none");
    }
}

//! Optimal model selection: the fixed function from a role's seven ratings to
//! one agent of the grid. It is pure, so every caller gets the same answer.

mod grid_agent;
mod mapping;

pub use grid_agent::{Effort, GridAgent, ParseGridAgentError, Tier};
pub use mapping::select;

/// Version of the selection function. Bumped whenever any input can produce
/// a different [`Selection`], so that old records keep their meaning.
pub const VERSION: u32 = 2;

/// A rating on the Low, Medium, High scale. A rating that does not apply is
/// `Option::None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, clap::ValueEnum)]
pub enum Level {
    Low = 1,
    Medium = 2,
    High = 3,
}

impl Level {
    fn position(self) -> i8 {
        self as i8
    }

    fn letter(self) -> char {
        match self {
            Level::Low => 'L',
            Level::Medium => 'M',
            Level::High => 'H',
        }
    }
}

/// The seven ratings of one role. Any of them may be `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Inputs {
    pub duration: Option<Level>,
    pub budget: Option<Level>,
    pub intelligence: Option<Level>,
    pub autonomy: Option<Level>,
    pub reliability: Option<Level>,
    pub consequence: Option<Level>,
    pub verification: Option<Level>,
}

impl Inputs {
    /// The seven ratings as seven characters, such as `-LHHHHL`: `L`, `M`,
    /// `H`, or `-` for `None`, in the order of the fields.
    #[must_use]
    pub fn code(&self) -> String {
        [
            self.duration,
            self.budget,
            self.intelligence,
            self.autonomy,
            self.reliability,
            self.consequence,
            self.verification,
        ]
        .into_iter()
        .map(|rating| rating.map_or('-', Level::letter))
        .collect()
    }
}

/// One rule of the selection function. The order of the variants is the
/// order in which fired rules are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    TierDefault,
    TierIntelligence,
    TierStakesLift,
    TierTopNeedsConsequence,
    EffortDefault,
    EffortStakes,
    EffortHardToVerify,
    EffortEasyToVerify,
    EffortAutonomyHigh,
    EffortAutonomyLow,
    CapBudgetTier,
    CapBudgetEffort,
    CapDurationEffort,
    FloorStakes,
    SnapEffort,
}

impl Rule {
    /// The permanent identifier of the rule, such as `T0` or `E4`. An
    /// identifier is never reused for a different rule.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Rule::TierDefault => "T0",
            Rule::TierIntelligence => "T1",
            Rule::TierStakesLift => "T2",
            Rule::TierTopNeedsConsequence => "T3",
            Rule::EffortDefault => "E0",
            Rule::EffortStakes => "E1",
            Rule::EffortHardToVerify => "E2",
            Rule::EffortEasyToVerify => "E3",
            Rule::EffortAutonomyHigh => "E4",
            Rule::EffortAutonomyLow => "E5",
            Rule::CapBudgetTier => "C1",
            Rule::CapBudgetEffort => "C2",
            Rule::CapDurationEffort => "C3",
            Rule::FloorStakes => "F1",
            Rule::SnapEffort => "V1",
        }
    }
}

/// A user limit that the floor for high stakes overrode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Overridden {
    Budget,
    Duration,
}

impl Overridden {
    /// `budget` or `duration`, the name of the input whose cap was overridden.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Overridden::Budget => "budget",
            Overridden::Duration => "duration",
        }
    }
}

/// The answer for one role: the agent to run, what the role needed before
/// budget and duration were applied, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub agent: GridAgent,
    pub demand: GridAgent,
    /// The rules that fired, in the order of [`Rule`].
    pub rules: Vec<Rule>,
    /// The caps that the floor overrode, budget first. Empty when none.
    pub conflict: Vec<Overridden>,
    pub inputs: Inputs,
}

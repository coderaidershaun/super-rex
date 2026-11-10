//! The names of the twenty grid agents, so that a chosen pair can be written
//! out and read back from what Claude Code reports.

use std::fmt;
use std::str::FromStr;

/// The model axis of the grid, from the cheapest model to the strongest.
/// Each tier names a Claude model alias, so the models behind it can change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Low = 1,
    Medium = 2,
    High = 3,
    Xhigh = 4,
}

impl Tier {
    const ALL: [Tier; 4] = [Tier::Low, Tier::Medium, Tier::High, Tier::Xhigh];

    fn name(self) -> &'static str {
        match self {
            Tier::Low => "low",
            Tier::Medium => "medium",
            Tier::High => "high",
            Tier::Xhigh => "xhigh",
        }
    }

    /// Position on the scale, 1 for `low` to 4 for `xhigh`.
    pub(super) fn position(self) -> i8 {
        self as i8
    }

    /// # Panics
    /// If `position` is outside 1 to 4, which is a bug in the caller.
    pub(super) fn at_position(position: i8) -> Tier {
        Tier::ALL
            .into_iter()
            .find(|tier| tier.position() == position)
            .expect("tier positions run from 1 to 4")
    }
}

/// Claude Code's reasoning effort setting, from least reasoning to most.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effort {
    Low = 1,
    Medium = 2,
    High = 3,
    Xhigh = 4,
    Max = 5,
}

impl Effort {
    pub(super) const ALL: [Effort; 5] = [
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::Xhigh,
        Effort::Max,
    ];

    fn name(self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::Xhigh => "xhigh",
            Effort::Max => "max",
        }
    }

    /// Position on the scale, 1 for `low` to 5 for `max`.
    pub(super) fn position(self) -> i8 {
        self as i8
    }

    /// # Panics
    /// If `position` is outside 1 to 5, which is a bug in the caller.
    pub(super) fn at_position(position: i8) -> Effort {
        Effort::ALL
            .into_iter()
            .find(|effort| effort.position() == position)
            .expect("effort positions run from 1 to 5")
    }
}

/// One of the twenty agent definitions: a tier and an effort level.
/// Displays and parses as `model-<tier>-effort-<level>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridAgent {
    pub tier: Tier,
    pub effort: Effort,
}

impl fmt::Display for GridAgent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "model-{}-effort-{}",
            self.tier.name(),
            self.effort.name()
        )
    }
}

/// Why a text is not the name of a grid agent.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseGridAgentError {
    #[error("`{0}` is not of the form model-<tier>-effort-<level>")]
    Shape(String),

    #[error("unknown model tier `{0}`; expected low, medium, high or xhigh")]
    Tier(String),

    #[error("unknown effort level `{0}`; expected low, medium, high, xhigh or max")]
    Effort(String),
}

impl FromStr for GridAgent {
    type Err = ParseGridAgentError;

    /// # Errors
    /// [`ParseGridAgentError::Shape`] if the text is not
    /// `model-<tier>-effort-<level>`, [`ParseGridAgentError::Tier`] or
    /// [`ParseGridAgentError::Effort`] if one part is not a known name.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let (tier_name, effort_name) = name
            .strip_prefix("model-")
            .and_then(|rest| rest.split_once("-effort-"))
            .ok_or_else(|| ParseGridAgentError::Shape(name.to_owned()))?;
        let tier = Tier::ALL
            .into_iter()
            .find(|tier| tier.name() == tier_name)
            .ok_or_else(|| ParseGridAgentError::Tier(tier_name.to_owned()))?;
        let effort = Effort::ALL
            .into_iter()
            .find(|effort| effort.name() == effort_name)
            .ok_or_else(|| ParseGridAgentError::Effort(effort_name.to_owned()))?;
        Ok(GridAgent { tier, effort })
    }
}

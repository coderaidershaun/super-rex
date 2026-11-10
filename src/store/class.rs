//! The classification values of a job, in one place, so that the command line
//! and the builder skill's lists can be checked against the same names.
//!
//! Warning: clap turns a `///` comment on a variant into help text for that
//! value, so a comment added there shows up in `rex job create --help`.

use std::fmt;

/// The kind of work a job does, stored in `job.work_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum WorkType {
    Feature,
    Fix,
    Refactor,
    Chore,
    Research,
    Other,
}

/// How big a job is, stored in `job.size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum Size {
    Small,
    Medium,
    Large,
}

/// What a mistake in the job would cost, stored in `job.consequence`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum Consequence {
    Low,
    Medium,
    High,
}

impl fmt::Display for WorkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            WorkType::Feature => "feature",
            WorkType::Fix => "fix",
            WorkType::Refactor => "refactor",
            WorkType::Chore => "chore",
            WorkType::Research => "research",
            WorkType::Other => "other",
        };
        f.write_str(text)
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Size::Small => "small",
            Size::Medium => "medium",
            Size::Large => "large",
        };
        f.write_str(text)
    }
}

impl fmt::Display for Consequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Consequence::Low => "low",
            Consequence::Medium => "medium",
            Consequence::High => "high",
        };
        f.write_str(text)
    }
}

#[cfg(test)]
mod tests {
    use clap::ValueEnum;

    use super::*;

    #[test]
    fn value_names_are_the_stored_text() {
        fn names<T: ValueEnum + fmt::Display>() -> Vec<(String, String)> {
            T::value_variants()
                .iter()
                .map(|value| {
                    let name = value.to_possible_value().unwrap().get_name().to_owned();
                    (name, value.to_string())
                })
                .collect()
        }
        let pairs = [names::<WorkType>(), names::<Size>(), names::<Consequence>()].concat();
        let value_names: Vec<&str> = pairs.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            value_names,
            [
                "feature", "fix", "refactor", "chore", "research", "other", "small", "medium",
                "large", "low", "medium", "high"
            ]
        );
        for (name, shown) in &pairs {
            assert_eq!(name, shown);
        }
    }
}

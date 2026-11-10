//! The classification values of a job, in one place, so that the command line
//! and the builder skill's lists can be checked against the same names.
//!
//! Warning: clap turns a `///` comment on a variant into help text for that
//! value, so a comment added there shows up in `rex job create --help`.

use std::fmt;

use clap::ValueEnum;

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

/// The stored text is the command-line value name, so both lists are one list.
fn write_value_name(value: &impl ValueEnum, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let name = value
        .to_possible_value()
        .expect("no classification value is skipped on the command line");
    f.write_str(name.get_name())
}

impl fmt::Display for WorkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_value_name(self, f)
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_value_name(self, f)
    }
}

impl fmt::Display for Consequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_value_name(self, f)
    }
}

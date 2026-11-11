//! One module per `rex` command (`catalog` serves `rex commands`, `gate` also
//! serves `rex limit`, `test_output` serves `rex test-hook` and `rex test-filter`),
//! each with a `run` handler, plus the private helpers that the commands share.
//!
//! A command has an options struct that derives `clap::Args` and one
//! `run(cwd, options)` that returns the command's own error type, which
//! implements [`crate::error::CommandError`]. Let clap check what it can: a
//! required flag, a value from a fixed list, the form of one value, and flags
//! that need or exclude each other. A check that clap cannot make, such as one
//! that spans several values, is the command's own, and its error says through
//! `is_usage` whether the command line is at fault.
//! Extra help for a command goes in `#[command(after_help = "...")]` on its
//! options struct, which clap keeps.
//!
//! Agents read the output, so every line costs tokens. stdout holds the result
//! and nothing else, as plain text with one record on one line: `key=value`
//! lines in a fixed order, the answer first. Warnings and errors go to stderr.
//! Print through `stdout`, which treats a reader that closes the pipe early as
//! success. Modules outside this folder never print.

pub mod catalog;
pub mod codebase;
mod comments;
pub mod feedback;
pub mod gate;
pub mod hook;
pub mod init;
pub mod job;
pub mod jobs;
mod language;
mod links;
mod module_comment;
pub mod oms;
pub mod recall;
pub mod skills;
pub mod smells;
pub mod status;
mod stdout;
pub mod test_output;
mod token_text;
mod walk;
pub mod watch;

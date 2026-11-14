//! One module per `rex` command (`catalog` serves `rex commands`, `codebase` also
//! serves `rex smells`), each with a `run` handler, plus the private helpers that
//! the commands share.
//!
//! A command has an options struct that derives `clap::Args` and one
//! `run(cwd, options)` that returns the command's own error type, which
//! implements [`crate::error::CommandError`]. Let clap check what it can: a
//! required flag, a value from a fixed list, the form of one value, and flags
//! that need or exclude each other. A check that clap cannot make, such as one
//! that spans several values, is the command's own, and its error says through
//! `is_usage` whether the command line is at fault.
//! The doc comment of the options struct is the command's description in
//! `rex --help` and `rex commands`, so it is one paragraph that says what the
//! command does. Extra help goes in `#[command(after_help = "...")]` beside it.
//!
//! Agents read the output, so every line costs tokens. stdout holds the result
//! and nothing else, as plain text with one record on one line: `key=value`
//! lines in a fixed order, the answer first. Warnings and errors go to stderr.
//! Print through `stdout`, which treats a reader that closes the pipe early as
//! success. Modules outside this folder never print.

// SMELL: this folder holds 21 modules, more than the 12 that one folder may hold.
// Eighteen of them are commands, so the fix is to group commands that belong
// together, as `codebase` groups two, and each such group changes public paths.
pub mod catalog;
pub mod codebase;
pub mod doctor;
pub mod feedback;
pub mod ingest;
pub mod init;
pub mod job;
pub mod jobs;
pub mod limit;
mod links;
pub mod migrate;
pub mod oms;
pub mod recall;
pub mod skills;
pub mod status;
mod stdout;
pub mod test_filter;
mod token_text;
pub mod tree;
pub mod watch;
pub mod worktree;

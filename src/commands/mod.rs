//! One module per `rex` command (`catalog` serves `rex commands`), each with a
//! `run` handler, plus the private helpers that the commands share.

pub mod catalog;
pub mod codebase;
// The private helpers have no callers until the commands that use them are
// built, so their dead-code warnings are allowed.
#[allow(dead_code)]
mod comments;
pub mod feedback;
pub mod hook;
pub mod init;
pub mod job;
pub mod jobs;
#[allow(dead_code)]
mod language;
#[allow(dead_code)]
mod links;
#[allow(dead_code)]
mod module_comment;
pub mod oms;
pub mod recall;
pub mod skills;
pub mod smells;
pub mod status;
#[allow(dead_code)]
mod stdout;
#[allow(dead_code)]
mod walk;
pub mod watch;

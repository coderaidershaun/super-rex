//! One module per `rex` command (`catalog` serves `rex commands`), each with a
//! `run` handler, plus the private helpers that the commands share.

pub mod catalog;
pub mod codebase;
mod comments;
pub mod feedback;
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
mod walk;
pub mod watch;

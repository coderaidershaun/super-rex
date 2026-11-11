//! `rex codebase` and `rex smells`: the two commands that read the source files of
//! the working directory, so an agent finds its way around a repo in one call.

mod comments;
mod language;
mod module_comment;
mod smells;
mod tree;
mod walk;

pub use smells::{SmellsOptions, run as run_smells};
pub use tree::{CodebaseOptions, run};

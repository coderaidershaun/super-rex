//! The safety settings, the deny rules and the depth that `rex init` merges into
//! `.claude/settings.local.json`, a file the person owns: what it adds, what
//! it keeps of theirs, what it takes out, and what it refuses to change.

mod expected;
mod merge;
mod safety;
mod stopgap;
mod unmergeable;

//! The safety settings and the hook registration that `rex init` merges into
//! `.claude/settings.local.json`, a file the person owns: what it adds, what
//! it keeps of theirs, and what it refuses to change.

mod expected;
mod merge;
mod safety;
mod unmergeable;

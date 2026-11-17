//! The permissions, the deny rules and the depth that `rex init` merges into
//! `.claude/settings.local.json`, a file the person owns: what it adds, what
//! it keeps of theirs, what it takes out, and the old sandbox it turns off.

mod expected;
mod merge;
mod old_sandbox;
mod safety;
mod unmergeable;
mod worktrees;

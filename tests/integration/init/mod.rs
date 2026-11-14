//! Integration tests of `rex init`.

#[cfg(unix)]
mod broken_pipe;
mod install;
#[cfg(unix)]
mod links;
mod old_claude_copies;
mod old_gitignore;
mod old_library;
mod pre_commit;
mod preflight;
mod settings;
mod store;
mod store_upgrade;
mod support;

//! Integration tests of `rex init`.

#[cfg(unix)]
mod broken_pipe;
mod install;
#[cfg(unix)]
mod links;
mod pre_commit;
mod preflight;
mod settings;
mod store;
mod support;
#[cfg(unix)]
mod unreadable;
mod update;

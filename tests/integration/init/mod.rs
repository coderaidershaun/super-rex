//! Integration tests of `rex init`.

#[cfg(unix)]
mod broken_pipe;
mod install;
#[cfg(unix)]
mod links;
mod pre_commit;
mod support;
#[cfg(unix)]
mod unreadable;
mod update;

//! Everything behind the `rex` binary, in library form so that tests can call
//! it directly instead of running the binary.
//!
//! Priorities, in order: correctness (authority fails closed), visible state (an observer that fails marks the job degraded), simplicity.
//! Never: merge without a merge answer from the person or the pinned policy.
//! Never: start a master or a builder from anything but the person's own prompt or the master's own tool.
//! Never: let a failed observer block a prompt, a tool call or a spawn.

pub mod bundle;
pub mod capture;
pub mod commands;
pub mod error;
pub mod ingest;
pub mod oms;
pub mod store;

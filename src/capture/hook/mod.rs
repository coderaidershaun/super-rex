//! What `rex hook` leaves and what `rex init` registers: the event log and the hook
//! entries that fill it.

mod log;
mod registration;

pub(crate) use log::{LogLine, append_event, read_log};
pub use registration::{GATE_COMMAND, HOOK_COMMAND, REX_COMMANDS, hook_registration};

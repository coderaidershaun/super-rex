//! Implementation behind the `rex` binary. Command logic lives here in
//! library form, so tests and other modules can call it directly.

pub mod bundle;
pub mod capture;
pub mod commands;
pub mod error;
pub mod oms;
pub mod store;

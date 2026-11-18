//! The port's name for Horizon's generated API.
//!
//! `horizon_client::mod_api` is generated from `native-hooks.json` at the pinned
//! commit: `Hooks`, `Host` and one event struct per hook. The port imports it
//! under this module name so the same code builds against Horizon and against
//! the hand-written stand-in the port's own tests use.

pub mod mod_api {
    pub use horizon_client::mod_api::*;
}

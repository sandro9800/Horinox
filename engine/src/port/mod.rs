//! HORIXON: a native Horizon trait mod.
//!
//! `hooks()` is the entry point Horizon's launcher links into the game
//! (`[api]` package contract: `pub fn hooks() -> Box<dyn horizon_client::mod_api::Hooks>`).
//! The rest of the crate is the port itself:
//!
//! * [`modules`] - the Meteor-derived modules, by category;
//! * [`hud`] - the HUD elements and their layout;
//! * [`gui`] - the original HORIXON Command Center GUI;
//! * [`shared`] - the systems the Java client keeps in `Systems`;
//! * [`env_impl`] - the one file that talks to Horizon's API.

pub mod commands;
pub mod env_impl;
pub mod gui;
pub mod hooks;
pub mod hud;
pub mod modules;
pub mod shared;

pub use hooks::HorixonHooks;

use crate::horizon_api_shim::mod_api::Hooks;

/// Horizon's trait-mod entry point.
pub fn hooks() -> Box<dyn Hooks> {
    Box::new(HorixonHooks::new())
}

/// The mod's id, which Horizon uses for its data folder and its messages.
pub const MOD_ID: &str = "horixon";

/// What the port reports as its version (`Cargo.toml`'s, kept in one place for
/// the crash report line and the GUI's watermark).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hooks_constructs() {
        let h = hooks();
        // The trait object is `Send + Sync` and answers a tick without a host.
        let _ = &h;
    }
}

//! HORIXON for Horizon: one native bundle, linked into the game by the launcher.
//!
//! * [`meteor_core`] - the engine-agnostic systems (modules, settings, config, HUD
//!   model, commands, friends, theme);
//! * [`port`] - the mod itself: the state, the `Env` over Horizon's `Host`, the
//!   hooks, the HORIXON Command Center, the HUD and the 167 expected modules;
//! * [`horizon_api_shim`] - the seam to `horizon_client::mod_api`.

pub mod horizon_api_shim;
pub mod meteor_core;
pub mod port;

pub use port::*;

/// Horizon's trait-mod entry point.
pub fn hooks() -> Box<dyn horizon_api_shim::mod_api::Hooks> {
    port::hooks()
}

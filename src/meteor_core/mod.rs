//! HORIXON's GPL-licensed port of Meteor Client's engine-agnostic systems.
//!
//! The Java client is organised as `systems` (modules, settings, config, friends,
//! hud, commands, macros, waypoints, accounts, profiles, proxies) plus an addon
//! surface and a GUI toolkit. This crate carries the part of that architecture
//! that does not touch the game engine at all, so the same code runs under
//! Horizon's trait-mod build and under plain `cargo test`.
//!
//! Port coverage and known API gaps are summarized in `PORT-MAP.md` at the
//! repository root. Source-level `PORT GAP` markers identify behaviours that
//! still need an equivalent Horizon hook or a more faithful implementation.

pub mod addon;
pub mod category;
pub mod color;
pub mod command;
pub mod config;
pub mod env;
pub mod friends;
pub mod hud;
pub mod manager;
pub mod math;
pub mod module;
pub mod setting;
pub mod theme;
pub mod util;

/// The Meteor Client version this port is tracking.
pub const METEOR_VERSION: &str = "1.0.0";
pub const HORIXON_VERSION: &str = "0.5.0";

/// HORIXON native mod ID (kept separate from the upstream Java namespace).
pub const MOD_ID: &str = "horixon";

/// Everything a module needs, in one `use`.
pub mod prelude {
    pub use crate::meteor_core::category::Category;
    pub use crate::meteor_core::color::{Color, SettingColor};
    pub use crate::meteor_core::command::{CmdCtx, Command, CommandManager};
    pub use crate::meteor_core::config::Tag;
    pub use crate::meteor_core::env::{
        EntityInfo, Env, Hand, InvSlot, ItemStackInfo, PlayerInfo, Raycast, Screen, WorldInfo,
    };
    pub use crate::meteor_core::friends::Friends;
    pub use crate::meteor_core::hud::{
        Alignment, Hud, HudElement, HudElementInfo, HudGroup, XAnchor, YAnchor,
    };
    pub use crate::meteor_core::manager::Modules;
    pub use crate::meteor_core::math::{BlockPos, Rotation, Timer, Vec3d};
    pub use crate::meteor_core::module::{Module, ModuleBase};
    pub use crate::meteor_core::setting::{
        BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, ItemListSet, KeybindSet, ListSet, Setting,
        SettingGroup, Settings, StringSet,
    };
    pub use crate::meteor_core::theme::MeteorTheme;
    pub use crate::meteor_core::util::{name_to_title, ChatMsg, Prefix};
}

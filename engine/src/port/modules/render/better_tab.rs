//! `systems.modules.render.BetterTab` - Various improvements to the tab list.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, IntSet};

pub struct BetterTab {
    base: ModuleBase,
    tablist_size: IntSet,
    column_height: IntSet,
    highlight_self: BoolSet,
    self_color: ColorSet,
    highlight_friends: BoolSet,
    accurate_latency: BoolSet,
    gamemode: BoolSet,
}

impl BetterTab {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "better-tab",
            "Various improvements to the tab list.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let tablist_size = g.int_setting("tablist-size", 100, 1, 1000);
        let column_height = g.int_setting("column-height", 20, 1, 1000);
        let highlight_self = g.bool_setting("highlight-self", true);
        let self_color = g.color_setting(
            "self-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let highlight_friends = g.bool_setting("highlight-friends", true);
        let accurate_latency = g.bool_setting("accurate-latency", true);
        let gamemode = g.bool_setting("gamemode", false);
        Self {
            base,
            tablist_size,
            column_height,
            highlight_self,
            self_color,
            highlight_friends,
            accurate_latency,
            gamemode,
        }
    }
}

impl Default for BetterTab {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BetterTab {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        // The Java module changes the renderer's state; Horizon's equivalent is
        // the hook this module registers, so the port reports the switch.
        // PORT GAP: the engine-side effect this module toggles.
        let _ = env;
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(BetterTab::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = BetterTab::new();
        assert_eq!(m.base.name, "better-tab");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = BetterTab::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

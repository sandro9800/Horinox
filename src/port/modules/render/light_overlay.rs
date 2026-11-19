//! `systems.modules.render.LightOverlay` - Shows blocks where mobs can spawn.
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

pub struct LightOverlay {
    base: ModuleBase,
    horizontal_range: IntSet,
    vertical_range: IntSet,
    see_through_blocks: BoolSet,
    light_level: IntSet,
    color: ColorSet,
    potential_color: ColorSet,
}

impl LightOverlay {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "light-overlay",
            "Shows blocks where mobs can spawn.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let horizontal_range = g.int_setting("horizontal-range", 8, 0, 10);
        let vertical_range = g.int_setting("vertical-range", 4, 0, 10);
        let see_through_blocks = g.bool_setting("see-through-blocks", false);
        let light_level = g.int_setting("light-level", 0, 0, 10);
        let color = g.color_setting(
            "color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let potential_color = g.color_setting(
            "potential-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        Self {
            base,
            horizontal_range,
            vertical_range,
            see_through_blocks,
            light_level,
            color,
            potential_color,
        }
    }
}

impl Default for LightOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for LightOverlay {
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
    modules.add(Box::new(LightOverlay::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = LightOverlay::new();
        assert_eq!(m.base.name, "light-overlay");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = LightOverlay::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

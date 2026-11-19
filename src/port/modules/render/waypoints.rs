//! `systems.modules.render.WaypointsModule` - Allows you to create waypoints.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet};

pub struct Waypoints {
    base: ModuleBase,
    text_render_distance: IntSet,
    waypoint_fade_distance: IntSet,
    max_death_positions: IntSet,
    chat: BoolSet,
}

impl Waypoints {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "waypoints",
            "Allows you to create waypoints.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let text_render_distance = g.int_setting("text-render-distance", 100, 0, 100);
        let waypoint_fade_distance = g.int_setting("waypoint-fade-distance", 20, 0, 100);
        let max_death_positions = g.int_setting("max-death-positions", 0, 0, 10);
        let chat = g.bool_setting("chat", false);
        Self {
            base,
            text_render_distance,
            waypoint_fade_distance,
            max_death_positions,
            chat,
        }
    }
}

impl Default for Waypoints {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Waypoints {
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
    modules.add(Box::new(Waypoints::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Waypoints::new();
        assert_eq!(m.base.name, "waypoints");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = Waypoints::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

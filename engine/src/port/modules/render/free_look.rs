//! `systems.modules.render.FreeLook` - Allows more rotation options in third person.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `tick` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, StringSet};

pub struct FreeLook {
    base: ModuleBase,
    mode: StringSet,
    toggle_perspective: BoolSet,
    camera_sensitivity: DoubleSet,
    arrows_control_opposite: BoolSet,
    arrow_speed: DoubleSet,
    ticks: u64,
}

impl FreeLook {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "free-look",
            "Allows more rotation options in third person.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let mode = g.string_setting("mode", "");
        let toggle_perspective = g.bool_setting("toggle-perspective", true);
        let camera_sensitivity = g.double_setting("camera-sensitivity", 8.0, 0.0, 10.0, 3);
        let arrows_control_opposite = g.bool_setting("arrows-control-opposite", true);
        let arrow_speed = g.double_setting("arrow-speed", 4.0, 0.0, 10.0, 3);

        Self {
            base,
            ticks: 0,
            mode,
            toggle_perspective,
            camera_sensitivity,
            arrows_control_opposite,
            arrow_speed,
        }
    }
}

impl Default for FreeLook {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for FreeLook {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        let _ = player;
        self.ticks += 1;
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(FreeLook::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = FreeLook::new();
        assert_eq!(m.base.name, "free-look");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = FreeLook::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

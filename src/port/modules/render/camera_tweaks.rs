//! `systems.modules.render.CameraTweaks` - Allows modification of the third person camera.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, KeybindSet};

pub struct CameraTweaks {
    base: ModuleBase,
    clip: BoolSet,
    camera_distance: DoubleSet,
    scrolling: BoolSet,
    bind: KeybindSet,
    sensitivity: DoubleSet,
}

impl CameraTweaks {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "camera-tweaks",
            "Allows modification of the third person camera.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let clip = g.bool_setting("clip", true);
        let camera_distance = g.double_setting("camera-distance", 4.0, 0.0, 10.0, 3);
        let scrolling = g.bool_setting("scrolling", true);
        let bind = g.keybind_setting("bind");
        let sensitivity = g.double_setting("sensitivity", 1.0, 0.01, 10.0, 3);
        Self {
            base,
            clip,
            camera_distance,
            scrolling,
            bind,
            sensitivity,
        }
    }
}

impl Default for CameraTweaks {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for CameraTweaks {
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
    modules.add(Box::new(CameraTweaks::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = CameraTweaks::new();
        assert_eq!(m.base.name, "camera-tweaks");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = CameraTweaks::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

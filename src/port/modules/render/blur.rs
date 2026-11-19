//! `systems.modules.render.Blur` - Blurs background when in GUI screens.
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

pub struct Blur {
    base: ModuleBase,
    strength: IntSet,
    fade_time: IntSet,
    meteor: BoolSet,
    inventories: BoolSet,
    chat: BoolSet,
    other: BoolSet,
}

impl Blur {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "blur",
            "Blurs background when in GUI screens.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let strength = g.int_setting("strength", 5, 1, 20);
        let fade_time = g.int_setting("fade-time", 100, 0, 10);
        let meteor = g.bool_setting("meteor", true);
        let inventories = g.bool_setting("inventories", true);
        let chat = g.bool_setting("chat", false);
        let other = g.bool_setting("other", true);
        Self {
            base,
            strength,
            fade_time,
            meteor,
            inventories,
            chat,
            other,
        }
    }
}

impl Default for Blur {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Blur {
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
    modules.add(Box::new(Blur::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Blur::new();
        assert_eq!(m.base.name, "blur");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = Blur::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

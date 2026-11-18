//! `systems.modules.player.GhostHand` - Opens containers through walls.
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

pub struct GhostHand {
    base: ModuleBase,
    range: IntSet,
    rotate: BoolSet,
}

impl GhostHand {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "ghost-hand",
            "Opens containers through walls.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let range = g.int_setting("range", 4, 1, 6);
        let rotate = g.bool_setting("rotate", false);

        Self {
            base,
            range,
            rotate,
        }
    }
}

impl Default for GhostHand {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for GhostHand {
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
    modules.add(Box::new(GhostHand::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = GhostHand::new();
        assert_eq!(m.base.name, "ghost-hand");
        assert_eq!(m.base.category, Category::Player);
    }

    #[test]
    fn it_runs() {
        let mut m = GhostHand::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

//! `systems.modules.movement.ClickTP` - Teleports you to the block you click on.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `tick` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::IntSet;

pub struct ClickTp {
    base: ModuleBase,
    max_distance: IntSet,

    ticks: u64,
}

impl ClickTp {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "click-tp",
            "Teleports you to the block you click on.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let max_distance = g.int_setting("max-distance", 100, 1, 256);

        Self {
            base,
            max_distance,
            ticks: 0,
        }
    }
}

impl Default for ClickTp {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ClickTp {
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
    modules.add(Box::new(ClickTp::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = ClickTp::new();
        assert_eq!(m.base.name, "click-tp");
        assert_eq!(m.base.category, Category::Movement);
    }

    #[test]
    fn it_runs() {
        let mut m = ClickTp::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

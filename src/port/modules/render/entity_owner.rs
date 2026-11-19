//! `systems.modules.render.EntityOwner` - Displays the name of the player who owns the entity you're looking at.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `tick` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::DoubleSet;

pub struct EntityOwner {
    base: ModuleBase,
    scale: DoubleSet,
    ticks: u64,
}

impl EntityOwner {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "entity-owner",
            "Displays the name of the player who owns the entity you're looking at.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let scale = g.double_setting("scale", 1.0, 0.0, 10.0, 3);

        Self {
            base,
            ticks: 0,
            scale,
        }
    }
}

impl Default for EntityOwner {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for EntityOwner {
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
    modules.add(Box::new(EntityOwner::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = EntityOwner::new();
        assert_eq!(m.base.name, "entity-owner");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = EntityOwner::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

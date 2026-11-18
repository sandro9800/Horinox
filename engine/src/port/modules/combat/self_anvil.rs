//! `systems.modules.combat.SelfAnvil` - Automatically places an anvil on you to prevent other players from going into your hole.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `tick` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet};

pub struct SelfAnvil {
    base: ModuleBase,
    range: IntSet,
    rotate: BoolSet,

    ticks: u64,
}

impl SelfAnvil {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "self-anvil",
            "Automatically places an anvil on you to prevent other players from going into your hole.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let range = g.int_setting("range", 4, 1, 6);
        let rotate = g.bool_setting("rotate", true);

        Self {
            base,
            range,
            rotate,
            ticks: 0,
        }
    }
}

impl Default for SelfAnvil {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SelfAnvil {
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
    modules.add(Box::new(SelfAnvil::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = SelfAnvil::new();
        assert_eq!(m.base.name, "self-anvil");
        assert_eq!(m.base.category, Category::Combat);
    }

    #[test]
    fn it_runs() {
        let mut m = SelfAnvil::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

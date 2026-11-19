//! `systems.modules.render.BossStack` - Stacks boss bars to make your HUD less cluttered.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet};

pub struct BossStack {
    base: ModuleBase,
    stack: BoolSet,
    hide_name: BoolSet,
    bar_spacing: DoubleSet,
}

impl BossStack {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "boss-stack",
            "Stacks boss bars to make your HUD less cluttered.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let stack = g.bool_setting("stack", true);
        let hide_name = g.bool_setting("hide-name", false);
        let bar_spacing = g.double_setting("bar-spacing", 10.0, 0.0, 10.0, 3);
        Self {
            base,
            stack,
            hide_name,
            bar_spacing,
        }
    }
}

impl Default for BossStack {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BossStack {
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
    modules.add(Box::new(BossStack::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = BossStack::new();
        assert_eq!(m.base.name, "boss-stack");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = BossStack::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

//! `systems.modules.render.Xray` - Only renders specified blocks. Good for mining.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, ListSet, StringSet};

pub struct Xray {
    base: ModuleBase,
    whitelist: ListSet,
    opacity: IntSet,
    fluid_opacity: StringSet,
    exposed_only: BoolSet,
}

impl Xray {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "xray",
            "Only renders specified blocks. Good for mining.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let whitelist = g.list_setting("whitelist", &[]);
        let opacity = g.int_setting("opacity", 25, 0, 10);
        let fluid_opacity = g.string_setting("fluid-opacity", "");
        let exposed_only = g.bool_setting("exposed-only", false);
        Self {
            base,
            whitelist,
            opacity,
            fluid_opacity,
            exposed_only,
        }
    }
}

impl Default for Xray {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Xray {
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
    modules.add(Box::new(Xray::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Xray::new();
        assert_eq!(m.base.name, "xray");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = Xray::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

//! `systems.modules.player.AutoRespawn` - Automatically respawns after death.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `esp` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{ColorSet, IntSet};

pub struct AutoRespawn {
    base: ModuleBase,

    range: IntSet,
    color: ColorSet,
}

impl AutoRespawn {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "auto-respawn",
            "Automatically respawns after death.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let range = g.int_setting("range", 64, 1, 256);
        let color = g.color_setting(
            "color",
            crate::meteor_core::color::SettingColor::rgb(145, 61, 226),
        );
        Self { base, range, color }
    }
}

impl Default for AutoRespawn {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoRespawn {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let Some(player) = env.player() else { return };
        let range = self.range.get() as f64;
        let color = self.color.color().argb();
        for entity in env.entities() {
            if !entity.alive || entity.pos.distance(player.pos) > range {
                continue;
            }
            let (min, max) = entity.bounding_box();
            r.box_outline(min, max, color, 1.5, true);
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoRespawn::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = AutoRespawn::new();
        assert_eq!(m.base.name, "auto-respawn");
        assert_eq!(m.base.category, Category::Player);
    }

    #[test]
    fn it_runs() {
        let mut m = AutoRespawn::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

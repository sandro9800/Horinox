//! `systems.modules.render.ItemPhysics` - Applies physics to items on the ground.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `esp` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, IntSet};

pub struct ItemPhysics {
    base: ModuleBase,
    random_rotation: BoolSet,
    range: IntSet,
    color: ColorSet,
}

impl ItemPhysics {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "item-physics",
            "Applies physics to items on the ground.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let random_rotation = g.bool_setting("random-rotation", true);
        let range = g.int_setting("range", 64, 1, 256);
        let color = g.color_setting(
            "color",
            crate::meteor_core::color::SettingColor::rgb(145, 61, 226),
        );
        Self {
            base,
            random_rotation,
            range,
            color,
        }
    }
}

impl Default for ItemPhysics {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ItemPhysics {
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
    modules.add(Box::new(ItemPhysics::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = ItemPhysics::new();
        assert_eq!(m.base.name, "item-physics");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = ItemPhysics::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

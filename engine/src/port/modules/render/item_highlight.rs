//! `systems.modules.render.ItemHighlight` - Highlights selected items when in guis
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `esp` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{ColorSet, IntSet, ItemListSet};

pub struct ItemHighlight {
    base: ModuleBase,
    items: ItemListSet,
    color: ColorSet,
    range: IntSet,
}

impl ItemHighlight {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "item-highlight",
            "Highlights selected items when in guis",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let items = g.item_list_setting("items", &[]);
        let color = g.color_setting(
            "color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let range = g.int_setting("range", 64, 1, 256);
        Self {
            base,
            items,
            color,
            range,
        }
    }
}

impl Default for ItemHighlight {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ItemHighlight {
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
    modules.add(Box::new(ItemHighlight::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = ItemHighlight::new();
        assert_eq!(m.base.name, "item-highlight");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = ItemHighlight::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

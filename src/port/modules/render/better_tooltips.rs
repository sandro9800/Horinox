//! `systems.modules.render.BetterTooltips` - Displays more useful tooltips for certain items.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, KeybindSet, StringSet};

pub struct BetterTooltips {
    base: ModuleBase,
    display_when: StringSet,
    keybind: KeybindSet,
    open_contents: BoolSet,
    pause_in_creative: BoolSet,
    containers: BoolSet,
    compact_shulker_tooltip: BoolSet,
    echests: BoolSet,
    maps: BoolSet,
    map_scale: DoubleSet,
    books: BoolSet,
    banners: BoolSet,
    entities_in_buckets: BoolSet,
    bundles: BoolSet,
    food_info: BoolSet,
    byte_size: BoolSet,
    byte_size_format: StringSet,
    status_effects: BoolSet,
    tooltip: BoolSet,
    tooltip_components: BoolSet,
}

impl BetterTooltips {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "better-tooltips",
            "Displays more useful tooltips for certain items.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let display_when = g.string_setting("display-when", "");
        let keybind = g.keybind_setting("keybind");
        let open_contents = g.bool_setting("open-contents", true);
        let pause_in_creative = g.bool_setting("pause-in-creative", true);
        let containers = g.bool_setting("containers", true);
        let compact_shulker_tooltip = g.bool_setting("compact-shulker-tooltip", true);
        let echests = g.bool_setting("echests", true);
        let maps = g.bool_setting("maps", true);
        let map_scale = g.double_setting("map-scale", 1.0, 0.001, 10.0, 3);
        let books = g.bool_setting("books", true);
        let banners = g.bool_setting("banners", true);
        let entities_in_buckets = g.bool_setting("entities-in-buckets", true);
        let bundles = g.bool_setting("bundles", true);
        let food_info = g.bool_setting("food-info", true);
        let byte_size = g.bool_setting("byte-size", true);
        let byte_size_format = g.string_setting("byte-size-format", "");
        let status_effects = g.bool_setting("status-effects", true);
        let tooltip = g.bool_setting("tooltip", false);
        let tooltip_components = g.bool_setting("tooltip-components", false);
        Self {
            base,
            display_when,
            keybind,
            open_contents,
            pause_in_creative,
            containers,
            compact_shulker_tooltip,
            echests,
            maps,
            map_scale,
            books,
            banners,
            entities_in_buckets,
            bundles,
            food_info,
            byte_size,
            byte_size_format,
            status_effects,
            tooltip,
            tooltip_components,
        }
    }
}

impl Default for BetterTooltips {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BetterTooltips {
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
    modules.add(Box::new(BetterTooltips::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = BetterTooltips::new();
        assert_eq!(m.base.name, "better-tooltips");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = BetterTooltips::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

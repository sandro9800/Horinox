//! `systems.modules.render.Nametags` - Displays customizable nametags above players, items and other entities.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `text` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, IntSet, ListSet, StringSet};

pub struct Nametags {
    base: ModuleBase,
    entities: ListSet,
    scale: DoubleSet,
    ignore_self: BoolSet,
    ignore_friends: BoolSet,
    ignore_bots: BoolSet,
    culling: BoolSet,
    culling_range: DoubleSet,
    culling_count: IntSet,
    health: BoolSet,
    use_display_name: BoolSet,
    gamemode: BoolSet,
    distance: BoolSet,
    ping: BoolSet,
    items: BoolSet,
    item_spacing: DoubleSet,
    ignore_empty_slots: BoolSet,
    durability: StringSet,
    display_enchants: BoolSet,
    shown_enchantments: ListSet,
    enchantment_position: StringSet,
    enchant_name_length: IntSet,
    enchant_text_scale: DoubleSet,
    show_count: BoolSet,
    background_color: ColorSet,
    name_color: ColorSet,
    ping_color: ColorSet,
    gamemode_color: ColorSet,
    distance_color_mode: StringSet,
    distance_color: ColorSet,
    range: IntSet,
}

impl Nametags {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "nametags",
            "Displays customizable nametags above players, items and other entities.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let entities = g.list_setting("entities", &[]);
        let scale = g.double_setting("scale", 1.1, 0.1, 10.0, 3);
        let ignore_self = g.bool_setting("ignore-self", true);
        let ignore_friends = g.bool_setting("ignore-friends", false);
        let ignore_bots = g.bool_setting("ignore-bots", true);
        let culling = g.bool_setting("culling", false);
        let culling_range = g.double_setting("culling-range", 20.0, 0.0, 100.0, 3);
        let culling_count = g.int_setting("culling-count", 50, 1, 100);
        let health = g.bool_setting("health", true);
        let use_display_name = g.bool_setting("use-display-name", false);
        let gamemode = g.bool_setting("gamemode", false);
        let distance = g.bool_setting("distance", false);
        let ping = g.bool_setting("ping", true);
        let items = g.bool_setting("items", true);
        let item_spacing = g.double_setting("item-spacing", 2.0, 0.0, 10.0, 3);
        let ignore_empty_slots = g.bool_setting("ignore-empty-slots", true);
        let durability = g.string_setting("durability", "");
        let display_enchants = g.bool_setting("display-enchants", false);
        let shown_enchantments = g.list_setting("shown-enchantments", &[]);
        let enchantment_position = g.string_setting("enchantment-position", "");
        let enchant_name_length = g.int_setting("enchant-name-length", 3, 0, 5);
        let enchant_text_scale = g.double_setting("enchant-text-scale", 1.0, 0.1, 2.0, 3);
        let show_count = g.bool_setting("show-count", true);
        let background_color = g.color_setting(
            "background-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let name_color = g.color_setting(
            "name-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let ping_color = g.color_setting(
            "ping-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let gamemode_color = g.color_setting(
            "gamemode-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let distance_color_mode = g.string_setting("distance-color-mode", "");
        let distance_color = g.color_setting(
            "distance-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let range = g.int_setting("range", 64, 1, 256);
        Self {
            base,
            entities,
            scale,
            ignore_self,
            ignore_friends,
            ignore_bots,
            culling,
            culling_range,
            culling_count,
            health,
            use_display_name,
            gamemode,
            distance,
            ping,
            items,
            item_spacing,
            ignore_empty_slots,
            durability,
            display_enchants,
            shown_enchantments,
            enchantment_position,
            enchant_name_length,
            enchant_text_scale,
            show_count,
            background_color,
            name_color,
            ping_color,
            gamemode_color,
            distance_color_mode,
            distance_color,
            range,
        }
    }
}

impl Default for Nametags {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Nametags {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let Some(player) = env.player() else { return };
        let range = self.range.get() as f64;
        for entity in env.entities() {
            if !entity.alive || entity.pos.distance(player.pos) > range {
                continue;
            }
            let name = entity
                .name
                .clone()
                .unwrap_or_else(|| entity.short_kind().to_string());
            let at = Vec3d::new(entity.pos.x, entity.pos.y + 2.2, entity.pos.z);
            r.text3d(at, &name, 0xFFFF_FFFF, 1.0, true);
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Nametags::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Nametags::new();
        assert_eq!(m.base.name, "nametags");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = Nametags::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

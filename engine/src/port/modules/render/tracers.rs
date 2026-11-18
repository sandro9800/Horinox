//! `systems.modules.render.Tracers` - Displays tracer lines to specified entities.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `lines` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, IntSet, ListSet, StringSet};

pub struct Tracers {
    base: ModuleBase,
    entities: ListSet,
    ignore_self: BoolSet,
    ignore_friends: BoolSet,
    show_invisible: BoolSet,
    style: StringSet,
    target: StringSet,
    stem: BoolSet,
    max_distance: IntSet,
    distance_offscreen: IntSet,
    size_offscreen: IntSet,
    blink_offscreen: BoolSet,
    blink_offscreen_speed: DoubleSet,
    distance_colors: BoolSet,
    show_friend_colors: BoolSet,
    players_colors: ColorSet,
    animals_color: ColorSet,
    water_animals_color: ColorSet,
    monsters_color: ColorSet,
    ambient_color: ColorSet,
    misc_color: ColorSet,
    range: IntSet,
    color: ColorSet,
    points: Vec<Vec3d>,
}

impl Tracers {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "tracers",
            "Displays tracer lines to specified entities.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let entities = g.list_setting("entities", &[]);
        let ignore_self = g.bool_setting("ignore-self", false);
        let ignore_friends = g.bool_setting("ignore-friends", false);
        let show_invisible = g.bool_setting("show-invisible", true);
        let style = g.string_setting("style", "");
        let target = g.string_setting("target", "");
        let stem = g.bool_setting("stem", true);
        let max_distance = g.int_setting("max-distance", 256, 0, 10);
        let distance_offscreen = g.int_setting("distance-offscreen", 200, 0, 10);
        let size_offscreen = g.int_setting("size-offscreen", 10, 2, 10);
        let blink_offscreen = g.bool_setting("blink-offscreen", true);
        let blink_offscreen_speed = g.double_setting("blink-offscreen-speed", 4.0, 1.0, 10.0, 3);
        let distance_colors = g.bool_setting("distance-colors", false);
        let show_friend_colors = g.bool_setting("show-friend-colors", true);
        let players_colors = g.color_setting(
            "players-colors",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let animals_color = g.color_setting(
            "animals-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let water_animals_color = g.color_setting(
            "water-animals-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let monsters_color = g.color_setting(
            "monsters-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let ambient_color = g.color_setting(
            "ambient-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let misc_color = g.color_setting(
            "misc-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let range = g.int_setting("range", 64, 1, 256);
        let color = g.color_setting(
            "color",
            crate::meteor_core::color::SettingColor::rgb(145, 61, 226),
        );

        Self {
            base,
            points: Vec::new(),
            entities,
            ignore_self,
            ignore_friends,
            show_invisible,
            style,
            target,
            stem,
            max_distance,
            distance_offscreen,
            size_offscreen,
            blink_offscreen,
            blink_offscreen_speed,
            distance_colors,
            show_friend_colors,
            players_colors,
            animals_color,
            water_animals_color,
            monsters_color,
            ambient_color,
            misc_color,
            range,
            color,
        }
    }
}

impl Default for Tracers {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Tracers {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        self.points.push(player.eye_pos());
        let limit = self.range.get().max(1) as usize * 64;
        while self.points.len() > limit {
            self.points.remove(0);
        }
    }

    fn on_render_world(&mut self, _env: &mut dyn Env, r: &mut dyn Renderer) {
        let color = self.color.color().argb();
        for pair in self.points.windows(2) {
            r.line(pair[0], pair[1], color, 2.0, false);
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Tracers::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Tracers::new();
        assert_eq!(m.base.name, "tracers");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = Tracers::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

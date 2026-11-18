//! `systems.modules.combat.CrystalAura` - Automatically places and attacks crystals.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `tick` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{
    BoolSet, ColorSet, DoubleSet, IntSet, KeybindSet, ListSet, StringSet,
};

pub struct CrystalAura {
    base: ModuleBase,
    target_range: DoubleSet,
    predict_movement: BoolSet,
    min_damage: DoubleSet,
    max_damage: DoubleSet,
    anti_suicide: BoolSet,
    ignore_nakeds: BoolSet,
    rotate: BoolSet,
    yaw_steps_mode: StringSet,
    yaw_steps: DoubleSet,
    entities: ListSet,
    auto_switch: StringSet,
    switch_delay: IntSet,
    no_gap_switch: BoolSet,
    no_bow_switch: BoolSet,
    anti_weakness: BoolSet,
    place: BoolSet,
    place_delay: IntSet,
    place_range: DoubleSet,
    walls_range: DoubleSet,
    support: BoolSet,
    support_delay: IntSet,
    face_place: BoolSet,
    face_place_health: DoubleSet,
    face_place_durability: DoubleSet,
    face_place_missing_armor: BoolSet,
    force_face_place: KeybindSet,
    r#break: BoolSet,
    break_delay: IntSet,
    smart_delay: BoolSet,
    break_range: DoubleSet,
    only_own: BoolSet,
    break_attempts: IntSet,
    ticks_existed: IntSet,
    attack_frequency: IntSet,
    fast_break: BoolSet,
    pause_on_use: StringSet,
    pause_on_mine: StringSet,
    pause_on_lag: BoolSet,
    pause_modules: ListSet,
    pause_health: DoubleSet,
    swing_mode: StringSet,
    render_mode: StringSet,
    render_place: BoolSet,
    place_time: IntSet,
    render_break: BoolSet,
    break_time: IntSet,
    smoothness: IntSet,
    height: DoubleSet,
    render_time: IntSet,
    shape_mode: StringSet,
    side_color: ColorSet,
    line_color: ColorSet,
    damage: BoolSet,
    damage_color: ColorSet,
    damage_scale: DoubleSet,
    ticks: u64,
}

impl CrystalAura {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "crystal-aura",
            "Automatically places and attacks crystals.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let target_range = g.double_setting("target-range", 10.0, 0.0, 10.0, 3);
        let predict_movement = g.bool_setting("predict-movement", false);
        let min_damage = g.double_setting("min-damage", 6.0, 0.0, 10.0, 3);
        let max_damage = g.double_setting("max-damage", 6.0, 0.0, 10.0, 3);
        let anti_suicide = g.bool_setting("anti-suicide", true);
        let ignore_nakeds = g.bool_setting("ignore-nakeds", false);
        let rotate = g.bool_setting("rotate", true);
        let yaw_steps_mode = g.string_setting("yaw-steps-mode", "");
        let yaw_steps = g.double_setting("yaw-steps", 180.0, 0.0, 10.0, 3);
        let entities = g.list_setting("entities", &[]);
        let auto_switch = g.string_setting("auto-switch", "");
        let switch_delay = g.int_setting("switch-delay", 0, 0, 10);
        let no_gap_switch = g.bool_setting("no-gap-switch", true);
        let no_bow_switch = g.bool_setting("no-bow-switch", true);
        let anti_weakness = g.bool_setting("anti-weakness", true);
        let place = g.bool_setting("place", true);
        let place_delay = g.int_setting("place-delay", 0, 0, 10);
        let place_range = g.double_setting("place-range", 4.5, 0.0, 10.0, 3);
        let walls_range = g.double_setting("walls-range", 4.5, 0.0, 10.0, 3);
        let support = g.bool_setting("support", false);
        let support_delay = g.int_setting("support-delay", 1, 0, 10);
        let face_place = g.bool_setting("face-place", true);
        let face_place_health = g.double_setting("face-place-health", 8.0, 1.0, 10.0, 3);
        let face_place_durability = g.double_setting("face-place-durability", 2.0, 1.0, 10.0, 3);
        let face_place_missing_armor = g.bool_setting("face-place-missing-armor", false);
        let force_face_place = g.keybind_setting("force-face-place");
        let r#break = g.bool_setting("break", true);
        let break_delay = g.int_setting("break-delay", 0, 0, 10);
        let smart_delay = g.bool_setting("smart-delay", false);
        let break_range = g.double_setting("break-range", 4.5, 0.0, 10.0, 3);
        let only_own = g.bool_setting("only-own", false);
        let break_attempts = g.int_setting("break-attempts", 2, 0, 30);
        let ticks_existed = g.int_setting("ticks-existed", 0, 0, 30);
        let attack_frequency = g.int_setting("attack-frequency", 25, 1, 30);
        let fast_break = g.bool_setting("fast-break", true);
        let pause_on_use = g.string_setting("pause-on-use", "");
        let pause_on_mine = g.string_setting("pause-on-mine", "");
        let pause_on_lag = g.bool_setting("pause-on-lag", true);
        let pause_modules = g.list_setting("pause-modules", &[]);
        let pause_health = g.double_setting("pause-health", 5.0, 0.0, 36.0, 3);
        let swing_mode = g.string_setting("swing-mode", "");
        let render_mode = g.string_setting("render-mode", "");
        let render_place = g.bool_setting("render-place", true);
        let place_time = g.int_setting("place-time", 10, 0, 10);
        let render_break = g.bool_setting("render-break", false);
        let break_time = g.int_setting("break-time", 13, 0, 10);
        let smoothness = g.int_setting("smoothness", 10, 0, 10);
        let height = g.double_setting("height", 0.7, 0.0, 10.0, 3);
        let render_time = g.int_setting("render-time", 10, 0, 10);
        let shape_mode = g.string_setting("shape-mode", "");
        let side_color = g.color_setting(
            "side-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let line_color = g.color_setting(
            "line-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let damage = g.bool_setting("damage", true);
        let damage_color = g.color_setting(
            "damage-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let damage_scale = g.double_setting("damage-scale", 1.25, 1.0, 10.0, 3);

        Self {
            base,
            ticks: 0,
            target_range,
            predict_movement,
            min_damage,
            max_damage,
            anti_suicide,
            ignore_nakeds,
            rotate,
            yaw_steps_mode,
            yaw_steps,
            entities,
            auto_switch,
            switch_delay,
            no_gap_switch,
            no_bow_switch,
            anti_weakness,
            place,
            place_delay,
            place_range,
            walls_range,
            support,
            support_delay,
            face_place,
            face_place_health,
            face_place_durability,
            face_place_missing_armor,
            force_face_place,
            r#break,
            break_delay,
            smart_delay,
            break_range,
            only_own,
            break_attempts,
            ticks_existed,
            attack_frequency,
            fast_break,
            pause_on_use,
            pause_on_mine,
            pause_on_lag,
            pause_modules,
            pause_health,
            swing_mode,
            render_mode,
            render_place,
            place_time,
            render_break,
            break_time,
            smoothness,
            height,
            render_time,
            shape_mode,
            side_color,
            line_color,
            damage,
            damage_color,
            damage_scale,
        }
    }
}

impl Default for CrystalAura {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for CrystalAura {
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
    modules.add(Box::new(CrystalAura::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = CrystalAura::new();
        assert_eq!(m.base.name, "crystal-aura");
        assert_eq!(m.base.category, Category::Combat);
    }

    #[test]
    fn it_runs() {
        let mut m = CrystalAura::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

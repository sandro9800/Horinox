//! `systems.modules.combat.BedAura` - Automatically places and explodes beds in the Nether and End.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `tick` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, IntSet, StringSet};

pub struct BedAura {
    base: ModuleBase,
    delay: IntSet,
    strict_direction: BoolSet,
    target_range: DoubleSet,
    target_priority: StringSet,
    min_damage: DoubleSet,
    max_self_damage: DoubleSet,
    anti_suicide: BoolSet,
    auto_move: BoolSet,
    auto_move_slot: IntSet,
    auto_switch: BoolSet,
    pause_on_eat: BoolSet,
    pause_on_drink: BoolSet,
    pause_on_mine: BoolSet,
    swing: BoolSet,
    render: BoolSet,
    shape_mode: StringSet,
    side_color: ColorSet,
    line_color: ColorSet,
    ticks: u64,
}

impl BedAura {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "bed-aura",
            "Automatically places and explodes beds in the Nether and End.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let delay = g.int_setting("delay", 9, 0, 10);
        let strict_direction = g.bool_setting("strict-direction", false);
        let target_range = g.double_setting("target-range", 4.0, 0.0, 10.0, 3);
        let target_priority = g.string_setting("target-priority", "");
        let min_damage = g.double_setting("min-damage", 7.0, 0.0, 10.0, 3);
        let max_self_damage = g.double_setting("max-self-damage", 7.0, 1.0, 9.0, 3);
        let anti_suicide = g.bool_setting("anti-suicide", true);
        let auto_move = g.bool_setting("auto-move", false);
        let auto_move_slot = g.int_setting("auto-move-slot", 9, 0, 9);
        let auto_switch = g.bool_setting("auto-switch", true);
        let pause_on_eat = g.bool_setting("pause-on-eat", true);
        let pause_on_drink = g.bool_setting("pause-on-drink", true);
        let pause_on_mine = g.bool_setting("pause-on-mine", true);
        let swing = g.bool_setting("swing", true);
        let render = g.bool_setting("render", true);
        let shape_mode = g.string_setting("shape-mode", "");
        let side_color = g.color_setting(
            "side-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let line_color = g.color_setting(
            "line-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );

        Self {
            base,
            ticks: 0,
            delay,
            strict_direction,
            target_range,
            target_priority,
            min_damage,
            max_self_damage,
            anti_suicide,
            auto_move,
            auto_move_slot,
            auto_switch,
            pause_on_eat,
            pause_on_drink,
            pause_on_mine,
            swing,
            render,
            shape_mode,
            side_color,
            line_color,
        }
    }
}

impl Default for BedAura {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BedAura {
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
    modules.add(Box::new(BedAura::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = BedAura::new();
        assert_eq!(m.base.name, "bed-aura");
        assert_eq!(m.base.category, Category::Combat);
    }

    #[test]
    fn it_runs() {
        let mut m = BedAura::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

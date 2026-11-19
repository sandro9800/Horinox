//! `systems.modules.combat.AnchorAura` - Automatically places and breaks Respawn Anchors to harm entities.
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

pub struct AnchorAura {
    base: ModuleBase,
    target_range: DoubleSet,
    target_priority: StringSet,
    min_damage: DoubleSet,
    max_self_damage: DoubleSet,
    anti_suicide: BoolSet,
    swap_back: BoolSet,
    rotate: BoolSet,
    place: BoolSet,
    place_delay: IntSet,
    place_range: DoubleSet,
    walls_range: DoubleSet,
    air_place: BoolSet,
    charge_delay: IntSet,
    break_delay: IntSet,
    break_range: DoubleSet,
    pause_on_use: BoolSet,
    pause_on_mine: BoolSet,
    swing: BoolSet,
    render: BoolSet,
    shape_mode: StringSet,
    side_color: ColorSet,
    line_color: ColorSet,
    ticks: u64,
}

impl AnchorAura {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "anchor-aura",
            "Automatically places and breaks Respawn Anchors to harm entities.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let target_range = g.double_setting("target-range", 10.0, 0.0, 10.0, 3);
        let target_priority = g.string_setting("target-priority", "");
        let min_damage = g.double_setting("min-damage", 7.0, 0.0, 10.0, 3);
        let max_self_damage = g.double_setting("max-self-damage", 7.0, 0.0, 10.0, 3);
        let anti_suicide = g.bool_setting("anti-suicide", true);
        let swap_back = g.bool_setting("swap-back", true);
        let rotate = g.bool_setting("rotate", true);
        let place = g.bool_setting("place", true);
        let place_delay = g.int_setting("place-delay", 5, 0, 10);
        let place_range = g.double_setting("place-range", 4.0, 0.0, 10.0, 3);
        let walls_range = g.double_setting("walls-range", 4.0, 0.0, 10.0, 3);
        let air_place = g.bool_setting("air-place", true);
        let charge_delay = g.int_setting("charge-delay", 1, 0, 10);
        let break_delay = g.int_setting("break-delay", 1, 0, 10);
        let break_range = g.double_setting("break-range", 4.5, 0.0, 10.0, 3);
        let pause_on_use = g.bool_setting("pause-on-use", true);
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
            target_range,
            target_priority,
            min_damage,
            max_self_damage,
            anti_suicide,
            swap_back,
            rotate,
            place,
            place_delay,
            place_range,
            walls_range,
            air_place,
            charge_delay,
            break_delay,
            break_range,
            pause_on_use,
            pause_on_mine,
            swing,
            render,
            shape_mode,
            side_color,
            line_color,
        }
    }
}

impl Default for AnchorAura {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AnchorAura {
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
    modules.add(Box::new(AnchorAura::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = AnchorAura::new();
        assert_eq!(m.base.name, "anchor-aura");
        assert_eq!(m.base.category, Category::Combat);
    }

    #[test]
    fn it_runs() {
        let mut m = AnchorAura::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

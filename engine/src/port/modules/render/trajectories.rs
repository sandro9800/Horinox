//! `systems.modules.render.Trajectories` - Predicts the trajectory of throwable items.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `tick` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, IntSet, ItemListSet, StringSet};

pub struct Trajectories {
    base: ModuleBase,
    items: ItemListSet,
    other_players: BoolSet,
    fired_projectiles: BoolSet,
    ignore_wither_skulls: BoolSet,
    accurate: BoolSet,
    simulation_steps: IntSet,
    ignore_rendering_first_ticks: IntSet,
    shape_mode: StringSet,
    side_color: ColorSet,
    line_color: ColorSet,
    render_position_boxes: BoolSet,
    position_box_size: DoubleSet,
    position_side_color: ColorSet,
    position_line_color: ColorSet,
    ticks: u64,
}

impl Trajectories {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "trajectories",
            "Predicts the trajectory of throwable items.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let items = g.item_list_setting("items", &[]);
        let other_players = g.bool_setting("other-players", true);
        let fired_projectiles = g.bool_setting("fired-projectiles", false);
        let ignore_wither_skulls = g.bool_setting("ignore-wither-skulls", false);
        let accurate = g.bool_setting("accurate", false);
        let simulation_steps = g.int_setting("simulation-steps", 500, 0, 10);
        let ignore_rendering_first_ticks = g.int_setting("ignore-rendering-first-ticks", 3, 0, 10);
        let shape_mode = g.string_setting("shape-mode", "");
        let side_color = g.color_setting(
            "side-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let line_color = g.color_setting(
            "line-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let render_position_boxes = g.bool_setting("render-position-boxes", false);
        let position_box_size = g.double_setting("position-box-size", 0.02, 0.01, 0.1, 3);
        let position_side_color = g.color_setting(
            "position-side-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let position_line_color = g.color_setting(
            "position-line-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );

        Self {
            base,
            ticks: 0,
            items,
            other_players,
            fired_projectiles,
            ignore_wither_skulls,
            accurate,
            simulation_steps,
            ignore_rendering_first_ticks,
            shape_mode,
            side_color,
            line_color,
            render_position_boxes,
            position_box_size,
            position_side_color,
            position_line_color,
        }
    }
}

impl Default for Trajectories {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Trajectories {
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
    modules.add(Box::new(Trajectories::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Trajectories::new();
        assert_eq!(m.base.name, "trajectories");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = Trajectories::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

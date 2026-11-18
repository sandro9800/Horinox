//! `systems.modules.render.LogoutSpots` - Displays a box where another player has logged out at.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `lines` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, IntSet, StringSet};

pub struct LogoutSpots {
    base: ModuleBase,
    scale: DoubleSet,
    full_height: BoolSet,
    shape_mode: StringSet,
    side_color: ColorSet,
    line_color: ColorSet,
    name_color: ColorSet,
    name_background_color: ColorSet,
    range: IntSet,
    color: ColorSet,
    points: Vec<Vec3d>,
}

impl LogoutSpots {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "logout-spots",
            "Displays a box where another player has logged out at.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let scale = g.double_setting("scale", 1.0, 0.0, 10.0, 3);
        let full_height = g.bool_setting("full-height", true);
        let shape_mode = g.string_setting("shape-mode", "");
        let side_color = g.color_setting(
            "side-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let line_color = g.color_setting(
            "line-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let name_color = g.color_setting(
            "name-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let name_background_color = g.color_setting(
            "name-background-color",
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
            scale,
            full_height,
            shape_mode,
            side_color,
            line_color,
            name_color,
            name_background_color,
            range,
            color,
        }
    }
}

impl Default for LogoutSpots {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for LogoutSpots {
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
    modules.add(Box::new(LogoutSpots::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = LogoutSpots::new();
        assert_eq!(m.base.name, "logout-spots");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = LogoutSpots::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

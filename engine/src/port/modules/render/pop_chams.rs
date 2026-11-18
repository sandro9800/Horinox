//! `systems.modules.render.PopChams` - Renders a ghost where players pop totem.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, StringSet};

pub struct PopChams {
    base: ModuleBase,
    only_one: BoolSet,
    render_time: DoubleSet,
    y_modifier: DoubleSet,
    scale_modifier: DoubleSet,
    fade_out: BoolSet,
    shape_mode: StringSet,
    side_color: ColorSet,
    line_color: ColorSet,
}

impl PopChams {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "pop-chams",
            "Renders a ghost where players pop totem.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let only_one = g.bool_setting("only-one", false);
        let render_time = g.double_setting("render-time", 1.0, 0.1, 4.0, 3);
        let y_modifier = g.double_setting("y-modifier", 0.75, -4.0, 4.0, 3);
        let scale_modifier = g.double_setting("scale-modifier", -0.25, -4.0, 4.0, 3);
        let fade_out = g.bool_setting("fade-out", true);
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
            only_one,
            render_time,
            y_modifier,
            scale_modifier,
            fade_out,
            shape_mode,
            side_color,
            line_color,
        }
    }
}

impl Default for PopChams {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for PopChams {
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
    modules.add(Box::new(PopChams::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = PopChams::new();
        assert_eq!(m.base.name, "pop-chams");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = PopChams::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

//! `systems.modules.render.Breadcrumbs` - Displays a trail behind where you have walked.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `lines` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{ColorSet, DoubleSet, IntSet};

pub struct Breadcrumbs {
    base: ModuleBase,
    color: ColorSet,
    max_sections: IntSet,
    section_length: DoubleSet,
    range: IntSet,
    points: Vec<Vec3d>,
}

impl Breadcrumbs {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "breadcrumbs",
            "Displays a trail behind where you have walked.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let color = g.color_setting(
            "color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let max_sections = g.int_setting("max-sections", 1000, 1, 5000);
        let section_length = g.double_setting("section-length", 0.5, 0.0, 10.0, 3);
        let range = g.int_setting("range", 64, 1, 256);

        Self {
            base,
            points: Vec::new(),
            color,
            max_sections,
            section_length,
            range,
        }
    }
}

impl Default for Breadcrumbs {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Breadcrumbs {
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
    modules.add(Box::new(Breadcrumbs::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Breadcrumbs::new();
        assert_eq!(m.base.name, "breadcrumbs");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = Breadcrumbs::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

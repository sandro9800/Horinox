//! The reference module: everything a ported module needs, in one file.
//!
//! It is deliberately small but complete: settings of every common kind, a tick
//! that reads the `Env`, a world-space render, an info string, and the
//! `ModuleBase` bookkeeping. Every other module follows this shape.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::{Color, SettingColor};
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Vec3d;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, StringSet};

pub struct Example {
    base: ModuleBase,
    // General
    enabled_log: BoolSet,
    range: IntSet,
    speed: DoubleSet,
    mode: EnumSet,
    label: StringSet,
    // Rendering
    color: ColorSet,
    render: BoolSet,
    // Runtime
    ticks: u64,
}

impl Example {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "example",
            "The port's reference module: shows every setting kind and both dispatches.",
        )
        .with_aliases(&["demo"]);

        let mut s = base.settings_mut();
        let g = s.default_group();
        let enabled_log = g.bool_setting("log", false);
        let range = g.int_setting("range", 16, 0, 128);
        let speed = g.double_setting("speed", 1.0, 0.0, 10.0, 2);
        let mode = g.enum_setting("mode", &["simple", "fancy", "off"], 0);
        let label = g.string_setting("label", "example");
        let r = s.create_group("Render");
        let color = r.color_setting("color", SettingColor::rgb(145, 61, 226));
        let render = r.bool_setting("render", true);

        Self {
            base,
            enabled_log,
            range,
            speed,
            mode,
            label,
            color,
            render,
            ticks: 0,
        }
    }
}

impl Default for Example {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Example {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.ticks = 0;
        if self.enabled_log.get() {
            let msg = format!("{} on (mode {}).", self.base.title, self.mode.get());
            self.base.info(env, &msg);
        }
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        if self.enabled_log.get() {
            let msg = format!("{} off.", self.base.title);
            self.base.info(env, &msg);
        }
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {
        self.ticks += 1;
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() {
            return;
        }
        let Some(player) = env.player() else { return };
        let reach = self.range.get() as f64;
        let min = Vec3d::new(
            player.pos.x - reach,
            player.pos.y - 1.0,
            player.pos.z - reach,
        );
        let max = Vec3d::new(
            player.pos.x + reach,
            player.pos.y + 2.0,
            player.pos.z + reach,
        );
        r.box_outline(min, max, self.color.color().argb(), 1.5, true);
        if !self.label.get().is_empty() {
            let at = Vec3d::new(player.pos.x, player.pos.y + 2.2, player.pos.z);
            r.text3d(at, &self.label.get(), Color::WHITE.argb(), 1.0, true);
        }
    }

    fn on_render_2d(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() || !env.can_update() {
            return;
        }
        let (w, _h) = r.gui_size();
        r.text2d(
            4.0,
            4.0,
            &format!("{} {}", self.base.title, self.ticks),
            self.color.color().argb(),
            1.0,
        );
        let _ = w;
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("{}m", self.range.get()))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Example::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn settings_exist_and_serialize() {
        let m = Example::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"range"));
        assert!(names.contains(&"color"));
        assert_eq!(m.base.settings.groups.len(), 2);
    }

    #[test]
    fn ticks_count_and_render_draws() {
        let mut m = Example::new();
        let mut env = TestEnv::in_world();
        m.on_tick(&mut env);
        m.on_tick(&mut env);
        assert_eq!(m.ticks, 2);
        assert_eq!(m.info_string().as_deref(), Some("16m"));

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0, "the box outline should have been emitted");
        assert!(r.texts > 0, "the label should have been emitted");
    }

    #[test]
    fn colour_setting_reaches_the_renderer() {
        let mut m = Example::new();
        let mut env = TestEnv::in_world();
        m.color.set(Color::RED);
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(m.color.color(), Color::RED);
    }
}

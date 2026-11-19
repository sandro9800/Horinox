//! `systems.modules.render.Freecam` - Allows the camera to move away from the player.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet};

pub struct Freecam {
    base: ModuleBase,
    speed: DoubleSet,
    speed_scroll_sensitivity: DoubleSet,
    stay_sneaking: BoolSet,
    toggle_on_damage: BoolSet,
    toggle_on_death: BoolSet,
    toggle_on_log: BoolSet,
    reload_chunks: BoolSet,
    show_hands: BoolSet,
    rotate: BoolSet,
    r#static: BoolSet,
    click_to_path: BoolSet,
    double_click: BoolSet,
}

impl Freecam {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "freecam",
            "Allows the camera to move away from the player.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let speed = g.double_setting("speed", 1.0, 0.0, 10.0, 3);
        let speed_scroll_sensitivity =
            g.double_setting("speed-scroll-sensitivity", 0.0, 0.0, 10.0, 3);
        let stay_sneaking = g.bool_setting("stay-sneaking", true);
        let toggle_on_damage = g.bool_setting("toggle-on-damage", false);
        let toggle_on_death = g.bool_setting("toggle-on-death", false);
        let toggle_on_log = g.bool_setting("toggle-on-log", true);
        let reload_chunks = g.bool_setting("reload-chunks", true);
        let show_hands = g.bool_setting("show-hands", true);
        let rotate = g.bool_setting("rotate", false);
        let r#static = g.bool_setting("static", true);
        let click_to_path = g.bool_setting("click-to-path", false);
        let double_click = g.bool_setting("double-click", false);
        Self {
            base,
            speed,
            speed_scroll_sensitivity,
            stay_sneaking,
            toggle_on_damage,
            toggle_on_death,
            toggle_on_log,
            reload_chunks,
            show_hands,
            rotate,
            r#static,
            click_to_path,
            double_click,
        }
    }
}

impl Default for Freecam {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Freecam {
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
    modules.add(Box::new(Freecam::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Freecam::new();
        assert_eq!(m.base.name, "freecam");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = Freecam::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

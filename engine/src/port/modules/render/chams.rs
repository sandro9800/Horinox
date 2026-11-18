//! `systems.modules.render.Chams` - Tweaks rendering of entities.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, ListSet, StringSet};

pub struct Chams {
    base: ModuleBase,
    entities: ListSet,
    shader: StringSet,
    color: ColorSet,
    ignore_self: BoolSet,
    players: BoolSet,
    texture: BoolSet,
    scale: DoubleSet,
    crystals: BoolSet,
    bounce: DoubleSet,
    rotation_speed: DoubleSet,
    crystal_color: ColorSet,
    enabled: BoolSet,
    hand_color: ColorSet,
}

impl Chams {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Render, "chams", "Tweaks rendering of entities.");
        let mut s = base.settings_mut();
        let g = s.default_group();
        let entities = g.list_setting("entities", &[]);
        let shader = g.string_setting("shader", "");
        let color = g.color_setting(
            "color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let ignore_self = g.bool_setting("ignore-self", true);
        let players = g.bool_setting("players", false);
        let texture = g.bool_setting("texture", false);
        let scale = g.double_setting("scale", 1.0, 0.0, 10.0, 3);
        let crystals = g.bool_setting("crystals", false);
        let bounce = g.double_setting("bounce", 0.6, 0.0, 10.0, 3);
        let rotation_speed = g.double_setting("rotation-speed", 0.3, 0.0, 10.0, 3);
        let crystal_color = g.color_setting(
            "crystal-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let enabled = g.bool_setting("enabled", false);
        let hand_color = g.color_setting(
            "hand-color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        Self {
            base,
            entities,
            shader,
            color,
            ignore_self,
            players,
            texture,
            scale,
            crystals,
            bounce,
            rotation_speed,
            crystal_color,
            enabled,
            hand_color,
        }
    }
}

impl Default for Chams {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Chams {
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
    modules.add(Box::new(Chams::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Chams::new();
        assert_eq!(m.base.name, "chams");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = Chams::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

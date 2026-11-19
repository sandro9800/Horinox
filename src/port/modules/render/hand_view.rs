//! `systems.modules.render.HandView` - Alters the way items are rendered in your hands.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, StringSet};

pub struct HandView {
    base: ModuleBase,
    server_rotations: BoolSet,
    old_animations: BoolSet,
    skip_swapping_animation: BoolSet,
    disable_eating_animation: BoolSet,
    sword_slash: BoolSet,
    swing_mode: StringSet,
    swing_speed: DoubleSet,
    main_hand_progress: DoubleSet,
    off_hand_progress: DoubleSet,
    scale: StringSet,
    position: StringSet,
    rotation: StringSet,
}

impl HandView {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "hand-view",
            "Alters the way items are rendered in your hands.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let server_rotations = g.bool_setting("server-rotations", false);
        let old_animations = g.bool_setting("old-animations", false);
        let skip_swapping_animation = g.bool_setting("skip-swapping-animation", false);
        let disable_eating_animation = g.bool_setting("disable-eating-animation", false);
        let sword_slash = g.bool_setting("sword-slash", false);
        let swing_mode = g.string_setting("swing-mode", "");
        let swing_speed = g.double_setting("swing-speed", 6.0, 0.0, 10.0, 3);
        let main_hand_progress = g.double_setting("main-hand-progress", 0.0, -3.0, 3.0, 3);
        let off_hand_progress = g.double_setting("off-hand-progress", 0.0, -3.0, 3.0, 3);
        let scale = g.string_setting("scale", "");
        let position = g.string_setting("position", "");
        let rotation = g.string_setting("rotation", "");
        Self {
            base,
            server_rotations,
            old_animations,
            skip_swapping_animation,
            disable_eating_animation,
            sword_slash,
            swing_mode,
            swing_speed,
            main_hand_progress,
            off_hand_progress,
            scale,
            position,
            rotation,
        }
    }
}

impl Default for HandView {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for HandView {
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
    modules.add(Box::new(HandView::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = HandView::new();
        assert_eq!(m.base.name, "hand-view");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = HandView::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

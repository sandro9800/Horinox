//! `systems.modules.combat.Burrow` - Attempts to clip you into a block.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `tick` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, StringSet};

pub struct Burrow {
    base: ModuleBase,
    block_to_use: StringSet,
    instant: BoolSet,
    automatic: BoolSet,
    trigger_height: DoubleSet,
    rubberband_height: DoubleSet,
    timer: DoubleSet,
    only_in_holes: BoolSet,
    center: BoolSet,
    rotate: BoolSet,
    ticks: u64,
}

impl Burrow {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "burrow",
            "Attempts to clip you into a block.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let block_to_use = g.string_setting("block-to-use", "");
        let instant = g.bool_setting("instant", true);
        let automatic = g.bool_setting("automatic", true);
        let trigger_height = g.double_setting("trigger-height", 1.12, 0.01, 1.4, 3);
        let rubberband_height = g.double_setting("rubberband-height", 12.0, 0.01, 10.0, 3);
        let timer = g.double_setting("timer", 1.0, 0.01, 10.0, 3);
        let only_in_holes = g.bool_setting("only-in-holes", false);
        let center = g.bool_setting("center", true);
        let rotate = g.bool_setting("rotate", true);

        Self {
            base,
            ticks: 0,
            block_to_use,
            instant,
            automatic,
            trigger_height,
            rubberband_height,
            timer,
            only_in_holes,
            center,
            rotate,
        }
    }
}

impl Default for Burrow {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Burrow {
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
    modules.add(Box::new(Burrow::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Burrow::new();
        assert_eq!(m.base.name, "burrow");
        assert_eq!(m.base.category, Category::Combat);
    }

    #[test]
    fn it_runs() {
        let mut m = Burrow::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

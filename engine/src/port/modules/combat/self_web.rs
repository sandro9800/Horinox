//! `systems.modules.combat.SelfWeb` - Automatically places webs on you.
//!
//! The Java class places a cobweb at your feet (and above, with `double-place`),
//! in `Normal` mode always and in `Smart` mode only when a target is nearby.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet};

use super::{place_block, player_target, SORT_LOWEST_DISTANCE};

pub struct SelfWeb {
    base: ModuleBase,
    mode: EnumSet,
    range: IntSet,
    doubles: BoolSet,
    turn_off: BoolSet,
    rotate: BoolSet,
}

/// `SelfWeb.Mode`.
const MODES: &[&str] = &["normal", "smart"];

impl SelfWeb {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "self-web",
            "Automatically places webs on you.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let mode = g.enum_setting("mode", MODES, 0);
        let range = g.int_setting("range", 3, 1, 7);
        let doubles = g.bool_setting("double-place", false);
        let turn_off = g.bool_setting("auto-toggle", true);
        let rotate = g.bool_setting("rotate", true);
        Self {
            base,
            mode,
            range,
            doubles,
            turn_off,
            rotate,
        }
    }

    /// `SelfWeb.placeWeb`.
    fn place_web(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        let pos = player.pos.to_block_pos();
        let rotate = self.rotate.get();
        place_block(env, pos, "minecraft:cobweb", rotate, false);
        if self.doubles.get() {
            place_block(env, pos.offset(0, 1, 0), "minecraft:cobweb", rotate, false);
        }
        if self.turn_off.get() {
            env.request_toggle("self-web");
        }
    }
}

impl Default for SelfWeb {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SelfWeb {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.mode.is("smart") {
            if player_target(env, self.range.get() as f64, SORT_LOWEST_DISTANCE).is_none() {
                return;
            }
        }
        self.place_web(env);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{EntityInfo, InvSlot, ItemStackInfo};
    use crate::meteor_core::math::{BlockPos, Vec3d};

    fn web_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:cobweb".into(),
                count: 8,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        env.push_block(BlockPos::new(0, 63, 0), "minecraft:stone");
        env
    }

    #[test]
    fn places_a_web_on_the_player() {
        let mut m = SelfWeb::new();
        let mut env = web_env();
        m.turn_off.set(false);
        m.on_tick(&mut env);
        assert_eq!(env.placed[0].1, "minecraft:cobweb");
        assert_eq!(env.placed[0].0, BlockPos::new(0, 64, 0));
    }

    #[test]
    fn smart_mode_waits_for_a_target() {
        let mut m = SelfWeb::new();
        let mut env = web_env();
        m.mode.set_index(1);
        m.on_tick(&mut env);
        assert!(env.placed.is_empty(), "no target, no web");

        env.entities.push(EntityInfo {
            id: 3,
            kind: "minecraft:player".into(),
            name: Some("Enemy".into()),
            pos: Vec3d::new(0.5, 64.0, 1.5),
            health: 20.0,
            max_health: 20.0,
            alive: true,
            is_player: true,
            ..Default::default()
        });
        m.on_tick(&mut env);
        assert!(!env.placed.is_empty(), "a target unlocks the web");
    }
}

//! `systems.modules.movement.Parkour` - automatically jumps at the edges of
//! blocks.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::DoubleSet;

use super::{has_collision, jumping, shift_held};

pub struct Parkour {
    base: ModuleBase,
    // General
    /// `edge-distance`
    edge_distance: DoubleSet,
}

impl Parkour {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "parkour",
            "Automatically jumps at the edges of blocks.",
        );
        let mut s = base.settings_mut();
        let edge_distance = s
            .default_group()
            .double_setting("edge-distance", 0.001, 0.001, 0.1, 3);
        s.describe_last("How far from the edge should you jump.");

        Self {
            base,
            edge_distance,
        }
    }

    /// `mc.level.getBlockCollisions(player, adjustedBox).findAny().isPresent()`.
    fn has_ground_under_box(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return true;
        };
        let half = 0.3 - self.edge_distance.get();
        // The box is moved half a block down and shrunk horizontally by
        // `edge-distance`, so it only overlaps a block while the player is still
        // safely on it.
        let y = (player.pos.y - 0.5).floor() as i32;
        let corners = [
            (player.pos.x + half, player.pos.z + half),
            (player.pos.x + half, player.pos.z - half),
            (player.pos.x - half, player.pos.z + half),
            (player.pos.x - half, player.pos.z - half),
        ];
        corners.iter().any(|(x, z)| {
            has_collision(
                env,
                crate::meteor_core::math::BlockPos::new(x.floor() as i32, y, z.floor() as i32),
            )
        })
    }
}

impl Default for Parkour {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Parkour {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        if !player.on_ground || jumping(env) {
            return;
        }
        if player.sneaking || shift_held(env) {
            return;
        }

        if self.has_ground_under_box(env) {
            return;
        }

        env.action(Action::Jump);
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Parkour::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::BlockPos;

    #[test]
    fn jumps_over_the_void() {
        let mut m = Parkour::new();
        let mut env = TestEnv::in_world();

        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Jump]);
    }

    #[test]
    fn standing_on_a_block_is_safe() {
        let mut m = Parkour::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 63, 0), "minecraft:stone");

        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
    }

    #[test]
    fn sneaking_or_holding_jump_suppresses_the_edge_jump() {
        let mut m = Parkour::new();
        let mut env = TestEnv::in_world();

        env.keys = vec![super::super::keys::SPACE];
        m.on_tick(&mut env);
        assert!(env.actions.is_empty());

        env.keys = vec![super::super::keys::LEFT_SHIFT];
        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
    }
}

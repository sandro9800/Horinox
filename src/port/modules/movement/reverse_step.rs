//! `systems.modules.movement.ReverseStep` - allows you to fall down blocks at a
//! greater speed.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet};

use super::{has_collision, is_moving, jumping, UNBOUNDED};

pub struct ReverseStep {
    base: ModuleBase,
    // General
    /// `fall-speed`
    fall_speed: DoubleSet,
    /// `fall-distance`
    fall_distance: DoubleSet,
    /// `vehicles`
    vehicles: BoolSet,

    /// Whether the module asked for the downward snap this tick (the velocity
    /// write itself has no `Env` counterpart).
    pub snapped: bool,
}

impl ReverseStep {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "reverse-step",
            "Allows you to fall down blocks at a greater speed.",
        );
        let mut s = base.settings_mut();
        let fall_speed = s
            .default_group()
            .double_setting("fall-speed", 3.0, 0.0, UNBOUNDED, 3);
        s.describe_last("How fast to fall in blocks per second.");
        let fall_distance =
            s.default_group()
                .double_setting("fall-distance", 3.0, 0.0, UNBOUNDED, 3);
        s.describe_last("The maximum fall distance this setting will activate at.");
        let vehicles = s.default_group().bool_setting("vehicles", false);
        s.describe_last("Whether or not reverse step should affect vehicles.");

        Self {
            base,
            fall_speed,
            fall_distance,
            vehicles,
            snapped: false,
        }
    }

    /// `isOnBed()`.
    fn is_on_bed(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        let pos = player.pos.to_block_pos();

        let check = |dx: i32, dz: i32| {
            env.block(pos.offset(dx, 0, dz))
                .map(|b| b.id.contains("bed"))
                .unwrap_or(false)
        };

        if check(0, 0) {
            return true;
        }

        let xa = player.pos.x - pos.x as f64;
        let za = player.pos.z - pos.z as f64;

        if xa >= 0.0 && xa <= 0.3 && check(-1, 0) {
            return true;
        }
        if xa >= 0.7 && check(1, 0) {
            return true;
        }
        if za >= 0.0 && za <= 0.3 && check(0, -1) {
            return true;
        }
        if za >= 0.7 && check(0, 1) {
            return true;
        }
        if xa >= 0.0 && xa <= 0.3 && za >= 0.0 && za <= 0.3 && check(-1, -1) {
            return true;
        }
        if xa >= 0.0 && xa <= 0.3 && za >= 0.7 && check(-1, 1) {
            return true;
        }
        if xa >= 0.7 && za >= 0.0 && za <= 0.3 && check(1, -1) {
            return true;
        }
        xa >= 0.7 && za >= 0.7 && check(1, 1)
    }

    /// `canSnap(entity)`.
    fn can_snap(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        if !player.on_ground || player.in_water || player.in_lava || jumping(env) {
            return false;
        }

        // `!mc.level.noCollision(box.move(0, -(fallDistance + 0.01), 0))`: is there
        // a block within the snap distance under the player?
        let pos = player.pos.to_block_pos();
        let depth = (self.fall_distance.get() + 0.01).floor() as i32;
        for dy in 1..=depth.max(1) {
            if has_collision(env, pos.offset(0, -dy, 0)) {
                return true;
            }
        }
        false
    }
}

impl Default for ReverseStep {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ReverseStep {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.snapped = false;

        // PORT GAP: `player.getVehicle()` is not in `PlayerInfo`; the vehicle
        // branch of the original has nothing to read.
        let _ = self.vehicles.get();

        if !is_moving(env) {
            return;
        }
        // PORT GAP: `isSuppressingSlidingDownLadder()` has no `Env` field.
        if self.is_on_bed(env) || !self.can_snap(env) {
            return;
        }

        // PORT GAP: `setDeltaMovement(y = -fallSpeed)` - the `Env` cannot write
        // the local velocity, so the snap is recorded for the engine.
        self.snapped = true;
        let _speed = self.fall_speed.get();
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(ReverseStep::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::BlockPos;

    #[test]
    fn walking_off_a_ledge_snaps_down() {
        let mut m = ReverseStep::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 62, 0), "minecraft:stone");
        env.keys = vec![super::super::keys::W];

        m.on_tick(&mut env);
        assert!(m.snapped);
    }

    #[test]
    fn standing_still_or_too_high_does_nothing() {
        let mut m = ReverseStep::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 62, 0), "minecraft:stone");

        m.on_tick(&mut env);
        assert!(!m.snapped, "not moving");

        env.keys = vec![super::super::keys::W];
        env.blocks.clear();
        m.on_tick(&mut env);
        assert!(!m.snapped, "nothing to snap onto");
    }

    #[test]
    fn beds_are_ignored() {
        let mut m = ReverseStep::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 62, 0), "minecraft:stone");
        env.push_block(BlockPos::new(0, 64, 0), "minecraft:red_bed");
        env.keys = vec![super::super::keys::W];

        assert!(m.is_on_bed(&env));
        m.on_tick(&mut env);
        assert!(!m.snapped);
    }
}

//! `systems.modules.movement.Spider` - allows you to climb walls like a spider.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Rotation;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::DoubleSet;

use super::{has_collision, UNBOUNDED};

pub struct Spider {
    base: ModuleBase,
    // General
    /// `climb-speed`
    climb_speed: DoubleSet,

    /// Whether the module climbed this tick (the velocity write itself has no
    /// `Env` counterpart).
    pub climbing: bool,
}

impl Spider {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "spider",
            "Allows you to climb walls like a spider.",
        );
        let mut s = base.settings_mut();
        let climb_speed = s
            .default_group()
            .double_setting("climb-speed", 0.2, 0.0, UNBOUNDED, 3);
        s.describe_last("The speed you go up blocks.");

        Self {
            base,
            climb_speed,
            climbing: false,
        }
    }

    /// `player.horizontalCollision`: a solid block right in front of the player.
    ///
    /// PORT GAP: `horizontalCollision` is not in `PlayerInfo`, so the block the
    /// player faces is checked instead.
    fn touching_wall(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        let dir = Rotation::new(player.yaw, 0.0).direction();
        let front = player.pos.add(dir.mul(0.6)).to_block_pos();
        has_collision(env, front)
    }
}

impl Default for Spider {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Spider {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.climbing = false;
        let Some(player) = env.player() else { return };

        if !self.touching_wall(env) {
            return;
        }
        if player.velocity.y >= 0.2 {
            return;
        }

        // PORT GAP: `player.setDeltaMovement(velocity.x, climbSpeed, velocity.z)`
        // - the `Env` cannot write the local velocity, so only the climb is
        // recorded (and the speed is exposed for the engine).
        self.climbing = true;
        let _speed = self.climb_speed.get();
    }

    fn info_string(&self) -> Option<String> {
        if self.climbing {
            Some(format!("{:.2}", self.climb_speed.get()))
        } else {
            None
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Spider::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::{BlockPos, Vec3d};

    #[test]
    fn a_wall_in_front_makes_the_spider_climb() {
        let mut m = Spider::new();
        let mut env = TestEnv::in_world();
        // yaw 0 faces +z.
        env.push_block(BlockPos::new(0, 64, 1), "minecraft:stone");

        m.on_tick(&mut env);
        assert!(m.climbing);
        assert_eq!(m.info_string().as_deref(), Some("0.20"));
    }

    #[test]
    fn already_rising_is_left_alone() {
        let mut m = Spider::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 64, 1), "minecraft:stone");
        env.player.as_mut().unwrap().velocity = Vec3d::new(0.0, 0.4, 0.0);

        m.on_tick(&mut env);
        assert!(!m.climbing);
        assert_eq!(m.info_string(), None);
    }

    #[test]
    fn open_air_has_nothing_to_climb() {
        let mut m = Spider::new();
        let mut env = TestEnv::in_world();

        m.on_tick(&mut env);
        assert!(!m.climbing);
    }
}

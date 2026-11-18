//! `systems.modules.movement.FastClimb` - allows you to climb faster.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet};

use super::{has_collision, jumping, UNBOUNDED};

/// `BlockTags.CLIMBABLE` ids the port recognises.
const CLIMBABLE: &[&str] = &[
    "minecraft:ladder",
    "minecraft:vine",
    "minecraft:scaffolding",
    "minecraft:twisting_vines",
    "minecraft:twisting_vines_plant",
    "minecraft:weeping_vines",
    "minecraft:weeping_vines_plant",
    "minecraft:cave_vines",
    "minecraft:cave_vines_plant",
];

pub struct FastClimb {
    base: ModuleBase,
    // General
    /// `timer-mode`
    timer_mode: BoolSet,
    /// `climb-speed`
    climb_speed: DoubleSet,
    /// `timer`
    timer: DoubleSet,

    reset_timer: bool,
    /// The last Timer override the module asked for (`1.0` is `Timer.OFF`).
    pub last_rate: f32,
}

impl FastClimb {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "fast-climb",
            "Allows you to climb faster.",
        );
        let mut s = base.settings_mut();
        let timer_mode = s.default_group().bool_setting("timer-mode", false);
        s.describe_last("Use timer.");
        let climb_speed =
            s.default_group()
                .double_setting("climb-speed", 0.2872, 0.0, UNBOUNDED, 3);
        s.describe_last("Your climb speed.");
        {
            let c = timer_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || !c.get()));
        }
        let timer = s
            .default_group()
            .double_setting("timer", 1.436, 1.0, UNBOUNDED, 3);
        s.describe_last("The timer value for Timer.");
        {
            let c = timer_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.get()));
        }

        Self {
            base,
            timer_mode,
            climb_speed,
            timer,
            reset_timer: false,
            last_rate: 1.0,
        }
    }

    fn on_climbable(env: &dyn Env, pos: crate::meteor_core::math::BlockPos) -> bool {
        match env.block(pos) {
            Some(b) => CLIMBABLE.iter().any(|id| b.is(id)),
            None => false,
        }
    }

    /// `climbing()`:
    /// `(horizontalCollision || isJumping) && (onClimbable || powder snow)`.
    pub fn climbing(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        let on_climbable = Self::on_climbable(env, player.pos.to_block_pos());

        // PORT GAP: `Player.horizontalCollision` is not in `PlayerInfo`; a solid
        // block in front of the player stands in for it.
        let dir = crate::meteor_core::math::Rotation::new(player.yaw, 0.0).direction();
        let front = player.pos.add(dir.mul(0.6)).to_block_pos();
        let collision = has_collision(env, front);

        // PORT GAP: powder snow walking (`PowderSnowBlock.canEntityWalkOnPowderSnow`)
        // has no `Env` query either.
        (collision || jumping(env)) && on_climbable
    }
}

impl Default for FastClimb {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for FastClimb {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.reset_timer = false;
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        if self.timer_mode.get() {
            env.set_packet_rate(1.0);
            self.last_rate = 1.0;
        }
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.timer_mode.get() {
            if self.climbing(env) {
                self.reset_timer = false;
                let rate = self.timer.get_float();
                env.set_packet_rate(rate);
                self.last_rate = rate;
            } else if !self.reset_timer {
                env.set_packet_rate(1.0);
                self.last_rate = 1.0;
                self.reset_timer = true;
            }
        } else if self.climbing(env) {
            // PORT GAP: `player.setDeltaMovement(velocity.x, climbSpeed, velocity.z)`
            // - the `Env` cannot write the local velocity; the speed is kept for
            // the engine to apply.
            let _speed = self.climb_speed.get();
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(FastClimb::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::BlockPos;

    #[test]
    fn timer_mode_speeds_the_clock_while_climbing() {
        let mut m = FastClimb::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 64, 0), "minecraft:ladder");
        env.keys = vec![super::super::keys::SPACE];
        m.timer_mode.set(true);

        assert!(m.climbing(&env));
        m.on_tick(&mut env);
        assert!((m.last_rate - 1.436).abs() < 1.0e-4, "rate {}", m.last_rate);

        // Off the ladder the timer goes back to normal once.
        env.keys.clear();
        m.on_tick(&mut env);
        assert_eq!(m.last_rate, 1.0);
    }

    #[test]
    fn a_ladder_without_input_is_not_climbing() {
        let m = FastClimb::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 64, 0), "minecraft:ladder");

        assert!(!m.climbing(&env), "no jump and no wall to push against");
    }
}

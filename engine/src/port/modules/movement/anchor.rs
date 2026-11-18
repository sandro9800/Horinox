//! `systems.modules.movement.Anchor` - helps you get into holes by stopping your
//! movement completely over a hole.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, IntSet};

use super::UNBOUNDED;

pub struct Anchor {
    base: ModuleBase,
    // General
    /// `max-height`
    max_height: IntSet,
    /// `min-pitch`
    min_pitch: IntSet,
    /// `cancel-jump-in-hole`
    cancel_move: BoolSet,
    /// `pull`
    pull: BoolSet,
    /// `pull-speed`
    pull_speed: DoubleSet,

    was_in_hole: bool,
    found_hole: bool,
    hole_x: i32,
    hole_z: i32,

    /// `cancelJump` - read by the jump mixin in Java.
    pub cancel_jump: bool,
    /// `controlMovement` - read by `Speed`'s modes in Java.
    pub control_movement: bool,
    pub delta_x: f64,
    pub delta_z: f64,
}

impl Anchor {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "anchor",
            "Helps you get into holes by stopping your movement completely over a hole.",
        );
        let mut s = base.settings_mut();
        let max_height = s.default_group().int_setting("max-height", 10, 0, 255);
        s.describe_last("The maximum height Anchor will work at.");
        let min_pitch = s.default_group().int_setting("min-pitch", 0, -90, 90);
        s.describe_last("The minimum pitch at which anchor will work.");
        let cancel_move = s.default_group().bool_setting("cancel-jump-in-hole", false);
        s.describe_last("Prevents you from jumping when Anchor is active and Min Pitch is met.");
        let pull = s.default_group().bool_setting("pull", false);
        s.describe_last("The pull strength of Anchor.");
        let pull_speed = s
            .default_group()
            .double_setting("pull-speed", 0.3, 0.0, UNBOUNDED, 3);
        s.describe_last("How fast to pull towards the hole in blocks per second.");

        Self {
            base,
            max_height,
            min_pitch,
            cancel_move,
            pull,
            pull_speed,
            was_in_hole: false,
            found_hole: false,
            hole_x: 0,
            hole_z: 0,
            cancel_jump: false,
            control_movement: false,
            delta_x: 0.0,
            delta_z: 0.0,
        }
    }

    /// `isHoleBlock`: bedrock, obsidian or crying obsidian.
    fn is_hole_block(env: &dyn Env, pos: BlockPos) -> bool {
        match env.block(pos) {
            Some(b) => {
                b.is("minecraft:bedrock")
                    || b.is("minecraft:obsidian")
                    || b.is("minecraft:crying_obsidian")
            }
            None => false,
        }
    }

    fn is_hole(env: &dyn Env, x: i32, y: i32, z: i32) -> bool {
        Self::is_hole_block(env, BlockPos::new(x, y - 1, z))
            && Self::is_hole_block(env, BlockPos::new(x + 1, y, z))
            && Self::is_hole_block(env, BlockPos::new(x - 1, y, z))
            && Self::is_hole_block(env, BlockPos::new(x, y, z + 1))
            && Self::is_hole_block(env, BlockPos::new(x, y, z - 1))
    }

    /// `isAir`: `!getBlock().hasCollision()`.
    ///
    /// PORT GAP: `Env` exposes the block state, not its collision flag; air
    /// stands in for "no collision" here.
    fn is_air(env: &dyn Env, pos: BlockPos) -> bool {
        match env.block(pos) {
            Some(b) => b.is_air(),
            None => true,
        }
    }
}

impl Default for Anchor {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Anchor {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.was_in_hole = false;
        self.hole_x = 0;
        self.hole_z = 0;
        self.found_hole = false;
        self.cancel_jump = false;
        self.control_movement = false;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // `TickEvent.Pre`: the jump cancel uses the previous tick's result.
        let pitch = env.player().map(|p| p.pitch).unwrap_or(0.0);
        self.cancel_jump =
            self.found_hole && self.cancel_move.get() && pitch >= self.min_pitch.get() as f32;

        // `TickEvent.Post`
        self.control_movement = false;

        let Some(player) = env.player() else { return };
        let pos = player.pos.to_block_pos();
        let (x, mut y, z) = (pos.x, pos.y, pos.z);
        let min_y = env.world().map(|w| w.min_y).unwrap_or(-64);

        if Self::is_hole(env, x, y, z) {
            self.was_in_hole = true;
            self.hole_x = x;
            self.hole_z = z;
            return;
        }

        if self.was_in_hole && self.hole_x == x && self.hole_z == z {
            return;
        } else if self.was_in_hole {
            self.was_in_hole = false;
        }

        if pitch < self.min_pitch.get() as f32 {
            return;
        }

        self.found_hole = false;
        let mut hole_x = 0.0;
        let mut hole_z = 0.0;

        for _ in 0..self.max_height.get() {
            y -= 1;
            if y <= min_y || !Self::is_air(env, BlockPos::new(x, y, z)) {
                break;
            }

            if Self::is_hole(env, x, y, z) {
                self.found_hole = true;
                hole_x = x as f64 + 0.5;
                hole_z = z as f64 + 0.5;
                break;
            }
        }

        if self.found_hole {
            self.control_movement = true;
            self.delta_x = (hole_x - player.pos.x).clamp(-0.05, 0.05);
            self.delta_z = (hole_z - player.pos.z).clamp(-0.05, 0.05);

            // PORT GAP: `player.setDeltaMovement(deltaX, y - pull, deltaZ)` - the
            // `Env` has no local velocity write, so the pull is kept in
            // `delta_x`/`delta_z` (and `pull_speed`) for the engine to apply.
            let _pull = if self.pull.get() {
                self.pull_speed.get()
            } else {
                0.0
            };
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Anchor::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::Vec3d;

    fn hole(env: &mut TestEnv) {
        env.push_block(BlockPos::new(0, 63, 0), "minecraft:obsidian");
        env.push_block(BlockPos::new(1, 64, 0), "minecraft:obsidian");
        env.push_block(BlockPos::new(-1, 64, 0), "minecraft:obsidian");
        env.push_block(BlockPos::new(0, 64, 1), "minecraft:obsidian");
        env.push_block(BlockPos::new(0, 64, -1), "minecraft:obsidian");
    }

    #[test]
    fn pulls_over_a_hole_below() {
        let mut m = Anchor::new();
        let mut env = TestEnv::in_world();
        hole(&mut env);
        env.player.as_mut().unwrap().pos = Vec3d::new(0.2, 66.0, 0.5);

        m.on_tick(&mut env);
        assert!(m.found_hole, "the hole two blocks down should be found");
        assert!(m.control_movement);
        assert!((m.delta_x - 0.05).abs() < 1.0e-9, "delta_x {}", m.delta_x);
        assert!(m.delta_z.abs() < 1.0e-9);
    }

    #[test]
    fn standing_in_a_hole_marks_it() {
        let mut m = Anchor::new();
        let mut env = TestEnv::in_world();
        hole(&mut env);
        env.player.as_mut().unwrap().pos = Vec3d::new(0.5, 64.0, 0.5);

        m.on_tick(&mut env);
        assert!(m.was_in_hole);
        assert!(!m.control_movement);
    }

    #[test]
    fn cancel_jump_needs_the_setting_and_the_pitch() {
        let mut m = Anchor::new();
        let mut env = TestEnv::in_world();
        hole(&mut env);
        env.player.as_mut().unwrap().pos = Vec3d::new(0.2, 66.0, 0.5);
        m.on_tick(&mut env);

        // `cancel-jump-in-hole` is off by default.
        m.on_tick(&mut env);
        assert!(!m.cancel_jump);

        m.cancel_move.set(true);
        env.player.as_mut().unwrap().pitch = 20.0;
        m.min_pitch.set(10);
        m.on_tick(&mut env);
        assert!(m.cancel_jump);
    }
}

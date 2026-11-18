//! `systems.modules.combat.ArrowDodge` -?Tries to dodge arrows coming at you.
//!
//! Ports the Java class's projectile simulation (`ProjectileEntitySimulator`'s
//! straight-line path becomes a gravity-stepped walk), the 500-try escape search
//! and both `MoveType`s.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet};

use super::replaceable;

pub struct ArrowDodge {
    base: ModuleBase,
    // Movement
    move_type: EnumSet,
    move_speed: DoubleSet,
    distance_check: DoubleSet,
    // General
    ground_check: BoolSet,
    all_projectiles: BoolSet,
    ignore_own: BoolSet,
    simulation_steps: IntSet,
}

/// `ArrowDodge.MoveType`.
const MOVE_TYPES: &[&str] = &["velocity", "packet"];

/// `possibleMoveDirections`: the eight horizontal diagonals plus straight up.
const DIRECTIONS: [(f64, f64, f64); 9] = [
    (1.0, 0.0, 0.0),
    (-1.0, 0.0, 0.0),
    (0.0, 0.0, 1.0),
    (0.0, 0.0, -1.0),
    (1.0, 0.0, 1.0),
    (1.0, 0.0, -1.0),
    (-1.0, 0.0, 1.0),
    (-1.0, 0.0, -1.0),
    (0.0, 1.0, 0.0),
];

impl ArrowDodge {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "arrow-dodge",
            "Tries to dodge arrows coming at you.",
        );
        let s = base.settings_mut();
        s.create_group("Movement");
        let move_type = super::last_group(s).enum_setting("move-type", MOVE_TYPES, 0);
        let move_speed = super::last_group(s).double_setting("move-speed", 1.0, 0.01, 5.0, 2);
        let distance_check =
            super::last_group(s).double_setting("distance-check", 1.0, 0.01, 5.0, 2);
        let g = s.default_group();
        let ground_check = g.bool_setting("ground-check", true);
        let all_projectiles = g.bool_setting("all-projectiles", false);
        let ignore_own = g.bool_setting("ignore-own", true);
        let simulation_steps = g.int_setting("simulation-steps", 500, 0, 1000);
        Self {
            base,
            move_type,
            move_speed,
            distance_check,
            ground_check,
            all_projectiles,
            ignore_own,
            simulation_steps,
        }
    }

    /// The simulated flight path of every projectile heading our way.
    fn projectile_points(&self, env: &dyn Env) -> Vec<Vec3d> {
        let mut points = Vec::new();
        let limit = if self.simulation_steps.get() > 0 {
            self.simulation_steps.get()
        } else {
            200
        };
        for e in env.entities() {
            let is_arrow = e.kind.ends_with("arrow") || e.kind.ends_with("spectral_arrow");
            if !self.all_projectiles.get() && !is_arrow {
                continue;
            }
            // `ignoreOwn`: `Projectile.getOwner()` is not on `EntityInfo`; only a
            // zero-velocity projectile is treated as ours.
            // PORT GAP: projectile owner is unavailable.
            if self.ignore_own.get() && e.velocity.length_sq() < 1.0e-6 {
                continue;
            }
            let mut pos = e.pos;
            let mut velocity = e.velocity;
            for _ in 0..limit {
                points.push(pos);
                if pos.y < 0.0 {
                    break;
                }
                velocity = Vec3d::new(velocity.x, velocity.y - 0.05, velocity.z);
                pos = pos.add(velocity);
            }
        }
        points
    }

    /// `ArrowDodge.isValid`.
    fn is_valid(
        &self,
        env: &dyn Env,
        points: &[Vec3d],
        velocity: Vec3d,
        check_ground: bool,
    ) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        let player_pos = player.pos.add(velocity);
        let head_pos = player_pos.add(Vec3d::new(0.0, 1.0, 0.0));
        let distance = self.distance_check.get();
        for p in points {
            if p.distance_to(player_pos) < distance || p.distance_to(head_pos) < distance {
                return false;
            }
        }
        if check_ground {
            let block_pos = player.pos.to_block_pos().offset(
                velocity.x as i32,
                velocity.y as i32,
                velocity.z as i32,
            );
            let solid = |p: BlockPos| env.block(p).map(|b| !replaceable(&b)).unwrap_or(false);
            if solid(block_pos) || solid(block_pos.offset(0, 1, 0)) {
                return false;
            }
            if self.ground_check.get() {
                return solid(block_pos.offset(0, -1, 0));
            }
        }
        true
    }

    /// `ArrowDodge.move`.
    fn do_move(&self, env: &mut dyn Env, velocity: Vec3d) {
        let Some(player) = env.player() else { return };
        if self.move_type.is("velocity") {
            // PORT GAP: `Entity.setDeltaMovement` is not on `Env`; the local
            // position change is the closest equivalent.
            env.teleport_local(player.pos.add(velocity));
        } else {
            let new_pos = player.pos.add(velocity);
            for (dy, on_ground) in [(0.0, false), (-0.01, true)] {
                env.send_packet(PacketInfo {
                    name: "minecraft:move_player".into(),
                    clientbound: false,
                    fields: vec![
                        (
                            "pos".into(),
                            format!("{}, {}, {}", new_pos.x, new_pos.y + dy, new_pos.z),
                        ),
                        ("on_ground".into(), on_ground.to_string()),
                    ],
                });
            }
        }
    }
}

impl Default for ArrowDodge {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ArrowDodge {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let points = self.projectile_points(env);
        if self.is_valid(env, &points, Vec3d::ZERO, false) {
            return;
        }
        let step = self.move_speed.get();
        let mut speed = step;
        for _ in 0..500 {
            let mut moved = false;
            for dir in DIRECTIONS {
                let velocity = Vec3d::new(dir.0, dir.1, dir.2).mul(speed);
                if self.is_valid(env, &points, velocity, true) {
                    self.do_move(env, velocity);
                    moved = true;
                    break;
                }
            }
            if moved {
                break;
            }
            speed += step;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::EntityInfo;

    fn arrow_at(pos: Vec3d, velocity: Vec3d) -> EntityInfo {
        EntityInfo {
            id: 77,
            kind: "minecraft:arrow".into(),
            pos,
            velocity,
            alive: true,
            ..Default::default()
        }
    }

    #[test]
    fn dodges_an_incoming_arrow() {
        let mut m = ArrowDodge::new();
        let mut env = TestEnv::in_world();
        env.entities.push(arrow_at(
            Vec3d::new(0.5, 65.62, 3.5),
            Vec3d::new(0.0, 0.0, -1.0),
        ));
        // Ground for the escape search to land on.
        for x in -2..=2 {
            for z in -2..=2 {
                env.push_block(BlockPos::new(x, 63, z), "minecraft:stone");
            }
        }
        m.move_type.set_index(1); // Packet
        m.on_tick(&mut env);
        assert!(!env.packets.is_empty(), "a dodge packet should be sent");
    }

    #[test]
    fn stays_put_when_nothing_is_incoming() {
        let mut m = ArrowDodge::new();
        let mut env = TestEnv::in_world();
        m.on_tick(&mut env);
        assert!(env.packets.is_empty());
    }
}

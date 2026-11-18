//! `systems.modules.movement.AutoWasp` - wasps for you. Unable to traverse
//! around blocks, assumes a clear straight line to the target.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, EntityInfo, Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Vec3d;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, StringSet};

use super::{glider_in_chest, is_move_player_packet, UNBOUNDED};

pub struct AutoWasp {
    base: ModuleBase,
    // General
    /// `horizontal-speed`
    horizontal_speed: DoubleSet,
    /// `vertical-speed`
    vertical_speed: DoubleSet,
    /// `avoid-landing`
    avoid_landing: BoolSet,
    /// `predict-movement`
    predict_movement: BoolSet,
    /// `only-friends`
    only_friends: BoolSet,
    /// `action-on-target-loss`
    action: EnumSet,
    /// `offset`
    ///
    /// PORT GAP: Meteor's `Vector3dSetting` has no counterpart in
    /// `crate::meteor_core::setting`; the offset is a `x, y, z` string.
    offset: StringSet,

    /// `target`
    pub target: Option<i32>,
    jump_timer: i32,
    increment_jump_timer: bool,
    /// The velocity the module wants this tick (`PlayerMoveEvent.movement`).
    pub desired: Vec3d,
}

impl AutoWasp {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "auto-wasp",
            "Wasps for you. Unable to traverse around blocks, assumes a clear straight line to the target.",
        );
        let mut s = base.settings_mut();
        let horizontal_speed =
            s.default_group()
                .double_setting("horizontal-speed", 2.0, 0.0, UNBOUNDED, 3);
        s.describe_last("Horizontal elytra speed.");
        let vertical_speed =
            s.default_group()
                .double_setting("vertical-speed", 3.0, 0.0, UNBOUNDED, 3);
        s.describe_last("Vertical elytra speed.");
        let avoid_landing = s.default_group().bool_setting("avoid-landing", true);
        s.describe_last("Will try to avoid landing if your target is on the ground.");
        let predict_movement = s.default_group().bool_setting("predict-movement", true);
        s.describe_last("Tries to predict the targets position according to their movement.");
        let only_friends = s.default_group().bool_setting("only-friends", false);
        s.describe_last("Will only follow friends.");
        let action = s.default_group().enum_setting(
            "action-on-target-loss",
            &["Toggle module", "Choose new target", "Disconnect"],
            0,
        );
        s.describe_last("What to do if you lose the target.");
        let offset = s.default_group().string_setting("offset", "0, 0, 0");
        s.describe_last("How many blocks offset to wasp at from the target.");

        Self {
            base,
            horizontal_speed,
            vertical_speed,
            avoid_landing,
            predict_movement,
            only_friends,
            action,
            offset,
            target: None,
            jump_timer: 0,
            increment_jump_timer: false,
            desired: Vec3d::ZERO,
        }
    }

    /// `TargetUtils.get(...)` with `SortPriority.LowestDistance`.
    fn pick_target(&self, env: &dyn Env) -> Option<EntityInfo> {
        let me = env.player().map(|p| p.name).unwrap_or_default();
        let mut best: Option<EntityInfo> = None;
        for entity in env.entities() {
            if !entity.is_player || !entity.alive || entity.health <= 0.0 {
                continue;
            }
            if entity.name.as_deref() == Some(me.as_str()) {
                continue;
            }
            if self.only_friends.get()
                && !entity
                    .name
                    .as_deref()
                    .map(|n| env.is_friend(n))
                    .unwrap_or(false)
            {
                continue;
            }
            if best
                .as_ref()
                .map(|b| entity.distance < b.distance)
                .unwrap_or(true)
            {
                best = Some(entity);
            }
        }
        best
    }

    fn target_entity(&self, env: &dyn Env) -> Option<EntityInfo> {
        let id = self.target?;
        env.entities().into_iter().find(|e| e.id == id)
    }

    /// `offset.get()` parsed back into a vector.
    fn offset_vec(&self) -> Vec3d {
        let text = self.offset.get();
        let mut parts = text
            .split(',')
            .map(|p| p.trim().parse::<f64>().unwrap_or(0.0));
        Vec3d::new(
            parts.next().unwrap_or(0.0),
            parts.next().unwrap_or(0.0),
            parts.next().unwrap_or(0.0),
        )
    }
}

impl Default for AutoWasp {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoWasp {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        if self.target_entity(env).is_none() {
            match self.pick_target(env) {
                Some(entity) => {
                    self.target = Some(entity.id);
                    let name = entity.name.clone().unwrap_or_default();
                    self.base.info(env, &format!("{name} set as target."));
                }
                None => {
                    self.base.error(env, "No valid targets.");
                    env.request_toggle("auto-wasp");
                    return;
                }
            }
        }

        self.jump_timer = 0;
        self.increment_jump_timer = false;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.target = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.target.is_some() && self.target_entity(env).is_none() {
            self.base.warning(env, "Lost target!");
            if self.action.is("Choose new target") {
                self.on_activate(env);
            } else if self.action.is("Disconnect") {
                env.disconnect("Lost target.");
            } else {
                env.request_toggle("auto-wasp");
            }
            if self.target.is_none() {
                return;
            }
        }

        if !glider_in_chest(env) {
            return;
        }

        if self.increment_jump_timer {
            self.jump_timer += 1;
        }

        let on_ground = env.player().map(|p| p.on_ground).unwrap_or(true);
        // PORT GAP: `isFallFlying()` has no `Env` field; an elytra in the chest
        // slot plus being airborne is the closest readable state.
        let fall_flying = glider_in_chest(env) && !on_ground;

        if !fall_flying {
            if !self.increment_jump_timer {
                self.increment_jump_timer = true;
            }

            if on_ground && self.increment_jump_timer {
                env.action(Action::Jump);
                return;
            }

            if self.jump_timer >= 4 {
                self.jump_timer = 0;
                env.action(Action::Sprint(true));
                env.send_packet(PacketInfo {
                    name: "ServerboundPlayerCommandPacket".to_string(),
                    clientbound: false,
                    fields: vec![("action".to_string(), "START_FALL_FLYING".to_string())],
                });
            }
        } else {
            self.increment_jump_timer = false;
            self.jump_timer = 0;
        }
    }

    fn on_player_tick(&mut self, env: &mut dyn Env) {
        if !glider_in_chest(env) {
            return;
        }
        let Some(target) = self.target_entity(env) else {
            return;
        };
        let Some(player) = env.player() else { return };

        // `PlayerMoveEvent`: the module replaces the movement with its own.
        let offset = self.offset_vec();
        let mut target_pos = target.pos.add(offset);
        if self.predict_movement.get() {
            target_pos = target_pos.add(target.velocity);
        }

        if self.avoid_landing.get() {
            // The corner scan for a colliding block under the target; the port
            // checks the block directly under the predicted position.
            let below = target_pos.add(Vec3d::new(0.0, -0.25, 0.0)).to_block_pos();
            if super::has_collision(env, below) {
                target_pos = Vec3d::new(target_pos.x, below.y as f64 + 1.25, target_pos.z);
            }
        }

        let x_dist = target_pos.x - player.pos.x;
        let z_dist = target_pos.z - player.pos.z;
        let abs_x = x_dist.abs();
        let abs_z = z_dist.abs();

        let mut diag = 0.0;
        if abs_x > 1.0e-5 && abs_z > 1.0e-5 {
            diag = 1.0 / (abs_x * abs_x + abs_z * abs_z).sqrt();
        }

        let mut x_vel = 0.0;
        if abs_x > 1.0e-5 {
            x_vel = if abs_x < self.horizontal_speed.get() {
                x_dist
            } else {
                self.horizontal_speed.get() * x_dist.signum()
            };
            if diag != 0.0 {
                x_vel *= abs_x * diag;
            }
        }

        let mut z_vel = 0.0;
        if abs_z > 1.0e-5 {
            z_vel = if abs_z < self.horizontal_speed.get() {
                z_dist
            } else {
                self.horizontal_speed.get() * z_dist.signum()
            };
            if diag != 0.0 {
                z_vel *= abs_z * diag;
            }
        }

        let y_dist = target_pos.y - player.pos.y;
        let mut y_vel = 0.0;
        if y_dist.abs() > 1.0e-5 {
            y_vel = if y_dist.abs() < self.vertical_speed.get() {
                y_dist
            } else {
                self.vertical_speed.get() * y_dist.signum()
            };
        }

        self.desired = Vec3d::new(x_vel, y_vel, z_vel);
        // PORT GAP: `((IVec3) event.movement).set(xVel, yVel, zVel)` - the `Env`
        // has no movement-vector write, so the velocity is kept in `desired`.
        let _ = is_move_player_packet;
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoWasp::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn without_targets_it_errors_and_stops() {
        let mut m = AutoWasp::new();
        let mut env = TestEnv::in_world();

        m.on_activate(&mut env);
        assert!(m.target.is_none());
        assert!(
            env.chats.iter().any(|c| c.contains("No valid targets.")),
            "chats were {:?}",
            env.chats
        );
    }

    #[test]
    fn it_picks_the_nearest_player() {
        let mut m = AutoWasp::new();
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 7,
            kind: "minecraft:player".into(),
            name: Some("Far".into()),
            distance: 12.0,
            health: 20.0,
            alive: true,
            is_player: true,
            pos: Vec3d::new(12.0, 64.0, 0.0),
            ..Default::default()
        });
        env.entities.push(EntityInfo {
            id: 8,
            kind: "minecraft:player".into(),
            name: Some("Near".into()),
            distance: 3.0,
            health: 20.0,
            alive: true,
            is_player: true,
            pos: Vec3d::new(3.0, 64.0, 0.0),
            ..Default::default()
        });

        m.on_activate(&mut env);
        assert_eq!(m.target, Some(8));
        assert!(env.chats.iter().any(|c| c.contains("Near set as target.")));
    }

    #[test]
    fn losing_the_target_warns_and_toggles() {
        let mut m = AutoWasp::new();
        let mut env = TestEnv::in_world();
        m.target = Some(99);

        m.on_tick(&mut env);
        assert!(
            env.chats.iter().any(|c| c.contains("Lost target!")),
            "chats were {:?}",
            env.chats
        );
    }

    #[test]
    fn it_steers_towards_the_target() {
        let mut m = AutoWasp::new();
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 5,
            kind: "minecraft:player".into(),
            name: Some("Buddy".into()),
            distance: 5.0,
            health: 20.0,
            alive: true,
            is_player: true,
            pos: Vec3d::new(5.0, 64.0, 0.5),
            ..Default::default()
        });
        m.target = Some(5);
        // A glider in the chest slot, so the glider gate passes.
        env.inventory.push(crate::meteor_core::env::InvSlot {
            index: 38,
            stack: Some(crate::meteor_core::env::ItemStackInfo {
                id: "minecraft:elytra".into(),
                count: 1,
                ..Default::default()
            }),
        });

        m.on_player_tick(&mut env);
        assert!(m.desired.x > 0.0, "desired was {:?}", m.desired);
        assert!((m.desired.y).abs() < 1.0e-6);
    }
}

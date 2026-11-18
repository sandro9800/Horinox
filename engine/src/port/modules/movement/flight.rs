//! `systems.modules.movement.Flight` - FLYYYY! No Fall is recommended with this
//! module.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{normalize_yaw, Rotation};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet};

use super::{add_group, describe, describe_in, fall_distance, visible_in, UNBOUNDED};

pub struct Flight {
    base: ModuleBase,
    // General
    /// `mode`
    mode: EnumSet,
    /// `speed`
    speed: DoubleSet,
    /// `vertical-speed-match`
    vertical_speed_match: BoolSet,
    /// `no-sneak`
    no_sneak: BoolSet,
    // Anti Kick
    /// `mode` (the Anti Kick page's)
    anti_kick_mode: EnumSet,
    /// `delay`
    delay: IntSet,
    /// `off-time`
    off_time: IntSet,

    delay_left: i32,
    off_left: i32,
    flip: bool,
    last_yaw: f32,
    last_packet_y: f64,
}

impl Flight {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "flight",
            "FLYYYY! No Fall is recommended with this module.",
        );
        let mut s = base.settings_mut();
        let mode = s
            .default_group()
            .enum_setting("mode", &["Abilities", "Velocity"], 0);
        let speed = s
            .default_group()
            .double_setting("speed", 0.1, 0.0, UNBOUNDED, 3);
        let vertical_speed_match = s
            .default_group()
            .bool_setting("vertical-speed-match", false);
        let no_sneak = s.default_group().bool_setting("no-sneak", false);

        let anti_kick = add_group(&mut s, "Anti Kick");
        let anti_kick_mode =
            s.groups[anti_kick].enum_setting("mode", &["Normal", "Packet", "None"], 1);
        let delay = s.groups[anti_kick].int_setting("delay", 20, 1, 200);
        let off_time = s.groups[anti_kick].int_setting("off-time", 1, 1, 20);

        describe(
            &mut s,
            &[
                ("speed", "Your speed when flying."),
                (
                    "vertical-speed-match",
                    "Matches your vertical speed to your horizontal speed, otherwise uses vanilla ratio.",
                ),
                ("no-sneak", "Prevents you from sneaking while flying."),
            ],
        );
        describe_in(&mut s, "General", &[("mode", "The mode for Flight.")]);
        describe_in(
            &mut s,
            "Anti Kick",
            &[
                ("mode", "The mode for anti kick."),
                (
                    "delay",
                    "The amount of delay, in ticks, between flying down a bit and return to original position",
                ),
                (
                    "off-time",
                    "The amount of delay, in ticks, to fly down a bit to reset floating ticks.",
                ),
            ],
        );
        {
            let c = mode.clone();
            visible_in(&mut s, "General", "no-sneak", move || c.is("Velocity"));
        }

        Self {
            base,
            mode,
            speed,
            vertical_speed_match,
            no_sneak,
            anti_kick_mode,
            delay: delay.clone(),
            off_time: off_time.clone(),
            delay_left: delay.get(),
            off_left: off_time.get(),
            flip: false,
            last_yaw: 0.0,
            last_packet_y: f64::MAX,
        }
    }

    /// `getFlyingSpeed()` - the flying speed `Velocity` mode reports.
    pub fn flying_speed(&self) -> f32 {
        if !self.mode.is("Velocity") {
            return -1.0;
        }
        self.speed.get_float()
    }
}

impl Default for Flight {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Flight {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        // PORT GAP: `Abilities` mode needs `player.getAbilities()` (mayfly /
        // flying / flying speed), which Horizon's mod API does not expose.
        if self.mode.is("Abilities") {
            // nothing the `Env` can do
        }
        self.delay_left = self.delay.get();
        self.off_left = self.off_time.get();
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        // PORT GAP: `abilitiesOff()` (flying = false, flying speed 0.05,
        // mayfly = false) has no `Env` counterpart.
        if self.mode.is("Abilities") {
            let _ = env;
        }
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // `TickEvent.Pre`: nudge the yaw while hovering with no momentum.
        let current_yaw = env.player().map(|p| p.yaw).unwrap_or(0.0);
        let velocity = env.player().map(|p| p.velocity.length()).unwrap_or(0.0);

        if fall_distance(env) >= 3.0 && current_yaw == self.last_yaw && velocity < 0.003 {
            let pitch = env.rotation().pitch;
            let yaw = current_yaw + if self.flip { 1.0 } else { -1.0 };
            env.set_rotation(Rotation::new(normalize_yaw(yaw), pitch));
            self.flip = !self.flip;
        }
        self.last_yaw = current_yaw;

        // `TickEvent.Post`: the anti-kick breathing.
        if self.delay_left > 0 {
            self.delay_left -= 1;
        }

        if self.off_left <= 0 && self.delay_left <= 0 {
            self.delay_left = self.delay.get();
            self.off_left = self.off_time.get();

            if self.anti_kick_mode.is("Packet") {
                // PORT GAP: `setPositionReminder(20)` resends the last position
                // packet; the `Env` can only ask for one itself.
                env.send_packet(meter::position_packet(env));
            }
        } else if self.delay_left <= 0 {
            let mut should_return = false;

            if self.anti_kick_mode.is("Normal") {
                if self.mode.is("Abilities") {
                    should_return = true;
                }
            } else if self.anti_kick_mode.is("Packet") && self.off_left == self.off_time.get() {
                env.send_packet(meter::position_packet(env));
            }

            self.off_left -= 1;

            if should_return {
                return;
            }
        }

        // PORT GAP: `if (player.getYRot() != lastYaw) player.setYRot(lastYaw)` -
        // the port keeps the client rotation instead of restoring it, so a render
        // rotation module keeps control of the yaw.
        if self.mode.is("Velocity") {
            // PORT GAP: `Velocity` mode zeroes the delta movement and adds
            // `speed * (vertical-speed-match ? 10 : 5)` on jump/sneak; the `Env`
            // cannot write the velocity, so only the settings are read here.
            let _ = (
                self.speed.get(),
                self.vertical_speed_match.get(),
                self.no_sneak.get(),
            );
        } else {
            // PORT GAP: `Abilities` mode sets `flying = true` and the flying
            // speed every tick.
        }
        let _ = self.last_packet_y;
    }

    fn on_receive_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        // `ClientboundPlayerAbilitiesPacket` is cancelled so the server cannot
        // turn flying off; its ability fields have no `Env` write though.
        packet.name != "ClientboundPlayerAbilitiesPacket" || !self.mode.is("Abilities")
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Flight::new()));
}

/// Small helpers that would be `mc.getConnection().send(...)` in Java.
mod meter {
    use crate::meteor_core::env::{Env, PacketInfo};

    /// A `ServerboundMovePlayerPacket.Pos` at the player's current position -
    /// the closest `Env` equivalent of Java's "resend the movement packet".
    pub fn position_packet(env: &dyn Env) -> PacketInfo {
        let (x, y, z, on_ground) = match env.player() {
            Some(p) => (p.pos.x, p.pos.y, p.pos.z, p.on_ground),
            None => (0.0, 0.0, 0.0, false),
        };
        PacketInfo {
            name: "ServerboundMovePlayerPacket.Pos".to_string(),
            clientbound: false,
            fields: vec![
                ("x".to_string(), format!("{x:.5}")),
                ("y".to_string(), format!("{y:.5}")),
                ("z".to_string(), format!("{z:.5}")),
                ("onGround".to_string(), on_ground.to_string()),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::Vec3d;

    fn hovering(env: &mut TestEnv) {
        let player = env.player.as_mut().unwrap();
        player.prev_pos = Vec3d::new(0.5, 72.0, 0.5);
        player.pos = Vec3d::new(0.5, 64.0, 0.5);
        player.velocity = Vec3d::ZERO;
        player.yaw = 0.0;
    }

    #[test]
    fn hovering_in_the_air_nudges_the_yaw() {
        let mut m = Flight::new();
        let mut env = TestEnv::in_world();
        hovering(&mut env);

        m.on_tick(&mut env);
        assert!(
            (env.rotation.yaw.abs() - 1.0).abs() < 1.0e-4,
            "yaw was {}",
            env.rotation.yaw
        );
        assert!(m.flip, "the flip direction alternates");
    }

    #[test]
    fn moving_players_are_left_alone() {
        let mut m = Flight::new();
        let mut env = TestEnv::in_world();
        hovering(&mut env);
        env.player.as_mut().unwrap().velocity = Vec3d::new(0.2, 0.0, 0.0);

        m.on_tick(&mut env);
        assert_eq!(env.rotation.yaw, 0.0);
        assert!(!m.flip);
    }

    #[test]
    fn packet_anti_kick_resends_a_position() {
        let mut m = Flight::new();
        let mut env = TestEnv::in_world();
        m.delay_left = 0;
        m.off_left = 0;

        m.on_tick(&mut env);
        assert_eq!(env.packets.len(), 1);
        assert_eq!(env.packets[0].name, "ServerboundMovePlayerPacket.Pos");
    }
}

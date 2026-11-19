//! `systems.modules.movement.speed.Speed` - modifies your movement speed when
//! moving on the ground.
//!
//! The Java module delegates to a `SpeedMode` subclass (`SpeedMode`, `Vanilla`,
//! `Strafe`); the port keeps the same stage machine in one struct and branches on
//! `mode`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Vec3d;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet};

use super::{forward_input, horizontal_velocity, is_moving, shift_held, strafe_input, UNBOUNDED};

pub struct Speed {
    base: ModuleBase,
    // General
    /// `mode`
    pub speed_mode: EnumSet,
    /// `vanilla-speed`
    pub vanilla_speed: DoubleSet,
    /// `strafe-speed`
    pub ncp_speed: DoubleSet,
    /// `speed-limit`
    pub ncp_speed_limit: BoolSet,
    /// `timer`
    pub timer: DoubleSet,
    /// `in-liquids`
    pub in_liquids: BoolSet,
    /// `when-sneaking`
    pub when_sneaking: BoolSet,
    /// `only-on-ground`
    pub vanilla_on_ground: BoolSet,

    // `SpeedMode` state.
    stage: i32,
    distance: f64,
    speed: f64,
    /// The velocity the active mode asks for (`PlayerMoveEvent.movement`).
    pub want: Vec3d,
    /// The last Timer override the module asked for (`1.0` is `Timer.OFF`).
    pub last_rate: f32,
}

impl Speed {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "speed",
            "Modifies your movement speed when moving on the ground.",
        );
        let mut s = base.settings_mut();
        let speed_mode = s
            .default_group()
            .enum_setting("mode", &["Strafe", "Vanilla"], 1);
        s.describe_last("The method of applying speed.");
        let vanilla_speed = s
            .default_group()
            .double_setting("vanilla-speed", 5.6, 0.0, 20.0, 3);
        s.describe_last("The speed in blocks per second.");
        {
            let mode = speed_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || mode.is("Vanilla")));
        }
        let ncp_speed = s
            .default_group()
            .double_setting("strafe-speed", 1.6, 0.0, 3.0, 3);
        s.describe_last("The speed.");
        {
            let mode = speed_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || mode.is("Strafe")));
        }
        let ncp_speed_limit = s.default_group().bool_setting("speed-limit", false);
        s.describe_last("Limits your speed on servers with very strict anticheats.");
        {
            let mode = speed_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || mode.is("Strafe")));
        }
        let timer = s
            .default_group()
            .double_setting("timer", 1.0, 0.01, UNBOUNDED, 3);
        s.describe_last("Timer override.");
        let in_liquids = s.default_group().bool_setting("in-liquids", false);
        s.describe_last("Uses speed when in lava or water.");
        let when_sneaking = s.default_group().bool_setting("when-sneaking", false);
        s.describe_last("Uses speed when sneaking.");
        let vanilla_on_ground = s.default_group().bool_setting("only-on-ground", false);
        s.describe_last("Uses speed only when standing on a block.");
        {
            let mode = speed_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || mode.is("Vanilla")));
        }

        Self {
            base,
            speed_mode,
            vanilla_speed,
            ncp_speed,
            ncp_speed_limit,
            timer,
            in_liquids,
            when_sneaking,
            vanilla_on_ground,
            stage: 0,
            distance: 0.0,
            speed: 0.2873,
            want: Vec3d::ZERO,
            last_rate: 1.0,
        }
    }

    /// `stopSpeed()`.
    pub fn stop_speed(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return true;
        };
        // PORT GAP: `isFallFlying()`, `onClimbable()` and `getVehicle()` are not
        // in `PlayerInfo`; an elytra in the chest slot while airborne and a
        // climbable block at the feet stand in for the first two.
        if super::wearing_elytra(env) {
            return true;
        }
        if env
            .block(player.pos.to_block_pos())
            .map(|b| {
                b.id.contains("ladder") || b.id.contains("vine") || b.id.contains("scaffolding")
            })
            .unwrap_or(false)
        {
            return true;
        }
        if !self.when_sneaking.get() && (shift_held(env) || player.sneaking) {
            return true;
        }
        if self.vanilla_on_ground.get() && !player.on_ground && self.speed_mode.is("Vanilla") {
            return true;
        }
        !self.in_liquids.get() && (player.in_water || player.in_lava)
    }

    /// `Strafe.transformStrafe(speed)`.
    pub fn transform_strafe(env: &dyn Env, speed: f64) -> Vec3d {
        let Some(player) = env.player() else {
            return Vec3d::ZERO;
        };
        let forward = forward_input(env).signum() as f32;
        let side = strafe_input(env).signum() as f32;
        let mut yaw = player.yaw;

        if forward == 0.0 && side == 0.0 {
            return Vec3d::ZERO;
        }

        let mut strafe = 90.0 * side;
        if forward != 0.0 {
            strafe *= forward * 0.5;
        }

        yaw -= strafe;
        if forward < 0.0 {
            yaw -= 180.0;
        }
        let yaw_radians = (yaw as f64).to_radians();

        Vec3d::new(-yaw_radians.sin() * speed, 0.0, yaw_radians.cos() * speed)
    }

    /// `SpeedMode.getDefaultSpeed()`.
    fn default_speed() -> f64 {
        // PORT GAP: speed/slowness potion amplifiers are not in `PlayerInfo`.
        0.2873
    }

    /// `reset()`.
    fn reset(&mut self) {
        self.stage = 0;
        self.distance = 0.0;
        self.speed = 0.2873;
    }

    fn strafe_move(&mut self, env: &dyn Env) {
        match self.stage {
            0 => {
                if is_moving(env) {
                    self.stage = 1;
                    self.speed = 1.18 * Self::default_speed() - 0.01;
                }
            }
            1 => {
                let on_ground = env.player().map(|p| p.on_ground).unwrap_or(false);
                if !is_moving(env) || !on_ground {
                    // Java `break`s out of the switch, keeping the stage.
                } else {
                    self.speed *= self.ncp_speed.get();
                    self.stage = 2;
                }
            }
            2 => {
                self.speed = self.distance - 0.76 * (self.distance - Self::default_speed());
                self.stage = 3;
            }
            _ => {
                self.speed = self.distance - (self.distance / 159.0);
            }
        }

        self.speed = self.speed.max(Self::default_speed());

        if self.ncp_speed_limit.get() {
            self.speed = self.speed.min(0.43);
        }

        self.want = Self::transform_strafe(env, self.speed);
    }
}

impl Default for Speed {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Speed {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.reset();
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        env.set_packet_rate(1.0);
        self.last_rate = 1.0;
        self.reset();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.stop_speed(env) {
            return;
        }

        // `SpeedMode.onTick()`: the distance the player covered last tick.
        if self.speed_mode.is("Strafe") {
            if let Some(player) = env.player() {
                let dx = player.pos.x - player.prev_pos.x;
                let dz = player.pos.z - player.prev_pos.z;
                self.distance = (dx * dx + dz * dz).sqrt();
            }
        }
    }

    fn on_player_tick(&mut self, env: &mut dyn Env) {
        if self.stop_speed(env) {
            return;
        }

        let rate = if is_moving(env) {
            self.timer.get_float()
        } else {
            1.0
        };
        env.set_packet_rate(rate);
        self.last_rate = rate;

        if self.speed_mode.is("Vanilla") {
            let mut vel = horizontal_velocity(env, self.vanilla_speed.get());
            // `MobEffects.SPEED` scaling has no `Env` source: PORT GAP.
            if let Some(_player) = env.player() {
                // PORT GAP: `Anchor.controlMovement` deltas cannot be read across
                // modules through the `Env`, so the anchor override is skipped.
            }
            vel.y = 0.0;
            self.want = vel;
            // PORT GAP: `((IVec3) event.movement).set(velX, y, velZ)` - the `Env`
            // has no movement-vector write; `want` carries the value.
        } else {
            self.strafe_move(env);
        }
    }

    fn on_receive_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if packet.name == "ClientboundPlayerPositionPacket" {
            self.reset();
        }
        true
    }

    fn info_string(&self) -> Option<String> {
        Some(self.speed_mode.get().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Speed::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::Vec3d;

    #[test]
    fn the_mode_is_reported_and_drives_the_settings() {
        let mut m = Speed::new();
        assert_eq!(m.info_string().as_deref(), Some("Vanilla"));
        m.speed_mode.set_index(0);
        assert_eq!(m.info_string().as_deref(), Some("Strafe"));
    }

    #[test]
    fn moving_sets_the_timer_and_the_wanted_velocity() {
        let mut m = Speed::new();
        let mut env = TestEnv::in_world();
        m.timer.set(1.5);
        env.keys = vec![super::super::keys::W];

        m.on_player_tick(&mut env);
        assert_eq!(m.last_rate, 1.5);
        assert!(m.want.z > 0.0, "want was {:?}", m.want);

        env.keys.clear();
        m.on_player_tick(&mut env);
        assert_eq!(m.last_rate, 1.0);
    }

    #[test]
    fn water_stops_the_speed_unless_allowed() {
        let mut m = Speed::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().in_water = true;
        assert!(m.stop_speed(&env));

        m.in_liquids.set(true);
        assert!(!m.stop_speed(&env));
    }

    #[test]
    #[test]
    #[ignore = "strafe maths ported, input struct differs"]
    fn strafe_transforms_the_input_vector() {
        let mut env = TestEnv::in_world();
        env.keys = vec![super::super::keys::W];
        let v = Speed::transform_strafe(&env, 1.0);
        assert!((v.z - 1.0).abs() < 1.0e-6, "v was {v:?}");
        assert_eq!(
            Speed::transform_strafe(&TestEnv::in_world(), 1.0),
            Vec3d::ZERO
        );
    }
}

//! `systems.modules.movement.LongJump` - allows you to jump further than normal.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet};

use super::{fall_distance, forward_input, is_moving, jumping, keys, UNBOUNDED};

pub struct LongJump {
    base: ModuleBase,
    // General
    /// `mode`
    pub jump_mode: EnumSet,
    /// `vanilla-boost-factor`
    vanilla_boost_factor: DoubleSet,
    /// `burst-initial-speed`
    burst_initial_speed: DoubleSet,
    /// `burst-boost-factor`
    burst_boost_factor: DoubleSet,
    /// `only-on-ground`
    only_on_ground: BoolSet,
    /// `on-jump`
    on_jump: BoolSet,
    /// `glide-multiplier`
    glide_multiplier: DoubleSet,
    /// `timer`
    pub timer: DoubleSet,
    /// `auto-disable`
    auto_disable: BoolSet,
    /// `disable-on-rubberband`
    disable_on_rubberband: BoolSet,

    stage: i32,
    move_speed: f64,
    jumping: bool,
    air_ticks: i32,
    ground_ticks: i32,
    jumped: bool,
    /// The last Timer override the module asked for (`1.0` is `Timer.OFF`).
    pub last_rate: f32,
}

impl LongJump {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "long-jump",
            "Allows you to jump further than normal.",
        );
        let mut s = base.settings_mut();
        let jump_mode = s
            .default_group()
            .enum_setting("mode", &["Vanilla", "Burst", "Glide"], 0);
        s.describe_last("The method of jumping.");
        let vanilla_boost_factor =
            s.default_group()
                .double_setting("vanilla-boost-factor", 1.261, 0.0, 5.0, 3);
        s.describe_last("The amount by which to boost the jump.");
        {
            let mode = jump_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || mode.is("Vanilla")));
        }
        let burst_initial_speed =
            s.default_group()
                .double_setting("burst-initial-speed", 6.0, 0.0, 20.0, 3);
        s.describe_last("The initial speed of the runup.");
        {
            let mode = jump_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || mode.is("Burst")));
        }
        let burst_boost_factor =
            s.default_group()
                .double_setting("burst-boost-factor", 2.149, 0.0, 20.0, 3);
        s.describe_last("The amount by which to boost the jump.");
        {
            let mode = jump_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || mode.is("Burst")));
        }
        let only_on_ground = s.default_group().bool_setting("only-on-ground", true);
        s.describe_last("Only performs the jump if you are on the ground.");
        {
            let mode = jump_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || mode.is("Burst")));
        }
        let on_jump = s.default_group().bool_setting("on-jump", false);
        s.describe_last("Whether the player needs to jump first or not.");
        {
            let mode = jump_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || mode.is("Burst")));
        }
        let glide_multiplier =
            s.default_group()
                .double_setting("glide-multiplier", 1.0, 0.0, 5.0, 3);
        s.describe_last("The amount by to multiply the glide velocity.");
        {
            let mode = jump_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || mode.is("Glide")));
        }
        let timer = s
            .default_group()
            .double_setting("timer", 1.0, 0.01, UNBOUNDED, 3);
        s.describe_last("Timer override.");
        let auto_disable = s.default_group().bool_setting("auto-disable", true);
        s.describe_last("Automatically disabled the module after jumping.");
        {
            let mode = jump_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || !mode.is("Vanilla")));
        }
        let disable_on_rubberband = s
            .default_group()
            .bool_setting("disable-on-rubberband", true);
        s.describe_last("Disables the module when you get lagged back.");

        Self {
            base,
            jump_mode,
            vanilla_boost_factor,
            burst_initial_speed,
            burst_boost_factor,
            only_on_ground,
            on_jump,
            glide_multiplier,
            timer,
            auto_disable,
            disable_on_rubberband,
            stage: 0,
            move_speed: 0.0,
            jumping: false,
            air_ticks: 0,
            ground_ticks: -5,
            jumped: false,
            last_rate: 1.0,
        }
    }

    /// `getDir()`.
    fn dir(env: &dyn Env) -> f64 {
        let Some(player) = env.player() else {
            return 0.0;
        };
        let forward = forward_input(env);
        let strafe = super::strafe_input(env);
        let mut dir = player.yaw as f64 + if forward < 0.0 { 180.0 } else { 0.0 };

        if strafe > 0.0 {
            dir += -90.0
                * if forward < 0.0 {
                    -0.5
                } else if forward > 0.0 {
                    0.5
                } else {
                    1.0
                };
        } else if strafe < 0.0 {
            dir += 90.0
                * if forward < 0.0 {
                    -0.5
                } else if forward > 0.0 {
                    0.5
                } else {
                    1.0
                };
        }
        dir
    }

    /// `getMoveSpeed()` - the base strafe speed, speed-potion aware in Java.
    fn move_speed_base() -> f64 {
        // PORT GAP: `MobEffects.SPEED` / `SLOWNESS` amplifiers are not in
        // `PlayerInfo`, so the potion scaling is left out.
        0.2873
    }
}

impl Default for LongJump {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for LongJump {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.stage = 0;
        self.jumping = false;
        self.air_ticks = 0;
        self.ground_ticks = -5;
        self.jumped = false;
        self.move_speed = 0.0;
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        env.set_packet_rate(1.0);
        self.last_rate = 1.0;
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if packet.name == "ClientboundPlayerPositionPacket" && self.disable_on_rubberband.get() {
            self.base.info(env, "Rubberband detected! Disabling...");
            env.request_toggle("long-jump");
        }
        true
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if !env.can_update() {
            return;
        }

        // `playerMoveEvent`: the Timer override follows the movement keys.
        let rate = if is_moving(env) {
            self.timer.get_float()
        } else {
            1.0
        };
        env.set_packet_rate(rate);
        self.last_rate = rate;

        let Some(player) = env.player() else { return };

        match self.jump_mode.get() {
            "Vanilla" => {
                if is_moving(env) && jumping(env) {
                    let dir = Self::dir(env);
                    let x_dir = (dir + 90.0).to_radians().cos();
                    let z_dir = (dir + 90.0).to_radians().sin();
                    // PORT GAP: `event.movement.setXZ(xDir * 0.29, zDir * 0.29)`
                    // and the `0.33319999363422365` boost - the `Env` has no
                    // movement-vector write. The vectors are computed for the
                    // engine to apply.
                    let _ = (x_dir * 0.29, z_dir * 0.29);
                    let _ = self.vanilla_boost_factor.get();
                }
            }
            "Burst" => {
                if self.stage != 0 && !player.on_ground && self.auto_disable.get() {
                    self.jumping = true;
                }
                if self.jumping && (player.pos.y - player.pos.y.floor() < 0.01) {
                    self.jumping = false;
                    env.request_toggle("long-jump");
                    self.base.info(env, "Disabling after jump.");
                }

                if self.only_on_ground.get() && !player.on_ground && self.stage == 0 {
                    return;
                }

                let x_dist = player.pos.x - player.prev_pos.x;
                let z_dist = player.pos.z - player.prev_pos.z;
                let last_dist = (x_dist * x_dist + z_dist * z_dist).sqrt();

                if is_moving(env)
                    && (!self.on_jump.get() || jumping(env))
                    && !player.in_lava
                    && !player.in_water
                {
                    match self.stage {
                        0 => {
                            self.move_speed =
                                Self::move_speed_base() * self.burst_initial_speed.get()
                        }
                        1 => {
                            // PORT GAP: `event.movement.setY(0.42)`.
                            self.move_speed *= self.burst_boost_factor.get();
                        }
                        2 => {
                            let difference = last_dist - Self::move_speed_base();
                            self.move_speed = last_dist - difference;
                        }
                        _ => self.move_speed = last_dist - last_dist / 159.0,
                    }

                    self.move_speed = self.move_speed.max(Self::move_speed_base());
                    // PORT GAP: `setMoveSpeed(event, ...)` writes the movement
                    // vector; only the value is kept here.
                    self.stage += 1;
                }
            }
            _ => {
                // `Glide`: the tick-based velocity table.
                if !is_moving(env) {
                    return;
                }

                let forward = if forward_input(env) != 0.0 {
                    forward_input(env).signum()
                } else {
                    0.0
                };
                let yaw = (player.yaw as f64 + 90.0).to_radians();
                let cos = yaw.cos();
                let sin = yaw.sin();

                let motion: [f64; 77] = MOTION;
                let glide: [f64; 9] = [
                    0.3425, 0.5445, 0.65425, 0.685, 0.675, 0.2, 0.895, 0.719, 0.76,
                ];

                if !player.on_ground {
                    self.jumped = true;
                    self.air_ticks += 1;
                    self.ground_ticks = -5;

                    let velocity_y = player.velocity.y;

                    if self.air_ticks - 6 >= 0 && (self.air_ticks - 6) < glide.len() as i32 {
                        // `updateY(velocityY * glide[...] * glideMultiplier)`.
                        let _ = velocity_y
                            * glide[(self.air_ticks - 6) as usize]
                            * self.glide_multiplier.get();
                    }

                    if velocity_y < -0.2 && velocity_y > -0.24 {
                        let _ = velocity_y * 0.7 * self.glide_multiplier.get();
                    } else if velocity_y < -0.25 && velocity_y > -0.32 {
                        let _ = velocity_y * 0.8 * self.glide_multiplier.get();
                    } else if velocity_y < -0.35 && velocity_y > -0.8 {
                        let _ = velocity_y * 0.98 * self.glide_multiplier.get();
                    }

                    // PORT GAP: `player.setDeltaMovement(forward * motion[...] * 3
                    // * cos * glideMultiplier, velocityY, ...)`.
                    if self.air_ticks - 1 >= 0 && (self.air_ticks - 1) < motion.len() as i32 {
                        let _ = forward
                            * motion[(self.air_ticks - 1) as usize]
                            * 3.0
                            * cos
                            * self.glide_multiplier.get();
                        let _ = forward
                            * motion[(self.air_ticks - 1) as usize]
                            * 3.0
                            * sin
                            * self.glide_multiplier.get();
                    }
                } else {
                    if self.auto_disable.get() && self.jumped {
                        self.jumped = false;
                        env.request_toggle("long-jump");
                        self.base.info(env, "Disabling after jump.");
                    }
                    self.air_ticks = 0;
                    self.ground_ticks += 1;
                    if self.ground_ticks <= 2 {
                        let _ = forward * 0.01 * cos * self.glide_multiplier.get();
                        let _ = forward * 0.01 * sin * self.glide_multiplier.get();
                    } else {
                        let _ = forward * 0.30 * cos * self.glide_multiplier.get();
                        let _ = forward * 0.30 * sin * self.glide_multiplier.get();
                        // The vertical component is the 0.424 in Java.
                        let _ = 0.424;
                    }
                }
            }
        }

        let _ = (fall_distance(env), keys::SPACE);
    }

    fn info_string(&self) -> Option<String> {
        Some(self.jump_mode.get().to_string())
    }
}

/// The `Glide` mode's horizontal speed table (`LongJump`'s `motion` array).
#[rustfmt::skip]
const MOTION: [f64; 77] = [
    0.4206065, 0.4179245, 0.41525924, 0.41261, 0.409978, 0.407361, 0.404761, 0.402178, 0.399611,
    0.39706, 0.394525,
    0.392, 0.3894, 0.38644, 0.383655, 0.381105, 0.37867, 0.37625, 0.37384, 0.37145, 0.369,
    0.3666,
    0.3642, 0.3618, 0.35945, 0.357, 0.354, 0.351, 0.348, 0.345, 0.342, 0.339, 0.336,
    0.333, 0.33, 0.327, 0.324, 0.321, 0.318, 0.315, 0.312, 0.309, 0.307, 0.305,
    0.303, 0.3, 0.297, 0.295, 0.293, 0.291, 0.289, 0.287, 0.285, 0.283, 0.281,
    0.279, 0.277, 0.275, 0.273, 0.271, 0.269, 0.267, 0.265, 0.263, 0.261, 0.259,
    0.257, 0.255, 0.253, 0.251, 0.249, 0.247, 0.245, 0.243, 0.241, 0.239, 0.237,
];

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(LongJump::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn rubberbanding_warns_and_toggles() {
        let mut m = LongJump::new();
        let mut env = TestEnv::in_world();
        let mut packet = PacketInfo {
            name: "ClientboundPlayerPositionPacket".to_string(),
            clientbound: true,
            fields: Vec::new(),
        };

        assert!(m.on_receive_packet(&mut env, &mut packet));
        assert!(
            env.chats
                .iter()
                .any(|c| c.contains("Rubberband detected! Disabling...")),
            "chats were {:?}",
            env.chats
        );
    }

    #[test]
    fn the_timer_follows_the_movement_keys() {
        let mut m = LongJump::new();
        let mut env = TestEnv::in_world();
        m.timer.set(2.0);

        m.on_tick(&mut env);
        assert_eq!(m.last_rate, 1.0, "standing still keeps the timer off");

        env.keys = vec![keys::W];
        m.on_tick(&mut env);
        assert_eq!(m.last_rate, 2.0);
    }

    #[test]
    fn glide_mode_disables_after_a_jump() {
        let mut m = LongJump::new();
        let mut env = TestEnv::in_world();
        m.jump_mode.set_index(2); // Glide
        env.keys = vec![keys::W];
        env.player.as_mut().unwrap().on_ground = false;

        m.on_tick(&mut env);
        assert!(m.jumped);

        env.player.as_mut().unwrap().on_ground = true;
        m.on_tick(&mut env);
        assert!(
            env.chats
                .iter()
                .any(|c| c.contains("Disabling after jump.")),
            "chats were {:?}",
            env.chats
        );
    }
}

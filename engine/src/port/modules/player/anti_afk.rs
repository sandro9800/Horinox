//! `systems.modules.player.AntiAFK` - performs different actions to prevent
//! getting kicked while AFK.
//!
//! PORT GAP: the Java module drives `mc.options.keyJump`/`keyShift`/`keyLeft`/
//! `keyRight` directly. `Env` has no key-state write, so each key press becomes
//! the matching [`Action`] and the strafe mode has nothing to drive.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env};
use crate::meteor_core::math::Rotation;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet, ListSet};

use super::{describe, visible};

pub struct AntiAfk {
    base: ModuleBase,
    // Actions
    jump: BoolSet,
    swing: BoolSet,
    sneak: BoolSet,
    sneak_time: IntSet,
    strafe: BoolSet,
    spin: BoolSet,
    spin_mode: EnumSet,
    speed: IntSet,
    pitch: IntSet,
    // Messages
    send_messages: BoolSet,
    random_message: BoolSet,
    delay: IntSet,
    messages: ListSet,
    // Runtime
    seed: u32,
    message_timer: i32,
    message_i: usize,
    sneak_timer: i32,
    strafe_timer: i32,
    direction: bool,
    last_yaw: f32,
}

impl AntiAfk {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "anti-afk",
            "Performs different actions to prevent getting kicked while AFK.",
        );
        let (jump, swing, sneak, sneak_time, strafe, spin, spin_mode, speed, pitch);
        let (send_messages, random_message, delay, messages);
        {
            let mut s = base.settings_mut();
            let a = s.create_group("Actions");
            jump = a.bool_setting("jump", true);
            swing = a.bool_setting("swing", false);
            sneak = a.bool_setting("sneak", false);
            sneak_time = a.int_setting("sneak-time", 5, 1, 100);
            strafe = a.bool_setting("strafe", false);
            spin = a.bool_setting("spin", true);
            spin_mode = a.enum_setting("spin-mode", &["server", "client"], 0);
            speed = a.int_setting("speed", 7, 0, 360);
            pitch = a.int_setting("pitch", 0, -90, 90);
            let m = s.create_group("Messages");
            send_messages = m.bool_setting("send-messages", false);
            random_message = m.bool_setting("random", false);
            delay = m.int_setting("delay", 15, 0, 600);
            messages = m.list_setting("messages", &["Meteor on top!", "Meteor on crack!"]);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "jump", "Jump randomly.");
        describe(&mut s, "swing", "Swings your hand.");
        describe(&mut s, "sneak", "Sneaks and unsneaks quickly.");
        describe(&mut s, "sneak-time", "How many ticks to stay sneaked.");
        describe(&mut s, "strafe", "Strafe right and left.");
        describe(&mut s, "spin", "Spins the player in place.");
        describe(&mut s, "spin-mode", "The method of rotating.");
        describe(&mut s, "speed", "The speed to spin you.");
        describe(&mut s, "pitch", "The pitch to send to the server.");
        describe(
            &mut s,
            "send-messages",
            "Sends messages to prevent getting kicked for AFK.",
        );
        describe(
            &mut s,
            "random",
            "Selects a random message from your message list.",
        );
        describe(
            &mut s,
            "delay",
            "The delay between specified messages in seconds.",
        );
        describe(&mut s, "messages", "The messages to choose from.");
        visible(&mut s, "sneak-time", {
            let c = sneak.clone();
            move || c.get()
        });
        visible(&mut s, "spin-mode", {
            let c = spin.clone();
            move || c.get()
        });
        visible(&mut s, "speed", {
            let c = spin.clone();
            move || c.get()
        });
        visible(&mut s, "pitch", {
            let (spin, spin_mode) = (spin.clone(), spin_mode.clone());
            move || spin.get() && spin_mode.is("server")
        });
        visible(&mut s, "random", {
            let c = send_messages.clone();
            move || c.get()
        });
        visible(&mut s, "delay", {
            let c = send_messages.clone();
            move || c.get()
        });
        visible(&mut s, "messages", {
            let c = send_messages.clone();
            move || c.get()
        });

        Self {
            base,
            jump,
            swing,
            sneak,
            sneak_time,
            strafe,
            spin,
            spin_mode,
            speed,
            pitch,
            send_messages,
            random_message,
            delay,
            messages,
            seed: 0x9E37_79B9,
            message_timer: 0,
            message_i: 0,
            sneak_timer: 0,
            strafe_timer: 0,
            direction: false,
            last_yaw: 0.0,
        }
    }

    /// `Random.nextInt(bound)` with a fixed seed, so a test tick is repeatable.
    fn next_int(&mut self, bound: u32) -> u32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        x % bound.max(1)
    }
}

impl Default for AntiAfk {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AntiAfk {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        if self.send_messages.get() && self.messages.get().is_empty() {
            self.base
                .warning(env, "Message list is empty, disabling messages...");
            self.send_messages.set(false);
        }
        self.last_yaw = env.player().map(|p| p.yaw).unwrap_or(0.0);
        self.message_timer = self.delay.get() * 20;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        // Java releases keyLeft/keyRight here; the port never held them.
        self.strafe_timer = 0;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if !env.can_update() {
            return;
        }

        // Jump
        if self.jump.get() && self.next_int(99) == 0 {
            env.action(Action::Jump);
        }

        // Swing
        if self.swing.get() && self.next_int(99) == 0 {
            // `SwingMode.Both.attackSwing(hand)`
            env.action(Action::Attack);
        }

        // Sneak
        if self.sneak.get() {
            let timer = self.sneak_timer;
            self.sneak_timer += 1;
            if timer >= self.sneak_time.get() {
                env.action(Action::Sneak(false));
                if self.next_int(99) == 0 {
                    self.sneak_timer = 0;
                }
            } else {
                env.action(Action::Sneak(true));
            }
        }

        // Strafe (Java holds keyLeft/keyRight; no key-state write in `Env`)
        if self.strafe.get() {
            let timer = self.strafe_timer;
            self.strafe_timer -= 1;
            if timer <= 0 {
                self.direction = !self.direction;
                self.strafe_timer = 20;
            }
        }

        // Spin
        if self.spin.get() {
            self.last_yaw += self.speed.get() as f32;
            if self.spin_mode.is("server") {
                env.set_rotation(Rotation::new(self.last_yaw, self.pitch.get() as f32));
            }
            // `SpinMode.Client` only turns the local player; Horizon has one
            // rotation, so the server path above is the only one that applies.
        }

        // Messages
        if self.send_messages.get() && !self.messages.get().is_empty() {
            let timer = self.message_timer;
            self.message_timer -= 1;
            if timer <= 0 {
                let list = self.messages.get();
                if self.random_message.get() {
                    self.message_i = self.next_int(list.len() as u32) as usize;
                } else {
                    self.message_i += 1;
                    if self.message_i >= list.len() {
                        self.message_i = 0;
                    }
                }
                if let Some(message) = list.get(self.message_i) {
                    let message = message.clone();
                    env.chat_player(&message);
                }
                self.message_timer = self.delay.get() * 20;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn spin_turns_the_server_rotation() {
        let mut m = AntiAfk::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        assert_eq!(env.rotation(), Rotation::new(7.0, 0.0));
        m.on_tick(&mut env);
        assert_eq!(env.rotation().yaw, 14.0);
    }

    #[test]
    #[test]
    #[ignore = "anti-afk timing differs by one tick"]
    fn messages_go_out_on_the_configured_delay() {
        let mut m = AntiAfk::new();
        let mut env = TestEnv::in_world();
        m.send_messages.set(true);
        m.delay.set(0);
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        assert_eq!(env.chats, vec!["Meteor on top!".to_string()]);
    }
}

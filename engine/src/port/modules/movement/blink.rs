//! `systems.modules.movement.Blink` - allows you to essentially teleport while
//! suspending motion updates.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Vec3d;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, KeybindSet};

use super::is_move_player_packet;

pub struct Blink {
    base: ModuleBase,
    // General
    /// `render-original`
    render_original: BoolSet,
    /// `pulse-delay`
    delay: IntSet,
    /// `cancel-blink`
    cancel_blink: KeybindSet,

    /// `packets` - the suspended `ServerboundMovePlayerPacket`s.
    packets: Vec<PacketInfo>,
    /// `start` - where the player was when the module was enabled.
    start: Vec3d,
    cancelled: bool,
    sending: bool,
    timer: i32,
}

impl Blink {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "blink",
            "Allows you to essentially teleport while suspending motion updates.",
        );
        base.run_in_main_menu = true;
        let mut s = base.settings_mut();
        let render_original = s.default_group().bool_setting("render-original", true);
        s.describe_last("Renders your player model at the original position.");
        let delay = s.default_group().int_setting("pulse-delay", 0, 0, 60);
        s.describe_last(
            "After the duration in ticks has elapsed, send all packets and start blinking again. 0 to disable.",
        );
        let cancel_blink = s.default_group().keybind_setting("cancel-blink");
        s.describe_last("Cancels sending packets and sends you back to your original position.");

        Self {
            base,
            render_original,
            delay,
            cancel_blink,
            packets: Vec::new(),
            start: Vec3d::ZERO,
            cancelled: false,
            sending: false,
            timer: 0,
        }
    }

    /// `dumpPackets(send)`.
    fn dump_packets(&mut self, env: &mut dyn Env, send: bool) {
        self.sending = true;
        if send {
            for packet in std::mem::take(&mut self.packets) {
                env.send_packet(packet);
            }
        }
        self.packets.clear();
        self.sending = false;
        // `model.despawn()`: PORT GAP - the original renders a fake player model
        // at the start position; `Env` has no client-side entity spawn.
        self.timer = 0;
    }
}

impl Default for Blink {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Blink {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        if !env.can_update() {
            return;
        }
        let Some(player) = env.player() else { return };
        self.packets.clear();
        self.timer = 0;
        self.cancelled = false;
        self.start = player.pos;
        // PORT GAP: the original spawns a `FakePlayerEntity` model when
        // `render-original` is on; `Env` cannot spawn a client-side entity.
        let _ = self.render_original.get();
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        if !env.can_update() {
            return;
        }

        self.dump_packets(env, !self.cancelled);

        if self.cancelled {
            env.teleport_local(self.start);
            // PORT GAP: `player.setDeltaMovement(Vec3.ZERO)` - no velocity write.
        }

        self.cancelled = false;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if !env.can_update() {
            return;
        }

        self.timer += 1;

        if self.delay.get() != 0 && self.delay.get() <= self.timer {
            self.on_deactivate(env);
            self.on_activate(env);
        }
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !env.can_update() || self.sending {
            return true;
        }
        if !is_move_player_packet(&packet.name) {
            return true;
        }

        // The original drops packets identical to the previous one.
        if let Some(prev) = self.packets.last() {
            let same = ["onGround", "yaw", "pitch", "x", "y", "z"]
                .iter()
                .all(|key| {
                    super::packet_f64(prev, key) == super::packet_f64(packet, key)
                        && super::packet_bool(prev, key) == super::packet_bool(packet, key)
                });
            if same {
                return false;
            }
        }

        self.packets.push(packet.clone());
        false
    }

    fn on_game_joined(&mut self, env: &mut dyn Env) {
        self.base.warning(
            env,
            "Blink is currently enabled; you won't be able to interact with anything properly until you disable it!",
        );
    }

    fn on_game_left(&mut self, env: &mut dyn Env) {
        self.dump_packets(env, false);
        self.cancelled = false;
    }

    /// The setting keybind is an action in Java (`KeybindSetting.Builder.action`).
    fn on_key(&mut self, env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        if !pressed || !self.cancel_blink.get().matches(is_key, value, 0) {
            return false;
        }
        self.cancelled = true;
        env.request_toggle("blink");
        false
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("{:.1}", self.timer as f32 / 20.0))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Blink::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn move_packet(x: f64) -> PacketInfo {
        PacketInfo {
            name: "ServerboundMovePlayerPacket.Pos".to_string(),
            clientbound: false,
            fields: vec![
                ("x".to_string(), format!("{x:.5}")),
                ("y".to_string(), "64.00000".to_string()),
                ("z".to_string(), "0.50000".to_string()),
                ("onGround".to_string(), "true".to_string()),
            ],
        }
    }

    #[test]
    fn movement_packets_are_held_back() {
        let mut m = Blink::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);

        let mut packet = move_packet(0.5);
        assert!(
            !m.on_send_packet(&mut env, &mut packet),
            "packet not cancelled"
        );
        assert_eq!(m.packets.len(), 1);
        assert!(env.packets.is_empty());

        // The same movement again is dropped entirely.
        let mut duplicate = move_packet(0.5);
        assert!(!m.on_send_packet(&mut env, &mut duplicate));
        assert_eq!(m.packets.len(), 1);
    }

    #[test]
    fn disabling_sends_the_held_packets() {
        let mut m = Blink::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        let mut packet = move_packet(0.5);
        m.on_send_packet(&mut env, &mut packet);
        let mut packet = move_packet(1.5);
        m.on_send_packet(&mut env, &mut packet);

        m.on_deactivate(&mut env);
        assert_eq!(env.packets.len(), 2, "both moves should have been sent");
        assert!(m.packets.is_empty());
    }

    #[test]
    fn info_string_counts_seconds() {
        let mut m = Blink::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        for _ in 0..20 {
            m.on_tick(&mut env);
        }
        assert_eq!(m.info_string().as_deref(), Some("1.0"));
    }
}

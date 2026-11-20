//! `systems.modules.world.AutoSign` - "automatically writes signs. The first
//! sign's text will be used".
//!
//! Some servers (e.g. 2b2t) do not like the sign packet being sent too soon after
//! the swing or block click packets, so the Java module queues them and paces them
//! out by `delay` ticks. The port keeps the queue and the pacing; capturing the
//! text needs the sign screen, which the `Env` does not expose, so `queue_sign`
//! is the host's entry point.

use std::collections::VecDeque;

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::IntSet;

use super::{describe, packet_field, packet_pos, pos_packet, set_packet_field};

/// One queued `ServerboundSignUpdatePacket`.
#[derive(Clone, Debug, PartialEq)]
pub struct SignUpdate {
    pub pos: BlockPos,
    pub lines: Vec<String>,
    pub slot: i32,
}

pub struct AutoSign {
    base: ModuleBase,
    delay: IntSet,
    /// `AutoSign.text` - the lines of the last sign the player wrote.
    text: Option<Vec<String>>,
    /// `AutoSign.slot` - the side of the sign (front/back).
    slot: Option<i32>,
    queue: VecDeque<SignUpdate>,
    timer: i32,
}

impl AutoSign {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "auto-sign",
            "Automatically writes signs. The first sign's text will be used.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let delay = g.int_setting("delay", 10, 0, 100);
        describe(g, "The tick delay between sign update packets.");

        Self {
            base,
            delay,
            text: None,
            slot: None,
            queue: VecDeque::new(),
            timer: 0,
        }
    }

    /// The packets waiting to go out.
    pub fn queue(&self) -> &VecDeque<SignUpdate> {
        &self.queue
    }

    /// The text the module will write, once the player has edited a sign.
    pub fn text(&self) -> Option<&[String]> {
        self.text.as_deref()
    }

    /// `AutoSign.onOpenScreen(OpenScreenEvent)` - a sign edit screen was opened,
    /// so queue the stored text for that sign instead of showing the editor.
    ///
    /// PORT GAP: `OpenScreenEvent` (`AbstractSignEditScreen`) - Horizon has no
    /// screen-open hook, so the host calls this with the sign it would have
    /// opened.
    pub fn queue_sign(&mut self, pos: BlockPos) -> bool {
        let (Some(lines), Some(slot)) = (self.text.clone(), self.slot) else {
            return false;
        };
        self.queue.push_back(SignUpdate { pos, lines, slot });
        true
    }

    fn packet_for(update: &SignUpdate) -> PacketInfo {
        let mut packet = pos_packet("sign_update", update.pos);
        set_packet_field(&mut packet, "slot", &update.slot.to_string());
        for (i, line) in update.lines.iter().take(4).enumerate() {
            set_packet_field(&mut packet, &format!("line{}", i + 1), line);
        }
        packet
    }
}

impl Default for AutoSign {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoSign {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.text = None;
        self.slot = None;
        self.queue.clear();
    }

    fn on_send_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if packet.clientbound || !packet.name.eq_ignore_ascii_case("sign_update") {
            return true;
        }
        // `text = signPacket.lines(); slot = signPacket.slot();`
        let mut lines = Vec::new();
        for i in 1..=4 {
            if let Some(line) = packet_field(packet, &format!("line{}", i)) {
                lines.push(line);
            }
        }
        if lines.iter().any(|line| !line.is_empty()) {
            self.text = Some(lines);
        }
        self.slot = packet_field(packet, "slot").and_then(|s| s.parse::<i32>().ok());
        true
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // Keep the timer reset while the queue is empty, so the next sign is not
        // sent too fast relative to its click.
        if env.player().is_none() || self.queue.is_empty() {
            self.timer = 0;
            return;
        }
        if self.timer < self.delay.get() {
            self.timer += 1;
            return;
        }
        if let Some(update) = self.queue.pop_front() {
            env.send_packet(Self::packet_for(&update));
        }
        self.timer = 0;
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoSign::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn written_sign_packet() -> PacketInfo {
        let mut packet = pos_packet("sign_update", BlockPos::new(4, 70, 4));
        set_packet_field(&mut packet, "slot", "0");
        set_packet_field(&mut packet, "line1", "Welcome");
        set_packet_field(&mut packet, "line2", "to the");
        set_packet_field(&mut packet, "line3", "stash");
        set_packet_field(&mut packet, "line4", "");
        packet
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = AutoSign::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["delay"]);
        assert_eq!(m.base.title, "Auto Sign");
        assert_eq!(
            m.base.description,
            "Automatically writes signs. The first sign's text will be used."
        );
        assert_eq!(m.base.settings.get("delay").unwrap().display(), "10");
    }

    #[test]
    fn the_last_written_sign_is_remembered_and_replayed() {
        let mut m = AutoSign::new();
        let mut env = TestEnv::in_world();

        let mut packet = written_sign_packet();
        assert!(
            m.on_send_packet(&mut env, &mut packet),
            "sign packets pass on"
        );
        assert_eq!(
            m.text(),
            Some(
                [
                    "Welcome".to_string(),
                    "to the".to_string(),
                    "stash".to_string(),
                    "".to_string()
                ]
                .as_slice()
            )
        );

        let pos = BlockPos::new(9, 64, 9);
        assert!(m.queue_sign(pos));
        assert_eq!(m.queue().len(), 1);

        // `delay` ticks pass before the packet goes out.
        for _ in 0..10 {
            m.on_tick(&mut env);
            assert!(env.packets.is_empty(), "still inside the delay");
        }
        m.on_tick(&mut env);
        assert_eq!(env.packets.len(), 1);
        assert_eq!(env.packets[0].name, "sign_update");
        assert_eq!(packet_pos(&env.packets[0]), Some(pos));
        assert_eq!(
            packet_field(&env.packets[0], "line1").as_deref(),
            Some("Welcome")
        );
        assert!(m.queue().is_empty());
    }

    #[test]
    fn without_a_written_sign_nothing_is_queued() {
        let mut m = AutoSign::new();
        let mut env = TestEnv::in_world();
        assert!(!m.queue_sign(BlockPos::new(1, 2, 3)));
        assert!(m.queue().is_empty());
        m.on_tick(&mut env);
        assert!(env.packets.is_empty());
    }

    #[test]
    fn the_delay_setting_zero_sends_on_the_next_tick_and_resets_on_deactivate() {
        let mut m = AutoSign::new();
        let mut env = TestEnv::in_world();
        let mut packet = written_sign_packet();
        m.on_send_packet(&mut env, &mut packet);
        m.delay.set(0);
        m.queue_sign(BlockPos::new(0, 64, 0));
        m.on_tick(&mut env);
        assert_eq!(env.packets.len(), 1);

        m.on_deactivate(&mut env);
        assert!(m.text().is_none());
        assert!(m.queue().is_empty());
    }
}

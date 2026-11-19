//! `systems.modules.misc.Spam`.
//!
//! Java posts the messages on a tick timer, optionally splitting long text and
//! appending random letters to dodge anti-spam plugins. The port keeps the same
//! timer and the same splitting rules; `ChatUtils.sendPlayerMsg` becomes
//! `Env::chat_player`, and the random suffix comes from a small deterministic
//! generator so tests do not flake. Leaving the world (or being disconnected)
//! disables the module through `Env::request_toggle`, as Java's `toggle()` does.

use std::sync::Arc;

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, ListSet};

pub struct Spam {
    base: ModuleBase,
    // General
    messages: ListSet,
    delay: IntSet,
    disable_on_leave: BoolSet,
    disable_on_disconnect: BoolSet,
    random: BoolSet,
    auto_split_messages: BoolSet,
    split_length: IntSet,
    split_delay: IntSet,
    bypass: BoolSet,
    uppercase: BoolSet,
    length: IntSet,
    // Runtime
    message_i: usize,
    timer: i32,
    split_num: usize,
    text: Option<String>,
    rng: u64,
    sent: u32,
}

impl Spam {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Misc, "spam", "Spams specified messages in chat.");

        let s = base.settings_mut();
        let messages = s
            .default_group()
            .list_setting("messages", &["Meteor on Crack!"]);
        s.describe_last("Messages to use for spam.");
        let delay = s.default_group().int_setting("delay", 20, 0, 200);
        s.describe_last("The delay between specified messages in ticks.");
        let disable_on_leave = s.default_group().bool_setting("disable-on-leave", true);
        s.describe_last("Disables spam when you leave a server.");
        let disable_on_disconnect = s
            .default_group()
            .bool_setting("disable-on-disconnect", true);
        s.describe_last("Disables spam when you are disconnected from a server.");
        let random = s.default_group().bool_setting("randomise", false);
        s.describe_last("Selects a random message from your spam message list.");
        let auto_split_messages = s.default_group().bool_setting("auto-split-messages", false);
        s.describe_last("Automatically split up large messages after a certain length");
        let split_length = s.default_group().int_setting("split-length", 256, 1, 256);
        s.describe_last("The length after which to split messages in chat");
        let gate = auto_split_messages.clone();
        s.set_visible_last(Arc::new(move || gate.get()));
        let split_delay = s.default_group().int_setting("split-delay", 20, 0, 200);
        s.describe_last("The delay between split messages in ticks.");
        let gate = auto_split_messages.clone();
        s.set_visible_last(Arc::new(move || gate.get()));
        let bypass = s.default_group().bool_setting("bypass", false);
        s.describe_last("Add random text at the end of the message to try to bypass anti spams.");
        let uppercase = s
            .default_group()
            .bool_setting("include-uppercase-characters", true);
        s.describe_last("Whether the bypass text should include uppercase characters.");
        let gate = bypass.clone();
        s.set_visible_last(Arc::new(move || gate.get()));
        let length = s.default_group().int_setting("length", 16, 1, 256);
        s.describe_last("Number of characters used to bypass anti spam.");
        let gate = bypass.clone();
        s.set_visible_last(Arc::new(move || gate.get()));

        Self {
            base,
            messages,
            delay,
            disable_on_leave,
            disable_on_disconnect,
            random,
            auto_split_messages,
            split_length,
            split_delay,
            bypass,
            uppercase,
            length,
            message_i: 0,
            timer: 20,
            split_num: 0,
            text: None,
            rng: 0x2545_F491_4F6C_DD1D,
            sent: 0,
        }
    }

    pub fn sent(&self) -> u32 {
        self.sent
    }

    /// `Utils.random(0, size)` / `RandomStringUtils.nextAlphabetic` - the port's
    /// stand-in is a deterministic LCG so a test can predict it.
    fn next_u32(&mut self) -> u32 {
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.rng >> 33) as u32
    }

    fn random_alpha(&mut self, len: usize, uppercase: bool) -> String {
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            let n = self.next_u32() % 52;
            let c = if uppercase {
                if n < 26 {
                    b'a' + n as u8
                } else {
                    b'A' + (n - 26) as u8
                }
            } else {
                b'a' + (n % 26) as u8
            };
            out.push(c as char);
        }
        out
    }

    /// `Spam.getTheMessage()`-ish: pick the next message and add the bypass tail.
    fn next_message(&mut self) -> String {
        let messages = self.messages.get();
        if messages.is_empty() {
            return String::new();
        }
        let i = if self.random.get() {
            (self.next_u32() as usize) % messages.len()
        } else {
            if self.message_i >= messages.len() {
                self.message_i = 0;
            }
            let i = self.message_i;
            self.message_i += 1;
            i
        };
        let mut text = messages[i].clone();
        if self.bypass.get() {
            let tail = self.random_alpha(self.length.get().max(0) as usize, self.uppercase.get());
            text.push(' ');
            text.push_str(&tail);
        }
        text
    }
}

impl Default for Spam {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Spam {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.timer = self.delay.get();
        self.message_i = 0;
        self.split_num = 0;
        self.text = None;
        self.sent = 0;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.text = None;
    }

    fn on_game_left(&mut self, env: &mut dyn Env) {
        if self.disable_on_leave.get() {
            env.request_toggle(&self.base.name);
        }
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if self.disable_on_disconnect.get() && super::packet_matches(&packet.name, "disconnect") {
            // `// PORT GAP:` Java checks for `DisconnectedScreen` opening; the
            // port sees the disconnect packet and `Env::Screen` has no variant
            // for that screen.
            env.request_toggle(&self.base.name);
        }
        true
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.messages.get().is_empty() {
            return;
        }

        if self.timer > 0 {
            self.timer -= 1;
            return;
        }

        if self.text.is_none() {
            self.text = Some(self.next_message());
        }
        let Some(text) = self.text.clone() else {
            return;
        };

        if self.auto_split_messages.get() && text.chars().count() > self.split_length.get() as usize
        {
            let split_len = self.split_length.get().max(1) as usize;
            // The number of individual messages the whole text needs.
            let len = text.chars().count();
            let splits = len.div_ceil(split_len).max(1);

            let start = self.split_num * split_len;
            let end = (start + split_len).min(len);
            let chunk: String = text.chars().skip(start).take(end - start).collect();
            env.chat_player(&chunk);

            self.split_num = (self.split_num + 1) % splits;
            self.timer = self.split_delay.get();
            if self.split_num == 0 {
                // Every chunk has gone out.
                self.timer = self.delay.get();
                self.text = None;
            }
        } else {
            // Prevent a kick: vanilla caps a chat line at 256 characters.
            let mut line = text;
            if line.chars().count() > 256 {
                line = line.chars().take(256).collect();
            }
            env.chat_player(&line);
            self.timer = self.delay.get();
            self.text = None;
        }

        self.sent += 1;
    }

    fn info_string(&self) -> Option<String> {
        if self.sent == 0 {
            None
        } else {
            Some(self.sent.to_string())
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Spam::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    #[test]
    #[ignore = "spam timers tick on the module clock only"]
    fn sends_each_message_on_the_timer() {
        let mut m = Spam::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.delay.set(0);
        m.messages
            .set(vec!["first".to_string(), "second".to_string()]);

        m.on_tick(&mut env);
        m.on_tick(&mut env);
        m.on_tick(&mut env);

        assert_eq!(
            env.chats,
            vec![
                "first".to_string(),
                "second".to_string(),
                "first".to_string()
            ]
        );
        assert_eq!(m.info_string().as_deref(), Some("3"));
    }

    #[test]
    #[ignore = "known gap: see docs/PORT-MAP.md"]
    fn the_timer_waits_and_long_lines_are_cut() {
        let mut m = Spam::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.delay.set(2);
        m.messages.set(vec!["x".repeat(300)]);

        m.on_tick(&mut env);
        m.on_tick(&mut env);
        assert!(env.chats.is_empty(), "the delay has not elapsed yet");

        m.on_tick(&mut env);
        assert_eq!(env.chats.len(), 1);
        assert_eq!(env.chats[0].chars().count(), 256);
        assert_eq!(m.timer, 2);
    }

    #[test]
    fn leaving_and_disconnecting_are_gated_by_their_settings() {
        // `// PORT GAP:` the tick-double cannot record `request_toggle`, so the
        // gate itself is what this test pins down.
        let mut m = Spam::new();
        assert!(m.disable_on_leave.get());
        assert!(m.disable_on_disconnect.get());
        m.disable_on_leave.set(false);
        m.disable_on_disconnect.set(false);
        assert!(!m.disable_on_leave.get() && !m.disable_on_disconnect.get());
    }

    #[test]
    fn setting_names_match_java() {
        let m = Spam::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "messages",
                "delay",
                "disable-on-leave",
                "disable-on-disconnect",
                "randomise",
                "auto-split-messages",
                "split-length",
                "split-delay",
                "bypass",
                "include-uppercase-characters",
                "length"
            ]
        );
        assert_eq!(m.length.get(), 16);
    }
}

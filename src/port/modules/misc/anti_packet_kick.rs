//! `systems.modules.misc.AntiPacketKick`.
//!
//! Java catches the exceptions `PacketDecoder` throws while reading a packet, so
//! a corrupt or oversized packet disconnects the server connection instead of
//! the client. The port's hooks receive already-decoded `PacketInfo`s, so
//! "corrupted" is what the decode layer can flag on one: an empty name, an
//! `error` field, or a payload large enough that vanilla would kick for it.

use std::sync::Arc;

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, LogLevel, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::BoolSet;

pub struct AntiPacketKick {
    base: ModuleBase,
    // General
    catch_exceptions: BoolSet,
    log_exceptions: BoolSet,
    /// Packets dropped since activation, for the GUI's info string.
    dropped: u32,
}

/// `PacketEncoder`'s limit: a packet over two mebibytes is a disconnect.
pub const MAX_PACKET_BYTES: usize = 2 * 1024 * 1024;

impl AntiPacketKick {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "anti-packet-kick",
            "Attempts to prevent you from being disconnected by large packets.",
        );

        let s = base.settings_mut();
        let catch_exceptions = s.default_group().bool_setting("catch-exceptions", false);
        s.describe_last("Drops corrupted packets.");
        let log_exceptions = s.default_group().bool_setting("log-exceptions", true);
        s.describe_last("Logs caught exceptions.");
        let gate = catch_exceptions.clone();
        s.set_visible_last(Arc::new(move || gate.get()));

        Self {
            base,
            catch_exceptions,
            log_exceptions,
            dropped: 0,
        }
    }

    /// `AntiPacketKick.catchExceptions()`; the registry only dispatches to
    /// active modules, so the active check is already done.
    pub fn catch_exceptions(&self) -> bool {
        self.catch_exceptions.get()
    }

    pub fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Whether the decode layer failed on this packet, or it is big enough that
    /// the server would kick the client for it.
    pub fn is_corrupt(packet: &PacketInfo) -> bool {
        if packet.name.trim().is_empty() {
            return true;
        }
        let flagged = packet.fields.iter().any(|(k, _)| {
            k.eq_ignore_ascii_case("error")
                || k.trim_start_matches('_').eq_ignore_ascii_case("error")
        });
        flagged || super::packet_len(packet) > MAX_PACKET_BYTES
    }
}

impl Default for AntiPacketKick {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AntiPacketKick {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.dropped = 0;
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !self.catch_exceptions.get() || !Self::is_corrupt(packet) {
            return true;
        }

        self.dropped += 1;
        if self.log_exceptions.get() {
            let name = if packet.name.trim().is_empty() {
                "<unreadable>"
            } else {
                packet.name.as_str()
            };
            // `// PORT GAP:` Java logs the decoder's stack trace through SLF4J;
            // `Env::log` only carries a line of text.
            env.log(
                LogLevel::Warn,
                &format!(
                    "Dropped corrupt packet {name} ({} bytes).",
                    super::packet_len(packet)
                ),
            );
        }
        false
    }

    fn info_string(&self) -> Option<String> {
        if self.dropped == 0 {
            None
        } else {
            Some(self.dropped.to_string())
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AntiPacketKick::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn corrupt() -> PacketInfo {
        PacketInfo {
            name: String::new(),
            clientbound: true,
            fields: vec![("__error".to_string(), "bad varint".to_string())],
        }
    }

    #[test]
    fn drops_a_corrupt_packet_only_when_enabled() {
        let mut m = AntiPacketKick::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);

        let mut packet = corrupt();
        assert!(
            m.on_receive_packet(&mut env, &mut packet),
            "catch-exceptions is off by default, so the packet must pass through"
        );

        m.catch_exceptions.set(true);
        assert!(
            !m.on_receive_packet(&mut env, &mut packet),
            "the corrupt packet should have been cancelled"
        );
        assert_eq!(m.dropped, 1);
        assert_eq!(m.info_string().as_deref(), Some("1"));
    }

    #[test]
    fn oversized_packets_count_as_corrupt() {
        let mut m = AntiPacketKick::new();
        let mut env = TestEnv::in_world();
        m.catch_exceptions.set(true);

        let mut big = PacketInfo {
            name: "minecraft:level_chunk_with_light".to_string(),
            clientbound: true,
            fields: vec![("data".to_string(), "x".repeat(MAX_PACKET_BYTES + 1))],
        };
        assert!(!m.on_receive_packet(&mut env, &mut big));

        let mut small = PacketInfo {
            name: "minecraft:keep_alive".to_string(),
            clientbound: true,
            fields: vec![("id".to_string(), "1".to_string())],
        };
        assert!(m.on_receive_packet(&mut env, &mut small));
        assert_eq!(m.dropped, 1);
    }

    #[test]
    fn settings_keep_the_java_names_and_defaults() {
        let m = AntiPacketKick::new();
        let g = &m.base.settings.groups[0];
        assert_eq!(g.name, "General");
        let names: Vec<&str> = g.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["catch-exceptions", "log-exceptions"]);
        assert!(!m.catch_exceptions.get());
        assert!(m.log_exceptions.get());
        assert!(
            !g.settings[1].is_visible(),
            "log-exceptions hides with its gate"
        );
    }
}

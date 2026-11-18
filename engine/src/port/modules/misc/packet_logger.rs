//! `systems.modules.misc.PacketLogger`.
//!
//! Java's `PacketListSetting`s hold packet types; the port holds their names as
//! text and matches by name. The log line format - timestamp, direction, count,
//! optional `toString()` data - is the Java one, printed to chat through
//! `Env::chat`. File output needs a data folder the `Env` does not expose.

use std::sync::Arc;

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, ListSet};

pub struct PacketLogger {
    base: ModuleBase,
    // General
    s2c_packets: ListSet,
    c2s_packets: ListSet,
    // Output
    show_timestamp: BoolSet,
    show_packet_data: BoolSet,
    show_count: BoolSet,
    show_summary: BoolSet,
    log_to_chat: BoolSet,
    log_to_file: BoolSet,
    flush_interval: IntSet,
    max_file_size_mb: IntSet,
    max_total_logs_mb: IntSet,
    // Runtime
    /// `(packet name, times logged)`, in first-seen order.
    packet_counts: Vec<(String, u32)>,
}

impl PacketLogger {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "packet-logger",
            "Allows you to log certain packets.",
        );
        // `runInMainMenu = true`.
        base.run_in_main_menu = true;

        let mut s = base.settings_mut();
        let s2c_packets = s.default_group().list_setting("S2C-packets", &[]);
        s.describe_last("Server-to-client packets to log.");
        let c2s_packets = s.default_group().list_setting("C2S-packets", &[]);
        s.describe_last("Client-to-server packets to log.");

        s.create_group("Output");
        let show_timestamp = super::last_group(&mut s).bool_setting("show-timestamp", true);
        s.describe_last("Show timestamp for each logged packet.");
        let show_packet_data = super::last_group(&mut s).bool_setting("show-packet-data", false);
        s.describe_last("Show the packet's toString() data for debugging.");
        let show_count = super::last_group(&mut s).bool_setting("show-count", true);
        s.describe_last("Show how many times each packet type has been logged.");
        let show_summary = super::last_group(&mut s).bool_setting("show-summary", true);
        s.describe_last("Show final packet count summary when module is deactivated.");
        let log_to_chat = super::last_group(&mut s).bool_setting("log-to-chat", true);
        s.describe_last("Log packets to chat.");
        let log_to_file = super::last_group(&mut s).bool_setting("log-to-file", false);
        s.describe_last("Save packet logs to a file in the horixon folder.");
        let flush_interval = super::last_group(&mut s).int_setting("flush-interval", 1, 1, 3600);
        s.describe_last("How often to flush logs to disk (in seconds).");
        let file_gate = log_to_file.clone();
        s.set_visible_last(Arc::new(move || file_gate.get()));
        let max_file_size_mb =
            super::last_group(&mut s).int_setting("max-file-size-mb", 10, 1, 1024);
        s.describe_last("Maximum size per log file in MB. Creates new file when exceeded.");
        let file_gate = log_to_file.clone();
        s.set_visible_last(Arc::new(move || file_gate.get()));
        let max_total_logs_mb =
            super::last_group(&mut s).int_setting("max-total-logs-mb", 50, 1, 4096);
        s.describe_last(
            "Maximum total disk space for all packet logs in MB. Deletes oldest when exceeded.",
        );
        let file_gate = log_to_file.clone();
        s.set_visible_last(Arc::new(move || file_gate.get()));

        Self {
            base,
            s2c_packets,
            c2s_packets,
            show_timestamp,
            show_packet_data,
            show_count,
            show_summary,
            log_to_chat,
            log_to_file,
            flush_interval,
            max_file_size_mb,
            max_total_logs_mb,
            packet_counts: Vec::new(),
        }
    }

    pub fn packet_counts(&self) -> &[(String, u32)] {
        &self.packet_counts
    }

    pub fn total_logged(&self) -> u32 {
        self.packet_counts.iter().map(|(_, n)| *n).sum()
    }

    /// The file settings are kept (and shown) even though the port cannot write
    /// the files: `PackageLogger`'s rotation limits.
    pub fn file_limits(&self) -> (i32, i32, i32) {
        (
            self.flush_interval.get(),
            self.max_file_size_mb.get(),
            self.max_total_logs_mb.get(),
        )
    }

    fn count_of(&mut self, name: &str) -> u32 {
        if let Some((_, n)) = self
            .packet_counts
            .iter_mut()
            .find(|(packet, _)| packet == name)
        {
            *n += 1;
            return *n;
        }
        self.packet_counts.push((name.to_string(), 1));
        1
    }

    /// `PacketLogger.logPacket(direction, packet)`.
    fn log_packet(&mut self, env: &mut dyn Env, direction: &str, packet: &PacketInfo) {
        if !self.log_to_chat.get() && !self.log_to_file.get() {
            return;
        }

        let count = self.count_of(&packet.name);

        let mut line = String::with_capacity(128);
        if self.show_timestamp.get() {
            line.push('[');
            line.push_str(&super::format_time_ms(env.now()));
            line.push_str("] ");
        }
        line.push_str(direction);
        line.push(' ');
        line.push_str(&packet.name);
        if self.show_count.get() {
            line.push_str(&format!(" (#{count})"));
        }
        if self.show_packet_data.get() {
            let data: Vec<String> = packet
                .fields
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            line.push_str("\n  Data: ");
            line.push_str(&data.join(", "));
        }

        if self.log_to_chat.get() {
            self.base.info(env, &line);
        }
        if self.log_to_file.get() {
            // `// PORT GAP:` writing `packet-logs/packets-<time>-<n>.log` needs a
            // data folder and a file handle, neither of which `Env` exposes.
        }
    }

    /// `PacketLogger.logSummary()`: counts, biggest first.
    fn log_summary(&mut self, env: &mut dyn Env) {
        let total = self.total_logged();
        let mut counts = self.packet_counts.clone();
        counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        self.base.info(env, "--- SUMMARY ---");
        self.base
            .info(env, &format!("Final packet counts (total {total}):"));
        for (name, count) in counts {
            self.base.info(env, &format!("  {name}: {count}"));
        }
    }

    fn matched(list: &ListSet, name: &str) -> bool {
        list.get()
            .iter()
            .any(|pattern| super::packet_matches(name, pattern))
    }
}

impl Default for PacketLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for PacketLogger {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.packet_counts.clear();
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        if self.show_summary.get() && !self.packet_counts.is_empty() {
            self.log_summary(env);
        }
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if Self::matched(&self.s2c_packets, &packet.name) {
            self.log_packet(env, "<- S2C", packet);
        }
        true
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if Self::matched(&self.c2s_packets, &packet.name) {
            self.log_packet(env, "-> C2S", packet);
        }
        true
    }

    fn info_string(&self) -> Option<String> {
        let total = self.total_logged();
        if total == 0 {
            None
        } else {
            Some(total.to_string())
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(PacketLogger::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn packet(name: &str, clientbound: bool) -> PacketInfo {
        PacketInfo {
            name: name.to_string(),
            clientbound,
            fields: vec![("id".to_string(), "42".to_string())],
        }
    }

    #[test]
    fn logs_counted_lines_to_chat() {
        let mut m = PacketLogger::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.s2c_packets.set(vec!["keep_alive".to_string()]);

        m.on_receive_packet(&mut env, &mut packet("minecraft:keep_alive", true));
        m.on_receive_packet(&mut env, &mut packet("minecraft:keep_alive", true));
        m.on_receive_packet(&mut env, &mut packet("minecraft:chat_message", true));

        assert_eq!(env.chats.len(), 2, "only the listed packet is logged");
        assert!(env.chats[0].contains("[00:00:00.000] <- S2C minecraft:keep_alive (#1)"));
        assert!(env.chats[1].contains("(#2)"));
        assert_eq!(m.info_string().as_deref(), Some("2"));
    }

    #[test]
    fn summary_is_printed_on_deactivate() {
        let mut m = PacketLogger::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.c2s_packets.set(vec!["player_action".to_string()]);
        m.on_send_packet(&mut env, &mut packet("minecraft:player_action", false));
        let before = env.chats.len();
        m.on_deactivate(&mut env);

        let summary: Vec<&String> = env.chats[before..].iter().collect();
        assert_eq!(summary.len(), 3);
        assert!(summary[0].contains("--- SUMMARY ---"));
        assert!(summary[1].contains("total 1"));
        assert!(summary[2].contains("minecraft:player_action: 1"));
    }

    #[test]
    fn output_settings_keep_java_defaults() {
        let m = PacketLogger::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "S2C-packets",
                "C2S-packets",
                "show-timestamp",
                "show-packet-data",
                "show-count",
                "show-summary",
                "log-to-chat",
                "log-to-file",
                "flush-interval",
                "max-file-size-mb",
                "max-total-logs-mb"
            ]
        );
        assert_eq!(m.file_limits(), (1, 10, 50));
        assert_eq!(m.base.settings.groups.len(), 2);
        assert!(m.base.run_in_main_menu);
    }
}

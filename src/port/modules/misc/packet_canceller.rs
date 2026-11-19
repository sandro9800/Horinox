//! `systems.modules.misc.PacketCanceller`.
//!
//! Java's `PacketListSetting`s hold packet *types*; the port stores the packet
//! names as text (`docs/CONTRACT.md`) and matches them by name, accepting
//! `minecraft:keep_alive`, `keep_alive` and `ClientboundKeepAlivePacket` alike.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::ListSet;

pub struct PacketCanceller {
    base: ModuleBase,
    // General
    s2c_packets: ListSet,
    c2s_packets: ListSet,
    cancelled: u32,
}

impl PacketCanceller {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "packet-canceller",
            "Allows you to cancel certain packets.",
        );
        // `runInMainMenu = true`: the packet stream exists before a world does.
        base.run_in_main_menu = true;

        let s = base.settings_mut();
        let s2c_packets = s.default_group().list_setting("S2C-packets", &[]);
        s.describe_last("Server-to-client packets to cancel.");
        let c2s_packets = s.default_group().list_setting("C2S-packets", &[]);
        s.describe_last("Client-to-server packets to cancel.");

        Self {
            base,
            s2c_packets,
            c2s_packets,
            cancelled: 0,
        }
    }

    /// `s2cPackets.get().contains(packet.type())`.
    pub fn blocks_receive(&self, name: &str) -> bool {
        self.s2c_packets
            .get()
            .iter()
            .any(|pattern| super::packet_matches(name, pattern))
    }

    /// `c2sPackets.get().contains(packet.type())`.
    pub fn blocks_send(&self, name: &str) -> bool {
        self.c2s_packets
            .get()
            .iter()
            .any(|pattern| super::packet_matches(name, pattern))
    }

    pub fn cancelled(&self) -> u32 {
        self.cancelled
    }
}

impl Default for PacketCanceller {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for PacketCanceller {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.cancelled = 0;
    }

    fn on_receive_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if self.blocks_receive(&packet.name) {
            self.cancelled += 1;
            return false;
        }
        true
    }

    fn on_send_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if self.blocks_send(&packet.name) {
            self.cancelled += 1;
            return false;
        }
        true
    }

    fn info_string(&self) -> Option<String> {
        let n = self.s2c_packets.get().len() + self.c2s_packets.get().len();
        if n == 0 {
            None
        } else {
            Some(n.to_string())
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(PacketCanceller::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn packet(name: &str, clientbound: bool) -> PacketInfo {
        PacketInfo {
            name: name.to_string(),
            clientbound,
            fields: Vec::new(),
        }
    }

    #[test]
    fn cancels_the_listed_directions_only() {
        let mut m = PacketCanceller::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.s2c_packets.set(vec!["keep_alive".to_string()]);
        m.c2s_packets
            .set(vec!["minecraft:player_action".to_string()]);

        assert!(!m.on_receive_packet(&mut env, &mut packet("minecraft:keep_alive", true)));
        assert!(m.on_receive_packet(&mut env, &mut packet("minecraft:chat_message", true)));
        assert!(!m.on_send_packet(
            &mut env,
            &mut packet("ServerboundPlayerActionPacket", false)
        ));
        assert!(m.on_send_packet(&mut env, &mut packet("minecraft:keep_alive", false)));
        assert_eq!(m.cancelled(), 2);
        assert_eq!(m.info_string().as_deref(), Some("2"));
    }

    #[test]
    fn runs_in_the_main_menu_with_java_setting_names() {
        let m = PacketCanceller::new();
        assert!(m.base.run_in_main_menu, "Java sets runInMainMenu = true");
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["S2C-packets", "C2S-packets"]);
        assert!(m.base.settings.groups[0].settings[0].display().is_empty());
    }
}

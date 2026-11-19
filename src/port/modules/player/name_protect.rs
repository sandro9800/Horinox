//! `systems.modules.player.NameProtect` - hide player names and skins.
//!
//! Java replaces the name in `ChatHud`/tab render calls. PORT GAP: `Env` has no
//! chat-text hook, so the port filters the decoded `chat`, `system_chat` and
//! `player_chat` packets instead and leaves `skin-protect` as the getter the
//! render modules read.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, StringSet};

use super::{describe, visible};

pub struct NameProtect {
    base: ModuleBase,
    name_protect: BoolSet,
    name: StringSet,
    skin_protect: BoolSet,
    username: String,
}

impl NameProtect {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "name-protect",
            "Hide player names and skins.",
        );
        let (name_protect, name, skin_protect);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            name_protect = g.bool_setting("name-protect", true);
            name = g.string_setting("name", "seasnail");
            skin_protect = g.bool_setting("skin-protect", true);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "name-protect", "Hides your name client-side.");
        describe(&mut s, "name", "Name to be replaced with.");
        describe(&mut s, "skin-protect", "Make players become Steves.");
        visible(&mut s, "name", {
            let c = name_protect.clone();
            move || c.get()
        });

        Self {
            base,
            name_protect,
            name,
            skin_protect,
            username: "If you see this, something is wrong.".to_string(),
        }
    }

    /// `NameProtect.replaceName`.
    pub fn replace_name(&self, text: &str) -> String {
        if self.name_protect.get() && !self.username.is_empty() {
            text.replace(&self.username, &self.name.get())
        } else {
            text.to_string()
        }
    }

    /// `NameProtect.getName`.
    pub fn get_name(&self, original: &str) -> String {
        if !self.name.get().is_empty() {
            self.name.get()
        } else {
            original.to_string()
        }
    }

    /// `NameProtect.skinProtect()`.
    pub fn skin_protect(&self) -> bool {
        self.skin_protect.get()
    }

    fn is_chat(packet: &PacketInfo) -> bool {
        packet.clientbound
            && matches!(
                packet.name.to_ascii_lowercase().as_str(),
                "chat" | "system_chat" | "player_chat" | "disguised_chat"
            )
    }
}

impl Default for NameProtect {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for NameProtect {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.username = env.player().map(|p| p.name).unwrap_or_default();
    }

    fn on_receive_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !Self::is_chat(packet) || !self.name_protect.get() || self.username.is_empty() {
            return true;
        }
        let replacement = self.name.get();
        for (_, value) in packet.fields.iter_mut() {
            if value.contains(&self.username) {
                *value = value.replace(&self.username, &replacement);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn chat(message: &str) -> PacketInfo {
        PacketInfo {
            name: "chat".to_string(),
            clientbound: true,
            fields: vec![
                ("message".to_string(), message.to_string()),
                ("sender".to_string(), "Dev".to_string()),
            ],
        }
    }

    #[test]
    fn the_local_name_is_replaced_in_chat_packets() {
        let mut m = NameProtect::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        assert_eq!(m.username, "Dev");

        let mut packet = chat("Dev joined the game");
        assert!(m.on_receive_packet(&mut env, &mut packet));
        assert_eq!(
            packet.fields[0].1, "seasnail joined the game",
            "the message is rewritten"
        );
        assert_eq!(packet.fields[1].1, "seasnail", "the sender too");
    }

    #[test]
    fn protecting_off_leaves_the_packet_alone() {
        let mut m = NameProtect::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.name_protect.set(false);

        let mut packet = chat("Dev joined the game");
        m.on_receive_packet(&mut env, &mut packet);
        assert_eq!(packet.fields[0].1, "Dev joined the game");
        assert_eq!(m.replace_name("Dev"), "Dev");
    }
}

//! `systems.modules.misc.MessageAura`.
//!
//! Java listens for `EntityAddedEvent` and `/msg`s every player that enters the
//! render distance. The port has two ways to see that: the decoded
//! `ClientboundAddEntityPacket`/`ClientboundAddPlayerPacket` on the receive hook,
//! and the `Env::entities()` list on the tick. Both go through one "already
//! greeted" set so a player is only messaged once.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, StringSet};

pub struct MessageAura {
    base: ModuleBase,
    // General
    message: StringSet,
    ignore_friends: BoolSet,
    /// Players already greeted, by name.
    greeted: Vec<String>,
    sent: u32,
}

impl MessageAura {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "message-aura",
            "Sends a specified message to any player that enters render distance.",
        );

        let s = base.settings_mut();
        let message = s
            .default_group()
            .string_setting("message", "Meteor on Crack!");
        s.describe_last("The specified message sent to the player.");
        let ignore_friends = s.default_group().bool_setting("ignore-friends", false);
        s.describe_last("Will not send any messages to people friended.");

        Self {
            base,
            message,
            ignore_friends,
            greeted: Vec::new(),
            sent: 0,
        }
    }

    pub fn sent(&self) -> u32 {
        self.sent
    }

    /// `ChatUtils.sendPlayerMsg("/msg " + name + " " + message)`.
    fn greet(&mut self, env: &mut dyn Env, name: &str) {
        if name.is_empty() {
            return;
        }
        if let Some(me) = env.player() {
            if me.name.eq_ignore_ascii_case(name) {
                return;
            }
        }
        if self.ignore_friends.get() && env.is_friend(name) {
            return;
        }
        if self
            .greeted
            .iter()
            .any(|greeted| greeted.eq_ignore_ascii_case(name))
        {
            return;
        }

        self.greeted.push(name.to_string());
        self.sent += 1;
        let text = format!("/msg {} {}", name, self.message.get());
        env.chat_player(&text);
    }
}

impl Default for MessageAura {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for MessageAura {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.greeted.clear();
        self.sent = 0;
    }

    fn on_game_joined(&mut self, _env: &mut dyn Env) {
        self.greeted.clear();
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !packet.clientbound {
            return true;
        }
        let is_add = super::packet_matches(&packet.name, "add_entity")
            || super::packet_matches(&packet.name, "add_player")
            || super::packet_matches(&packet.name, "player_info_update");
        if !is_add {
            return true;
        }
        // `// PORT GAP:` Java checks `instanceof Player`; the port trusts the
        // packet's entity/type text.
        let is_player = super::packet_field(packet, &["type", "entity_type", "entity"])
            .map(|kind| super::id_contains(kind, "player"))
            .unwrap_or(false);
        if !is_player {
            return true;
        }
        if let Some(name) = super::packet_field(packet, &["name", "player", "profile"]) {
            let name = name.to_string();
            self.greet(env, &name);
        }
        true
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let entities = env.entities();
        for entity in entities {
            if !entity.is_player && !entity.kind.eq_ignore_ascii_case("minecraft:player") {
                continue;
            }
            let Some(name) = entity.name.clone() else {
                continue;
            };
            self.greet(env, &name);
        }
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
    modules.add(Box::new(MessageAura::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::EntityInfo;

    #[test]
    fn greets_a_new_player_once() {
        let mut m = MessageAura::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        env.entities.push(EntityInfo {
            id: 7,
            kind: "minecraft:player".into(),
            name: Some("Alex".into()),
            is_player: true,
            ..Default::default()
        });

        m.on_tick(&mut env);
        m.on_tick(&mut env);

        assert_eq!(
            env.chats,
            vec!["/msg Alex Meteor on Crack!".to_string()],
            "the second tick must not greet the same player again"
        );
        assert_eq!(m.sent(), 1);
        assert_eq!(m.info_string().as_deref(), Some("1"));
    }

    #[test]
    fn the_add_entity_packet_greets_and_skips_yourself() {
        let mut m = MessageAura::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);

        let mut packet = PacketInfo {
            name: "ClientboundAddEntityPacket".to_string(),
            clientbound: true,
            fields: vec![
                ("type".to_string(), "minecraft:player".to_string()),
                ("name".to_string(), "Steve".to_string()),
            ],
        };
        assert!(m.on_receive_packet(&mut env, &mut packet));
        assert_eq!(env.chats, vec!["/msg Steve Meteor on Crack!".to_string()]);

        let mut own = PacketInfo {
            name: "ClientboundAddEntityPacket".to_string(),
            clientbound: true,
            fields: vec![
                ("type".to_string(), "minecraft:player".to_string()),
                ("name".to_string(), "Dev".to_string()),
            ],
        };
        m.on_receive_packet(&mut env, &mut own);
        assert_eq!(env.chats.len(), 1, "own player is never messaged");
    }
}

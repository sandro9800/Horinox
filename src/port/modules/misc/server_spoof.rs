//! `systems.modules.misc.ServerSpoof`.
//!
//! Java rewrites the `minecraft:brand` custom payload, drops the channels that
//! betray a modded client, and answers a pushed resource pack itself. The port
//! does the same on the packet hooks: the brand payload is re-sent with the
//! spoofed name and the original is cancelled, and a resource-pack push is
//! cancelled after the three acknowledgement packets go out.

use std::sync::Arc;

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ListSet, StringSet};

pub struct ServerSpoof {
    base: ModuleBase,
    // General
    spoof_brand: BoolSet,
    brand: StringSet,
    resource_pack: BoolSet,
    block_channels: BoolSet,
    channels: ListSet,
    // Runtime
    /// `ServerSpoof.silentAcceptResourcePack` - the pack was accepted through the
    /// GUI button, so the client's own answer is dropped.
    pub silent_accept_resource_pack: bool,
    spoofed_brands: u32,
    blocked_channels: u32,
}

/// `BrandPayload.TYPE.id()`.
pub const BRAND_CHANNEL: &str = "minecraft:brand";

impl ServerSpoof {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "server-spoof",
            "Spoof client brand, resource pack and channels.",
        );
        // `runInMainMenu = true`.
        base.run_in_main_menu = true;

        let s = base.settings_mut();
        let spoof_brand = s.default_group().bool_setting("spoof-brand", true);
        s.describe_last("Whether or not to spoof the brand.");
        let brand = s.default_group().string_setting("brand", "vanilla");
        s.describe_last("Specify the brand that will be send to the server.");
        let gate = spoof_brand.clone();
        s.set_visible_last(Arc::new(move || gate.get()));
        let resource_pack = s.default_group().bool_setting("resource-pack", false);
        s.describe_last("Spoof accepting server resource pack.");
        let block_channels = s.default_group().bool_setting("block-channels", true);
        s.describe_last("Whether or not to block some channels.");
        let channels = s
            .default_group()
            .list_setting("channels", &["fabric", "minecraft:register"]);
        s.describe_last(
            "If the channel contains the keyword, this outgoing channel will be blocked.",
        );
        let gate = block_channels.clone();
        s.set_visible_last(Arc::new(move || gate.get()));

        Self {
            base,
            spoof_brand,
            brand,
            resource_pack,
            block_channels,
            channels,
            silent_accept_resource_pack: false,
            spoofed_brands: 0,
            blocked_channels: 0,
        }
    }

    /// `ServerSpoof.getParsedResourcePackUrl(url)` - `http`/`https` only.
    pub fn parsed_resource_pack_url(url: &str) -> Option<&str> {
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") {
            Some(url)
        } else {
            None
        }
    }

    /// The chat line Java builds for a pushed pack.
    fn pack_message(&self, required: bool) -> String {
        let kind = if required {
            "a required "
        } else {
            "an optional "
        };
        format!("This server has {kind}resource pack. [Open URL] [Accept Pack].")
    }
}

impl Default for ServerSpoof {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ServerSpoof {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.silent_accept_resource_pack = false;
        self.spoofed_brands = 0;
        self.blocked_channels = 0;
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        let is_custom_payload = super::packet_matches(&packet.name, "custom_payload");
        if is_custom_payload {
            let channel = super::packet_field(packet, &["channel", "id", "type"]).unwrap_or("");
            let channel = channel.to_string();

            if self.block_channels.get()
                && self
                    .channels
                    .get()
                    .iter()
                    .any(|keyword| super::id_contains(&channel, keyword))
            {
                self.blocked_channels += 1;
                return false;
            }

            if self.spoof_brand.get() && super::same_id(&channel, BRAND_CHANNEL) {
                let brand = self.brand.get();
                env.send_packet(PacketInfo {
                    name: packet.name.clone(),
                    clientbound: false,
                    fields: vec![
                        ("channel".to_string(), BRAND_CHANNEL.to_string()),
                        ("brand".to_string(), brand),
                    ],
                });
                self.spoofed_brands += 1;
                return false;
            }
        }

        // Accept the pack silently so the server does not see the bypass on login.
        if self.silent_accept_resource_pack && super::packet_matches(&packet.name, "resource_pack")
        {
            return false;
        }

        true
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !self.resource_pack.get() || !super::packet_matches(&packet.name, "resource_pack_push") {
            return true;
        }

        let id = super::packet_field(packet, &["id", "uuid"])
            .unwrap_or("")
            .to_string();
        let url = super::packet_field(packet, &["url"])
            .unwrap_or("")
            .to_string();
        let required = super::packet_field(packet, &["required"])
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        for action in ["accepted", "downloaded", "successfully_loaded"] {
            env.send_packet(PacketInfo {
                name: "ServerboundResourcePackPacket".to_string(),
                clientbound: false,
                fields: vec![
                    ("id".to_string(), id.clone()),
                    ("action".to_string(), action.to_string()),
                ],
            });
        }

        // Java's `[Open URL]` button refuses anything but http(s).
        if Self::parsed_resource_pack_url(&url).is_none() {
            self.base
                .error(env, &format!("Invalid resource pack URL: {url}"));
        }

        self.base.info(env, &self.pack_message(required));
        false
    }

    fn info_string(&self) -> Option<String> {
        if self.spoof_brand.get() {
            Some(self.brand.get())
        } else {
            None
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(ServerSpoof::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn payload(channel: &str) -> PacketInfo {
        PacketInfo {
            name: "ServerboundCustomPayloadPacket".to_string(),
            clientbound: false,
            fields: vec![("channel".to_string(), channel.to_string())],
        }
    }

    #[test]
    fn brand_payload_is_replaced_and_channels_blocked() {
        let mut m = ServerSpoof::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.brand.set("vanilla");

        let mut brand = payload(BRAND_CHANNEL);
        assert!(
            !m.on_send_packet(&mut env, &mut brand),
            "the client's own brand payload is cancelled"
        );
        assert_eq!(env.packets.len(), 1);
        assert_eq!(
            env.packets[0].fields[1],
            ("brand".to_string(), "vanilla".to_string())
        );

        let mut fabric = payload("fabric:registry/sync");
        assert!(!m.on_send_packet(&mut env, &mut fabric));
        assert_eq!(m.blocked_channels, 1);

        let mut other = payload("minecraft:chat_command");
        assert!(m.on_send_packet(&mut env, &mut other));
        assert_eq!(m.info_string().as_deref(), Some("vanilla"));
    }

    #[test]
    fn resource_pack_push_is_answered_and_cancelled() {
        let mut m = ServerSpoof::new();
        let mut env = TestEnv::in_world();
        m.resource_pack.set(true);

        let mut push = PacketInfo {
            name: "ClientboundResourcePackPushPacket".to_string(),
            clientbound: true,
            fields: vec![
                ("id".to_string(), "abc".to_string()),
                (
                    "url".to_string(),
                    "https://example.net/pack.zip".to_string(),
                ),
                ("required".to_string(), "true".to_string()),
            ],
        };
        assert!(!m.on_receive_packet(&mut env, &mut push));
        assert_eq!(
            env.packets.len(),
            3,
            "accepted, downloaded and loaded go out"
        );
        assert_eq!(env.packets[2].fields[1].1, "successfully_loaded");
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("This server has a required resource pack.")));
        assert_eq!(
            ServerSpoof::parsed_resource_pack_url("ftp://example.net/pack.zip"),
            None
        );
    }
}

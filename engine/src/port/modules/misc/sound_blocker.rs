//! `systems.modules.misc.SoundBlocker`.
//!
//! Java cancels `PlaySoundEvent` in `SoundEngineMixin` and stops tickable sounds
//! that match the list. The port sees the decoded `ClientboundSoundPacket` /
//! `ClientboundSoundEntityPacket` on the receive hook and cancels it before the
//! client turns it into a `SoundInstance`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::ListSet;

pub struct SoundBlocker {
    base: ModuleBase,
    // General
    sounds: ListSet,
    blocked: u32,
}

impl SoundBlocker {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "sound-blocker",
            "Cancels out selected sounds.",
        );

        let s = base.settings_mut();
        let sounds = s.default_group().list_setting("sounds", &[]);
        s.describe_last("Sounds to block.");

        Self {
            base,
            sounds,
            blocked: 0,
        }
    }

    /// `SoundBlocker.shouldBlock(SoundInstance)`.
    pub fn should_block(&self, sound_id: &str) -> bool {
        self.sounds
            .get()
            .iter()
            .any(|wanted| super::same_id(wanted, sound_id))
    }

    pub fn blocked(&self) -> u32 {
        self.blocked
    }

    /// The sound a sound packet carries, under whichever key the hook used.
    pub fn packet_sound(packet: &PacketInfo) -> Option<&str> {
        let is_sound = super::packet_matches(&packet.name, "sound")
            || super::packet_matches(&packet.name, "sound_entity");
        if !is_sound {
            return None;
        }
        super::packet_field(packet, &["sound", "sound_id", "id", "event"])
    }
}

impl Default for SoundBlocker {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SoundBlocker {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.blocked = 0;
    }

    fn on_receive_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        let Some(sound) = Self::packet_sound(packet) else {
            return true;
        };
        if self.should_block(sound) {
            self.blocked += 1;
            return false;
        }
        true
    }

    fn info_string(&self) -> Option<String> {
        let n = self.sounds.get().len();
        if n == 0 {
            None
        } else {
            Some(n.to_string())
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(SoundBlocker::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn sound_packet(id: &str) -> PacketInfo {
        PacketInfo {
            name: "ClientboundSoundPacket".to_string(),
            clientbound: true,
            fields: vec![("sound".to_string(), id.to_string())],
        }
    }

    #[test]
    fn cancels_only_the_listed_sounds() {
        let mut m = SoundBlocker::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.sounds
            .set(vec!["minecraft:block.note_block.harp".to_string()]);

        assert!(!m.on_receive_packet(
            &mut env,
            &mut sound_packet("minecraft:block.note_block.harp")
        ));
        assert!(m.on_receive_packet(&mut env, &mut sound_packet("minecraft:entity.player.hurt")));
        assert_eq!(m.blocked(), 1);

        // Non-sound packets are left alone even when they carry an `id` field.
        let mut other = PacketInfo {
            name: "minecraft:keep_alive".to_string(),
            clientbound: true,
            fields: vec![(
                "id".to_string(),
                "minecraft:block.note_block.harp".to_string(),
            )],
        };
        assert!(m.on_receive_packet(&mut env, &mut other));
        assert_eq!(m.info_string().as_deref(), Some("1"));
    }

    #[test]
    fn sound_ids_match_without_the_namespace() {
        let mut m = SoundBlocker::new();
        m.sounds.set(vec!["block.note_block.bass".to_string()]);
        assert!(m.should_block("minecraft:block.note_block.bass"));
        assert!(m.should_block("block.note_block.bass"));
        assert!(!m.should_block("minecraft:block.note_block.harp"));
    }
}

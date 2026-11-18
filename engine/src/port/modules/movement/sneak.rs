//! `systems.modules.movement.Sneak` - sneaks for you.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::EnumSet;

pub struct Sneak {
    base: ModuleBase,
    // General
    /// `mode`
    mode: EnumSet,
}

impl Sneak {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Movement, "sneak", "Sneaks for you");
        let mut s = base.settings_mut();
        let mode = s
            .default_group()
            .enum_setting("mode", &["Packet", "Vanilla"], 1);
        s.describe_last("Which method to sneak.");

        Self { base, mode }
    }

    /// `doPacket()`.
    pub fn do_packet(&self, env: &dyn Env) -> bool {
        // PORT GAP: `!player.getAbilities().flying` is not in `PlayerInfo`; the
        // flight module being on stands in for it.
        !env.module_active("flight") && self.mode.is("Packet")
    }

    /// `doVanilla()`.
    pub fn do_vanilla(&self, env: &dyn Env) -> bool {
        !env.module_active("flight") && self.mode.is("Vanilla")
    }
}

impl Default for Sneak {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Sneak {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if env.player().is_none() {
            return;
        }

        if self.do_vanilla(env) {
            env.action(Action::Sneak(true));
        } else if self.do_packet(env) {
            env.send_packet(PacketInfo {
                name: "ServerboundPlayerCommandPacket".to_string(),
                clientbound: false,
                fields: vec![("action".to_string(), "PRESS_SHIFT_KEY".to_string())],
            });
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.mode.get().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Sneak::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn vanilla_mode_holds_sneak() {
        let mut m = Sneak::new();
        let mut env = TestEnv::in_world();

        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Sneak(true)]);
    }

    #[test]
    fn packet_mode_sends_the_press() {
        let mut m = Sneak::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(0); // Packet

        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
        assert_eq!(env.packets.len(), 1);
        assert_eq!(
            env.packets[0].fields[0].1, "PRESS_SHIFT_KEY",
            "fields were {:?}",
            env.packets[0].fields
        );
    }
}

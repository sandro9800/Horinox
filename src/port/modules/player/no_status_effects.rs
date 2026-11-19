//! `systems.modules.player.NoStatusEffects` - blocks specified status effects.
//!
//! Java's mixin drops the effect before the client applies it; the port does the
//! same on the decoded `entity_effect` packet.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::ListSet;

use super::{bare, describe, field};

pub struct NoStatusEffects {
    base: ModuleBase,
    blocked_effects: ListSet,
    enabled: bool,
}

impl NoStatusEffects {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "no-status-effects",
            "Blocks specified status effects.",
        );
        let blocked_effects;
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            blocked_effects = g.list_setting(
                "blocked-effects",
                &[
                    "minecraft:levitation",
                    "minecraft:jump_boost",
                    "minecraft:slow_falling",
                    "minecraft:dolphins_grace",
                ],
            );
        }
        let mut s = base.settings_mut();
        describe(&mut s, "blocked-effects", "Effects to block.");

        Self {
            base,
            blocked_effects,
            enabled: false,
        }
    }

    /// `NoStatusEffects.shouldBlock(effect)`.
    pub fn should_block(&self, effect: &str) -> bool {
        self.enabled
            && (self.blocked_effects.contains(effect)
                || self.blocked_effects.contains(bare(effect)))
    }
}

impl Default for NoStatusEffects {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for NoStatusEffects {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.enabled = true;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.enabled = false;
    }

    fn on_receive_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !packet.clientbound || !packet.name.eq_ignore_ascii_case("entity_effect") {
            return true;
        }
        let Some(effect) = field(packet, "effect").map(|e| e.to_string()) else {
            return true;
        };
        !self.should_block(&effect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn effect(id: &str) -> PacketInfo {
        PacketInfo {
            name: "entity_effect".to_string(),
            clientbound: true,
            fields: vec![("effect".to_string(), id.to_string())],
        }
    }

    #[test]
    fn levitation_is_blocked_and_regeneration_is_not() {
        let mut m = NoStatusEffects::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);

        let mut levitation = effect("minecraft:levitation");
        assert!(!m.on_receive_packet(&mut env, &mut levitation));

        let mut regeneration = effect("minecraft:regeneration");
        assert!(m.on_receive_packet(&mut env, &mut regeneration));
        assert!(m.should_block("minecraft:jump_boost"));
        assert!(!m.should_block("minecraft:regeneration"));
    }
}

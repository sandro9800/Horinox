//! `systems.modules.combat.AutoTotem` -?Automatically equips a totem in your offhand.
//!
//! Ported from the Java class's `TickEvent.Pre` handler and its
//! `ClientboundEntityEventPacket` (totem pop) listener: the "lock" decision uses
//! the same health-minus-possible-damage comparison, and the move to the off hand
//! is the same `InvUtils.move().from(slot).toOffhand()` click.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, PacketInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet};

use super::{count_item, move_to_offhand, possible_health_reductions, OFFHAND_SLOT};

pub struct AutoTotem {
    base: ModuleBase,
    mode: EnumSet,
    delay: IntSet,
    health: IntSet,
    elytra: BoolSet,
    fall: BoolSet,
    explosion: BoolSet,
    // Runtime
    locked: bool,
    totems: i32,
    ticks: i32,
}

impl AutoTotem {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "auto-totem",
            "Automatically equips a totem in your offhand.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let mode = g.enum_setting("mode", &["smart", "strict"], 0);
        let delay = g.int_setting("delay", 0, 0, 20);
        let health = g.int_setting("health", 10, 0, 36);
        let elytra = g.bool_setting("elytra", true);
        let fall = g.bool_setting("fall", true);
        let explosion = g.bool_setting("explosion", true);
        s.describe_last("Will hold a totem when explosion damage could kill you.");

        Self {
            base,
            mode,
            delay,
            health,
            elytra,
            fall,
            explosion,
            locked: false,
            totems: 0,
            ticks: 0,
        }
    }

    /// `AutoTotem.isLocked()`, which the Crystal Aura reads.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// `mc.player.getOffhandItem().getItem() == Items.TOTEM_OF_UNDYING`.
    fn offhand_holds_totem(env: &dyn Env) -> bool {
        env.inventory()
            .into_iter()
            .find(|s| s.index == OFFHAND_SLOT)
            .and_then(|s| s.stack)
            .map(|s| s.id == "minecraft:totem_of_undying")
            .unwrap_or(false)
    }
}

impl Default for AutoTotem {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoTotem {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let found = super::find_item(env, &["minecraft:totem_of_undying"]);
        self.totems = count_item(env, &["minecraft:totem_of_undying"]);

        if self.totems <= 0 {
            self.locked = false;
            return;
        }
        if self.ticks < self.delay.get() {
            self.ticks += 1;
            return;
        }

        let Some(player) = env.player() else { return };
        let reductions = possible_health_reductions(env, self.explosion.get(), self.fall.get());
        let low = player.health + player.absorption - reductions <= self.health.get() as f32;
        // PORT GAP: `mc.player.getItemBySlot(CHEST) == Items.ELYTRA &&
        // isFallFlying()` needs the player's equipment and elytra state, which
        // `PlayerInfo` does not carry; `possible_health_reductions(fall)` covers
        // the fall that elytra flying is meant to prevent.
        let _ = self.elytra.get();

        self.locked = self.mode.is("strict") || (self.mode.is("smart") && low);

        if self.locked && !Self::offhand_holds_totem(env) {
            if let Some((slot, _)) = found {
                move_to_offhand(env, slot);
            }
        }

        self.ticks = 0;
    }

    fn on_receive_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        // `ClientboundEntityEventPacket` with `EntityEvent.PROTECTED_FROM_DEATH`
        // (35) for the local player resets the delay.
        if !packet.name.contains("entity_event") {
            return true;
        }
        let event = packet
            .fields
            .iter()
            .find(|(k, _)| k == "event_id" || k == "event" || k == "id")
            .map(|(_, v)| v.as_str());
        if event == Some("35") {
            self.ticks = 0;
        }
        true
    }

    fn info_string(&self) -> Option<String> {
        Some(self.totems.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo, PacketInfo};

    fn env_with_totem(health: f32) -> TestEnv {
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 5,
            stack: Some(ItemStackInfo {
                id: "minecraft:totem_of_undying".into(),
                count: 2,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().health = health;
        env
    }

    #[test]
    fn moves_a_totem_to_the_offhand_when_low() {
        let mut m = AutoTotem::new();
        let mut env = env_with_totem(4.0);
        m.on_tick(&mut env);

        assert!(m.is_locked(), "a 4 hp player should lock the totem");
        assert_eq!(
            env.clicks,
            vec![(5, OFFHAND_SLOT as i32, ClickMode::Swap)],
            "the totem should be moved to the off hand"
        );
        assert_eq!(m.info_string().as_deref(), Some("2"));
    }

    #[test]
    fn keeps_a_totem_in_hand_when_healthy() {
        let mut m = AutoTotem::new();
        let mut env = env_with_totem(20.0);
        m.on_tick(&mut env);

        assert!(!m.is_locked());
        assert!(env.clicks.is_empty());
    }

    #[test]
    fn strict_mode_always_holds() {
        let mut m = AutoTotem::new();
        let mut env = env_with_totem(20.0);
        m.mode.set_index(1);
        m.on_tick(&mut env);
        assert!(m.is_locked());
        assert_eq!(env.clicks.len(), 1);
    }

    #[test]
    fn delay_is_reset_by_a_totem_pop() {
        let mut m = AutoTotem::new();
        let mut env = env_with_totem(4.0);
        m.delay.set(1);
        m.on_tick(&mut env); // ticks 0 -> 1, no move yet
        assert!(env.clicks.is_empty());

        let mut pop = PacketInfo {
            name: "minecraft:entity_event".into(),
            clientbound: true,
            fields: vec![("event_id".into(), "35".into())],
        };
        m.on_receive_packet(&mut env, &mut pop);
        assert_eq!(m.ticks, 0);
    }

    #[test]
    fn mode_setting_is_the_java_enum() {
        let m = AutoTotem::new();
        assert_eq!(m.mode.names(), ["smart", "strict"].as_slice());
        assert!(m.base.settings.get("health").is_some());
    }
}

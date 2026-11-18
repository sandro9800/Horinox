//! `systems.modules.player.AutoGap` - automatically eats Gaps or E-Gaps.
//!
//! PORT GAP: the potion half of `shouldEatPotions` needs the player's active
//! effects (`Player.getActiveEffectsMap`), which `PlayerInfo` does not carry.
//! Every potion check therefore stays off and the module eats from the health
//! half only; the settings are still there, in the Java order and wording.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env, ItemStackInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet};

use super::{bare, describe, is_hotbar, select_hotbar, stack, visible, HOTBAR_END, HOTBAR_START};

/// The auras AutoGap pauses while it eats (`AutoGap.AURAS`).
const AURAS: [&str; 4] = ["kill-aura", "crystal-aura", "anchor-aura", "bed-aura"];

pub struct AutoGap {
    base: ModuleBase,
    // General
    allow_egap: BoolSet,
    always: BoolSet,
    pause_auras: BoolSet,
    pause_baritone: BoolSet,
    // Potions
    before_expiry: BoolSet,
    expiry_threshold: IntSet,
    potions_regeneration: BoolSet,
    potions_fire_resistance: BoolSet,
    potions_absorption: BoolSet,
    // Health
    health_enabled: BoolSet,
    health_threshold: IntSet,
    // Runtime
    requires_egap: bool,
    eating: bool,
    slot: Option<usize>,
    prev_slot: Option<usize>,
    was_aura: Vec<&'static str>,
    was_baritone: bool,
}

impl AutoGap {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "auto-gap",
            "Automatically eats Gaps or E-Gaps.",
        );
        let (allow_egap, always, pause_auras, pause_baritone);
        let (before_expiry, expiry_threshold, potions_regeneration, potions_fire_resistance);
        let (potions_absorption, health_enabled, health_threshold);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            allow_egap = g.bool_setting("allow-egap", true);
            always = g.bool_setting("always", false);
            pause_auras = g.bool_setting("pause-auras", true);
            pause_baritone = g.bool_setting("pause-baritone", true);
            let p = s.create_group("Potions");
            before_expiry = p.bool_setting("before-expiry", false);
            expiry_threshold = p.int_setting("expiry-threshold", 60, 0, 200);
            potions_regeneration = p.bool_setting("potions-regeneration", false);
            potions_fire_resistance = p.bool_setting("potions-fire-resistance", true);
            potions_absorption = p.bool_setting("potions-absorption", false);
            let h = s.create_group("Health");
            health_enabled = h.bool_setting("health-enabled", true);
            health_threshold = h.int_setting("health-threshold", 20, 0, 40);
        }
        let mut s = base.settings_mut();
        describe(
            &mut s,
            "allow-egap",
            "Allow eating E-Gaps over Gaps if found.",
        );
        describe(&mut s, "always", "If it should always eat.");
        describe(&mut s, "pause-auras", "Pauses all auras when eating.");
        describe(&mut s, "pause-baritone", "Pause baritone when eating.");
        describe(
            &mut s,
            "before-expiry",
            "If it should eat before potion effects expire.",
        );
        describe(
            &mut s,
            "expiry-threshold",
            "Time in ticks before the potion effect expires to start eating.",
        );
        describe(
            &mut s,
            "potions-regeneration",
            "If it should eat when Regeneration runs out.",
        );
        describe(
            &mut s,
            "potions-fire-resistance",
            "If it should eat when Fire Resistance runs out. Requires E-Gaps.",
        );
        describe(
            &mut s,
            "potions-absorption",
            "If it should eat when Absorption runs out. Requires E-Gaps.",
        );
        describe(
            &mut s,
            "health-enabled",
            "If it should eat when health drops below threshold.",
        );
        describe(
            &mut s,
            "health-threshold",
            "Health threshold to eat at. Includes absorption.",
        );
        visible(&mut s, "expiry-threshold", {
            let c = before_expiry.clone();
            move || c.get()
        });
        visible(&mut s, "potions-fire-resistance", {
            let c = allow_egap.clone();
            move || c.get()
        });
        visible(&mut s, "potions-absorption", {
            let c = allow_egap.clone();
            move || c.get()
        });

        Self {
            base,
            allow_egap,
            always,
            pause_auras,
            pause_baritone,
            before_expiry,
            expiry_threshold,
            potions_regeneration,
            potions_fire_resistance,
            potions_absorption,
            health_enabled,
            health_threshold,
            requires_egap: false,
            eating: false,
            slot: None,
            prev_slot: None,
            was_aura: Vec::new(),
            was_baritone: false,
        }
    }

    fn is_not_gap_or_egap(stack: &ItemStackInfo) -> bool {
        !matches!(bare(&stack.id), "golden_apple" | "enchanted_golden_apple")
    }

    /// `AutoGap.findSlot`: an E-Gap when allowed, otherwise a Gap.
    fn find_slot(&self, env: &dyn Env) -> Option<usize> {
        for i in HOTBAR_START..=HOTBAR_END {
            let Some(stack) = stack(env, i) else { continue };
            if Self::is_not_gap_or_egap(&stack) {
                continue;
            }
            let id = bare(&stack.id);
            if id == "enchanted_golden_apple" && self.allow_egap.get() {
                return Some(i);
            }
            if id == "golden_apple" && !self.requires_egap {
                return Some(i);
            }
        }
        None
    }

    /// `AutoGap.shouldEat`, which also resets `requiresEGap`.
    pub fn should_eat(&mut self, env: &dyn Env) -> bool {
        self.requires_egap = false;
        if self.always.get() {
            return true;
        }
        if self.should_eat_potions() {
            return true;
        }
        self.should_eat_health(env)
    }

    /// PORT GAP: no active-effect list in `Env`; see the module docs.
    fn should_eat_potions(&self) -> bool {
        let _ = (
            self.before_expiry.get(),
            self.expiry_threshold.get(),
            self.potions_regeneration.get(),
            self.potions_fire_resistance.get(),
            self.potions_absorption.get(),
        );
        false
    }

    fn should_eat_health(&self, env: &dyn Env) -> bool {
        if !self.health_enabled.get() {
            return false;
        }
        let Some(player) = env.player() else {
            return false;
        };
        let health = (player.health + player.absorption).round() as i32;
        health < self.health_threshold.get()
    }

    fn change_slot(&mut self, env: &mut dyn Env, slot: usize) {
        if is_hotbar(slot) {
            select_hotbar(env, slot);
            self.slot = Some(slot);
        }
    }

    fn eat(&mut self, env: &mut dyn Env) {
        let Some(slot) = self.slot else { return };
        self.change_slot(env, slot);
        // `setPressed(true)` + `Utils.rightClick()`
        let using = env.player().map(|p| p.using_item).unwrap_or(false);
        if !using {
            env.action(Action::Use);
        }
        self.eating = true;
    }

    fn start_eating(&mut self, env: &mut dyn Env) {
        self.prev_slot = env.player().map(|p| p.selected_slot);
        self.eat(env);
        if !self.eating {
            return;
        }

        self.was_aura.clear();
        if self.pause_auras.get() {
            for name in AURAS {
                if env.module_active(name) {
                    self.was_aura.push(name);
                    env.request_toggle(name);
                }
            }
        }

        // PORT GAP: `PathManagers.get().pause()`.
        self.was_baritone = false;
    }

    fn stop_eating(&mut self, env: &mut dyn Env) {
        if let Some(prev) = self.prev_slot {
            self.change_slot(env, prev);
        }
        self.eating = false;

        if self.pause_auras.get() {
            for name in std::mem::take(&mut self.was_aura) {
                env.request_toggle(name);
            }
        } else {
            self.was_aura.clear();
        }

        // PORT GAP: `PathManagers.get().resume()`.
        if self.was_baritone {
            self.was_baritone = false;
        }
    }

    pub fn is_eating(&self) -> bool {
        self.eating
    }
}

impl Default for AutoGap {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoGap {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.eating = false;
        self.was_aura.clear();
        self.slot = None;
        self.prev_slot = None;
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        if self.eating {
            self.stop_eating(env);
        }
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.eating {
            if self.should_eat(env) {
                let current = self.slot;
                let still_gap = current
                    .and_then(|s| stack(env, s))
                    .map(|s| !Self::is_not_gap_or_egap(&s))
                    .unwrap_or(false);
                if !still_gap {
                    let Some(slot) = self.find_slot(env) else {
                        self.stop_eating(env);
                        return;
                    };
                    self.change_slot(env, slot);
                }
                self.eat(env);
            } else {
                self.stop_eating(env);
            }
        } else if self.should_eat(env) {
            if let Some(slot) = self.find_slot(env) {
                self.slot = Some(slot);
                self.start_eating(env);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::InvSlot;
    use crate::port::modules::player::{field, HOTBAR_START};

    fn gap() -> ItemStackInfo {
        ItemStackInfo {
            id: "minecraft:golden_apple".to_string(),
            count: 16,
            ..Default::default()
        }
    }

    #[test]
    fn low_health_selects_and_eats_a_gap() {
        let mut m = AutoGap::new();
        let mut env = TestEnv::in_world();
        if let Some(player) = env.player.as_mut() {
            player.health = 10.0;
            player.absorption = 0.0;
        }
        env.inventory.push(InvSlot {
            index: HOTBAR_START + 4,
            stack: Some(gap()),
        });

        m.on_tick(&mut env);

        assert!(m.is_eating(), "a gap is eaten below the health threshold");
        assert!(env
            .packets
            .iter()
            .any(|p| p.name == "set_carried_item" && field(p, "slot") == Some("4")));
        assert!(env.actions.contains(&Action::Use));
    }

    #[test]
    fn full_health_leaves_the_gap_alone() {
        let mut m = AutoGap::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(gap()),
        });

        m.on_tick(&mut env);
        assert!(!m.is_eating());
        assert!(env.actions.is_empty());
    }
}

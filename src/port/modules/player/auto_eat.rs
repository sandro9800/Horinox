//! `systems.modules.player.AutoEat` - automatically eats food.
//!
//! PORT GAP: `Modules.get().get(AutoGap.class).isEating()` and
//! `PathManagers.get()` (baritone) are cross-system state the `Env` does not
//! carry; the port keeps the same pause bookkeeping but only the auras can be
//! resumed through `request_toggle`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env, ItemStackInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet, ItemListSet};

use super::{
    bare, can_always_eat, describe, find_empty, food, is_hotbar, move_stack, offhand,
    select_hotbar, stack, visible, HOTBAR_END, HOTBAR_START, MAIN_END, MAIN_START, OFFHAND,
};

/// The auras AutoEat pauses while it eats (`AutoEat.AURAS`).
const AURAS: [&str; 4] = ["kill-aura", "crystal-aura", "anchor-aura", "bed-aura"];

pub struct AutoEat {
    base: ModuleBase,
    // General
    blacklist: ItemListSet,
    pause_auras: BoolSet,
    pause_baritone: BoolSet,
    search_inventory: BoolSet,
    priority: EnumSet,
    // Threshold
    threshold_mode: EnumSet,
    health_threshold: DoubleSet,
    hunger_threshold: IntSet,
    // Runtime
    /// `AutoEat.eating`, which `AutoGap` and the HUD read.
    pub eating: bool,
    slot: Option<usize>,
    prev_slot: Option<usize>,
    was_aura: Vec<&'static str>,
    was_baritone: bool,
}

impl AutoEat {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Player, "auto-eat", "Automatically eats food.");
        let (blacklist, pause_auras, pause_baritone, search_inventory, priority);
        let (threshold_mode, health_threshold, hunger_threshold);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            blacklist = g.item_list_setting(
                "blacklist",
                &[
                    "minecraft:enchanted_golden_apple",
                    "minecraft:golden_apple",
                    "minecraft:chorus_fruit",
                    "minecraft:poisonous_potato",
                    "minecraft:pufferfish",
                    "minecraft:chicken",
                    "minecraft:rotten_flesh",
                    "minecraft:spider_eye",
                    "minecraft:suspicious_stew",
                ],
            );
            pause_auras = g.bool_setting("pause-auras", true);
            pause_baritone = g.bool_setting("pause-baritone", true);
            search_inventory = g.bool_setting("search-inventory", false);
            priority = g.enum_setting("food-priority", &["combined", "hunger", "saturation"], 2);
            let t = s.create_group("Threshold");
            threshold_mode =
                t.enum_setting("threshold-mode", &["health", "hunger", "any", "both"], 2);
            health_threshold = t.double_setting("health-threshold", 10.0, 1.0, 19.0, 1);
            hunger_threshold = t.int_setting("hunger-threshold", 16, 1, 19);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "blacklist", "Which items to not eat.");
        describe(&mut s, "pause-auras", "Pauses all auras when eating.");
        describe(&mut s, "pause-baritone", "Pause baritone when eating.");
        describe(
            &mut s,
            "search-inventory",
            "Search the full inventory for food, not only the hotbar.",
        );
        describe(
            &mut s,
            "food-priority",
            "Which aspect of the food to prioritise selecting for.",
        );
        describe(
            &mut s,
            "threshold-mode",
            "The threshold mode to trigger auto eat.\n'Both' == health AND hunger, 'Any' == health OR hunger",
        );
        describe(
            &mut s,
            "health-threshold",
            "The level of health you eat at.",
        );
        describe(
            &mut s,
            "hunger-threshold",
            "The level of hunger you eat at.",
        );
        visible(&mut s, "health-threshold", {
            let c = threshold_mode.clone();
            move || !c.is("hunger")
        });
        visible(&mut s, "hunger-threshold", {
            let c = threshold_mode.clone();
            move || !c.is("health")
        });

        Self {
            base,
            blacklist,
            pause_auras,
            pause_baritone,
            search_inventory,
            priority,
            threshold_mode,
            health_threshold,
            hunger_threshold,
            eating: false,
            slot: None,
            prev_slot: None,
            was_aura: Vec::new(),
            was_baritone: false,
        }
    }

    fn is_edible_stack(&self, env: &dyn Env, stack: &ItemStackInfo) -> bool {
        if food(&stack.id).is_none() {
            return false;
        }
        if self.blacklist.contains(&stack.id) || self.blacklist.contains(bare(&stack.id)) {
            return false;
        }
        let needs_food = env.player().map(|p| p.food < 20).unwrap_or(false);
        needs_food || can_always_eat(&stack.id)
    }

    fn is_edible(&self, env: &dyn Env, slot: usize) -> bool {
        match stack(env, slot) {
            Some(stack) => self.is_edible_stack(env, &stack),
            None => false,
        }
    }

    /// `AutoEat.findBestFood`: the food with the best priority value in a range.
    fn find_best_food(&self, env: &dyn Env, start: usize, end: usize) -> Option<usize> {
        let mut best = None;
        let mut best_value = -1.0f32;

        for i in start..=end {
            if !self.is_edible(env, i) {
                continue;
            }
            let Some(stack) = stack(env, i) else { continue };
            let Some((nutrition, saturation)) = food(&stack.id) else {
                continue;
            };
            let value = match self.priority.get() {
                "combined" => nutrition + saturation,
                "hunger" => nutrition,
                _ => saturation,
            };
            if value > best_value {
                best_value = value;
                best = Some(i);
            }
        }

        best
    }

    /// `AutoEat.findSlot`: off hand, then the hotbar, then the main inventory.
    fn find_slot(&self, env: &dyn Env) -> Option<usize> {
        if self.is_edible(env, OFFHAND) {
            return Some(OFFHAND);
        }
        if let Some(slot) = self.find_best_food(env, HOTBAR_START, HOTBAR_END) {
            return Some(slot);
        }
        if self.search_inventory.get() {
            return self.find_best_food(env, MAIN_START, MAIN_END);
        }
        None
    }

    /// `AutoEat.shouldEat`, which also picks the slot it would eat from.
    pub fn should_eat(&mut self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        let health_low = player.health <= self.health_threshold.get() as f32;
        let hunger_low = player.food <= self.hunger_threshold.get();
        let triggered = match self.threshold_mode.get() {
            "health" => health_low,
            "hunger" => hunger_low,
            "both" => health_low && hunger_low,
            _ => health_low || hunger_low,
        };
        if !triggered {
            return false;
        }
        self.slot = self.find_slot(env);
        self.slot.is_some()
    }

    /// `AutoEat.changeSlot`: off hand or hotbar directly, otherwise move the
    /// stack into an empty hotbar slot first.
    fn change_slot(&mut self, env: &mut dyn Env, slot: usize) -> bool {
        if slot == OFFHAND {
            self.slot = Some(OFFHAND);
            return true;
        }
        if is_hotbar(slot) {
            select_hotbar(env, slot);
            self.slot = Some(slot);
            return true;
        }
        let Some(empty) = find_empty(env, HOTBAR_START, HOTBAR_END) else {
            return false;
        };
        move_stack(env, slot, empty);
        select_hotbar(env, empty);
        self.slot = Some(empty);
        true
    }

    fn eat(&mut self, env: &mut dyn Env) {
        let Some(slot) = self.slot else { return };
        if !self.change_slot(env, slot) {
            return;
        }
        // `setPressed(true)` + `Utils.rightClick()`: `Env` has no key state, so
        // the use itself is the action.
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

        // PORT GAP: `PathManagers.get().pause()` - Horizon has no pathing system.
        self.was_baritone = false;
    }

    fn stop_eating(&mut self, env: &mut dyn Env) {
        if let Some(prev) = self.prev_slot {
            if prev != OFFHAND {
                self.change_slot(env, prev);
            }
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
}

impl Default for AutoEat {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoEat {
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
        let Some(_player) = env.player() else { return };

        // PORT GAP: `Modules.get().get(AutoGap.class).isEating()` - the port
        // cannot read another module's runtime state, only its active flag.
        if self.eating {
            if !self.should_eat(env) {
                self.stop_eating(env);
                return;
            }

            let current = self.slot;
            let still_edible = current.map(|s| self.is_edible(env, s)).unwrap_or(false);
            if !still_edible {
                let Some(new_slot) = self.find_slot(env) else {
                    self.stop_eating(env);
                    return;
                };
                self.change_slot(env, new_slot);
            }

            self.eat(env);
            return;
        }

        if self.should_eat(env) {
            self.start_eating(env);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::InvSlot;
    use crate::port::modules::player::HOTBAR_START;

    fn item(id: &str, count: i32) -> ItemStackInfo {
        ItemStackInfo {
            id: id.to_string(),
            count,
            ..Default::default()
        }
    }

    #[test]
    fn low_food_selects_and_eats_from_the_hotbar() {
        let mut m = AutoEat::new();
        let mut env = TestEnv::in_world();
        if let Some(player) = env.player.as_mut() {
            player.health = 20.0;
            player.food = 6;
            player.selected_slot = 0;
        }
        env.inventory.push(InvSlot {
            index: HOTBAR_START + 3,
            stack: Some(item("minecraft:bread", 42)),
        });

        m.on_tick(&mut env);

        assert!(m.eating, "auto-eat starts eating when hunger is low");
        assert!(
            env.packets.iter().any(|p| p.name == "set_carried_item"
                && p.fields.iter().any(|(k, v)| k == "slot" && v == "3")),
            "the food slot is selected, got {:?}",
            env.packets
        );
        assert!(env.actions.contains(&Action::Use), "the food is used");
    }

    #[test]
    fn full_hunger_and_health_never_eats() {
        let mut m = AutoEat::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(item("minecraft:bread", 42)),
        });

        m.on_tick(&mut env);
        assert!(!m.eating);
        assert!(env.actions.is_empty());
    }
}

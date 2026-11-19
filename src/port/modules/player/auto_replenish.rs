//! `systems.modules.player.AutoReplenish` - automatically refills items in your
//! hotbar, main hand, or offhand.
//!
//! PORT GAP: `mc.player.containerMenu.getItems().size() != 46` (the "no container
//! open" guard) has no equivalent - the port checks that no screen is open
//! instead. `AutoTotem.isLocked()` is read through `module_settings`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, ItemStackInfo, Screen};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, ItemListSet};

use super::{
    bare, describe, is, is_stackable, move_stack, offhand, stack, visible, ARMOR_END, HOTBAR_END,
    HOTBAR_START, OFFHAND,
};

pub struct AutoReplenish {
    base: ModuleBase,
    min_count: IntSet,
    tick_delay: IntSet,
    offhand: BoolSet,
    unstackable: BoolSet,
    same_enchants: BoolSet,
    search_hotbar: BoolSet,
    excluded_items: ItemListSet,
    /// `items`: indices 0-8 are the hotbar, index 9 the off hand.
    items: [Option<ItemStackInfo>; 10],
    prev_had_open_screen: bool,
    tick_delay_left: i32,
}

impl AutoReplenish {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "auto-replenish",
            "Automatically refills items in your hotbar, main hand, or offhand.",
        );
        let (min_count, tick_delay, offhand_setting, unstackable, same_enchants, search_hotbar);
        let excluded_items;
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            min_count = g.int_setting("min-count", 8, 1, 63);
            tick_delay = g.int_setting("delay", 1, 0, 100);
            offhand_setting = g.bool_setting("offhand", true);
            unstackable = g.bool_setting("unstackable", true);
            same_enchants = g.bool_setting("same-enchants", true);
            search_hotbar = g.bool_setting("search-hotbar", false);
            excluded_items = g.item_list_setting("excluded-items", &[]);
        }
        let mut s = base.settings_mut();
        describe(
            &mut s,
            "min-count",
            "Replenish a slot when it reaches this item count.",
        );
        describe(
            &mut s,
            "delay",
            "How long in ticks to wait between replenishing your hotbar.",
        );
        describe(
            &mut s,
            "offhand",
            "Whether or not to replenish items in your offhand.",
        );
        describe(&mut s, "unstackable", "Replenish unstackable items.");
        describe(
            &mut s,
            "same-enchants",
            "Only replace unstackables with items that have the same enchants.",
        );
        describe(
            &mut s,
            "search-hotbar",
            "Combine stacks in your hotbar/offhand as a last resort.",
        );
        describe(&mut s, "excluded-items", "Items that won't be replenished.");
        visible(&mut s, "same-enchants", {
            let c = unstackable.clone();
            move || c.get()
        });

        Self {
            base,
            min_count,
            tick_delay,
            offhand: offhand_setting,
            unstackable,
            same_enchants,
            search_hotbar,
            excluded_items,
            items: std::array::from_fn(|_| None),
            prev_had_open_screen: false,
            tick_delay_left: 0,
        }
    }

    fn excluded(&self, id: &str) -> bool {
        self.excluded_items.contains(id) || self.excluded_items.contains(bare(id))
    }

    fn fill_items(&mut self, env: &dyn Env) {
        for i in 0..9 {
            self.items[i] = stack(env, HOTBAR_START + i);
        }
        self.items[9] = offhand(env);
    }

    /// `Modules.get().get(AutoTotem.class).isLocked()`, read from the snapshot.
    fn auto_totem_locked(&self, env: &dyn Env) -> bool {
        env.module_settings("auto-totem")
            .iter()
            .any(|(name, value)| name.eq_ignore_ascii_case("lock") && value == "true")
    }

    fn same_components(a: &ItemStackInfo, b: &ItemStackInfo) -> bool {
        a.id == b.id && a.damage == b.damage && a.name == b.name && a.enchantments == b.enchantments
    }

    /// `AutoReplenish.findItem`: the fullest matching stack outside `excluded_slot`.
    fn find_item(
        &self,
        env: &dyn Env,
        look_for: &ItemStackInfo,
        excluded_slot: usize,
        good_enough_count: i32,
        must_combine: bool,
    ) -> Option<usize> {
        let mut slot = None;
        let mut count = 0;
        let lower = if self.search_hotbar.get() {
            HOTBAR_START
        } else {
            9
        };

        for i in (lower..=ARMOR_END).rev() {
            if i == excluded_slot {
                continue;
            }
            let Some(stack) = stack(env, i) else { continue };
            if !is(&stack, &look_for.id) {
                continue;
            }
            if must_combine && !Self::same_components(look_for, &stack) {
                continue;
            }
            if self.same_enchants.get() && stack.enchantments != look_for.enchantments {
                continue;
            }
            if stack.count > count {
                slot = Some(i);
                count = stack.count;
                if count >= good_enough_count {
                    break;
                }
            }
        }

        slot
    }

    fn check_slot(&mut self, env: &mut dyn Env, slot: usize, current: Option<ItemStackInfo>) {
        let prev = self.items[slot].clone();
        self.items[slot] = current.clone();

        let inv_slot = if slot == 9 { OFFHAND } else { slot };

        if current
            .as_ref()
            .map(|s| self.excluded(&s.id))
            .unwrap_or(false)
        {
            return;
        }
        if prev.as_ref().map(|s| self.excluded(&s.id)).unwrap_or(false) {
            return;
        }

        let min_count = self.min_count.get();
        let empty_now = current.as_ref().map(|s| s.is_empty()).unwrap_or(true);
        let mut from_slot: Option<usize> = None;

        // Still some items left, but the stack just crossed the threshold.
        if let Some(stack) = current.as_ref() {
            if is_stackable(stack) && !stack.is_empty() && stack.count <= min_count {
                from_slot = self.find_item(env, stack, inv_slot, min_count - stack.count + 1, true);
            }
        }

        // The stack went from above the threshold to empty in one tick.
        if let Some(prev_stack) = prev.as_ref() {
            if is_stackable(prev_stack) && empty_now && !prev_stack.is_empty() {
                from_slot = self.find_item(env, prev_stack, inv_slot, min_count + 1, false);
            }

            // Unstackable items.
            if self.unstackable.get()
                && !is_stackable(prev_stack)
                && empty_now
                && !prev_stack.is_empty()
            {
                from_slot = self.find_item(env, prev_stack, inv_slot, 1, false);
            }
        }

        let Some(from) = from_slot else { return };
        let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);

        // Eliminate the occasional loops when moving items from the hotbar to itself.
        if from == selected || from == OFFHAND {
            return;
        }
        if from < 9 && from < inv_slot && inv_slot != selected && inv_slot != OFFHAND {
            return;
        }

        move_stack(env, from, inv_slot);
    }
}

impl Default for AutoReplenish {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoReplenish {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.fill_items(env);
        self.tick_delay_left = self.tick_delay.get();
        self.prev_had_open_screen = env.screen() != Screen::Playing;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let screen_open = env.screen() != Screen::Playing;
        if !screen_open && self.prev_had_open_screen {
            self.fill_items(env);
        }
        self.prev_had_open_screen = screen_open;

        if screen_open {
            return;
        }

        if self.tick_delay_left > 0 {
            self.tick_delay_left -= 1;
            return;
        }

        // Hotbar
        for i in 0..9 {
            let current = stack(env, HOTBAR_START + i);
            self.check_slot(env, i, current);
        }

        // Off hand
        if self.offhand.get() && !self.auto_totem_locked(env) {
            let current = offhand(env);
            self.check_slot(env, 9, current);
        }

        self.tick_delay_left = self.tick_delay.get();
    }
}

/// The click mode every `InvUtils.move()` uses, named here so the tests read the
/// same as the Java call.
const PICKUP: ClickMode = ClickMode::Pickup;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::InvSlot;
    use crate::port::modules::player::HOTBAR_START;

    fn cobble(count: i32) -> ItemStackInfo {
        ItemStackInfo {
            id: "minecraft:cobblestone".to_string(),
            count,
            ..Default::default()
        }
    }

    #[test]
    fn a_threshold_crossing_stack_is_refilled_from_the_main_inventory() {
        let mut m = AutoReplenish::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(cobble(2)),
        });
        env.inventory.push(InvSlot {
            index: 10,
            stack: Some(cobble(64)),
        });

        m.tick_delay.set(0);
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert_eq!(
            env.clicks,
            vec![(10, 0, PICKUP), (36, 0, PICKUP)],
            "main inventory slot 10 into hotbar slot 0"
        );
    }

    #[test]
    fn a_full_stack_is_left_alone() {
        let mut m = AutoReplenish::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(cobble(64)),
        });
        env.inventory.push(InvSlot {
            index: 10,
            stack: Some(cobble(64)),
        });

        m.tick_delay.set(0);
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert!(env.clicks.is_empty());
    }
}

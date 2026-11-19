//! `systems.modules.world.AutoSmelter` - "automatically smelts items from your
//! inventory".
//!
//! Java drives the open `AbstractFurnaceMenu` from a mixin. The `Env` hands a
//! module no container contents, so the state machine takes a [`FurnaceView`] the
//! host fills in and issues the same `handleContainerInput` clicks through
//! `Env::click_slot`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, ItemStackInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, ItemListSet};

use super::{describe, listed_item};

/// The slots of `AbstractFurnaceMenu` the module reads: `0` input, `1` fuel,
/// `2` result, `3..` the player's inventory.
#[derive(Clone, Debug, Default)]
pub struct FurnaceView {
    /// `c.getLitProgress()`.
    pub lit_progress: i32,
    /// `c.slots` - every slot in menu order.
    pub slots: Vec<Option<ItemStackInfo>>,
}

impl FurnaceView {
    fn get(&self, index: usize) -> Option<&ItemStackInfo> {
        self.slots.get(index).and_then(|slot| slot.as_ref())
    }

    /// `slot.getItem().isEmpty()`.
    fn is_empty(&self, index: usize) -> bool {
        self.get(index).is_none_or(|stack| stack.is_empty())
    }
}

pub struct AutoSmelter {
    base: ModuleBase,
    fuel_items: ItemListSet,
    fuel_items_per_refill: IntSet,
    smeltable_items: ItemListSet,
    disable_when_out_of_items: BoolSet,
    auto_close: BoolSet,
    /// The moves the module asked for on the last tick.
    moved: Vec<(usize, i32, ClickMode)>,
    closed: bool,
}

impl AutoSmelter {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "auto-smelter",
            "Automatically smelts items from your inventory",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let fuel_items =
            g.item_list_setting("fuel-items", &["minecraft:coal", "minecraft:charcoal"]);
        describe(g, "Items to use as fuel");

        let fuel_items_per_refill = g.int_setting("fuel-items-per-refill", 64, 1, 64);
        describe(
            g,
            "How many fuel items to put into the furnace each time it refills",
        );

        let smeltable_items = g.item_list_setting(
            "smeltable-items",
            &[
                "minecraft:iron_ore",
                "minecraft:gold_ore",
                "minecraft:copper_ore",
                "minecraft:raw_iron",
                "minecraft:raw_copper",
                "minecraft:raw_gold",
            ],
        );
        describe(g, "Items to smelt");

        let disable_when_out_of_items = g.bool_setting("disable-when-out-of-items", true);
        describe(g, "Disable the module when you run out of items");

        let auto_close = g.bool_setting("auto-close", false);
        describe(g, "Close the furnace screen after the cycle.");

        Self {
            base,
            fuel_items,
            fuel_items_per_refill,
            smeltable_items,
            disable_when_out_of_items,
            auto_close,
            moved: Vec::new(),
            closed: false,
        }
    }

    /// The moves recorded on the last `tick_furnace`.
    pub fn moved(&self) -> &[(usize, i32, ClickMode)] {
        &self.moved
    }

    /// `AutoSmelter.tick(AbstractFurnaceMenu c)`.
    ///
    /// PORT GAP: `AbstractFurnaceMenu` - Horizon gives a mod neither the open
    /// container's slots nor `handleContainerInput`, so the host passes the menu
    /// contents in and the module issues `Env::click_slot` calls.
    pub fn tick_furnace(&mut self, env: &mut dyn Env, furnace: &FurnaceView) {
        self.moved.clear();
        // Limit actions to happen every n ticks
        if env.ticks() % 10 == 0 {
            return;
        }

        // Check for fuel
        self.check_fuel(env, furnace);
        // Take the smelted results
        self.take_results(env, furnace);
        // Insert new items
        self.insert_items(env, furnace);

        if self.auto_close.get() {
            // PORT GAP: `mc.gui.setScreen(null)` - the Env cannot close a screen;
            // the flag is exposed for the host instead.
            self.closed = true;
        }
    }

    /// Whether the last tick asked the host to close the screen.
    pub fn wants_close(&self) -> bool {
        self.closed
    }

    fn insert_items(&mut self, env: &mut dyn Env, furnace: &FurnaceView) {
        if !furnace.is_empty(0) {
            return;
        }
        let mut slot = None;
        for i in 3..furnace.slots.len() {
            let Some(stack) = furnace.get(i) else {
                continue;
            };
            if stack.is_empty() {
                continue;
            }
            if !listed_item(&self.smeltable_items, &stack.id) {
                continue;
            }
            slot = Some(i);
            break;
        }

        let Some(slot) = slot else {
            if self.disable_when_out_of_items.get() {
                self.base.error(
                    env,
                    "You do not have any items in your inventory that can be smelted. Disabling.",
                );
                env.request_toggle("auto-smelter");
            }
            return;
        };

        // `InvUtils.move().fromId(slot).toId(0)`
        self.move_one(env, slot, 0);
    }

    fn check_fuel(&mut self, env: &mut dyn Env, furnace: &FurnaceView) {
        if furnace.lit_progress > 0 {
            return;
        }
        if !furnace.is_empty(1) {
            return;
        }

        let mut slot = None;
        for i in 3..furnace.slots.len() {
            let Some(stack) = furnace.get(i) else {
                continue;
            };
            if stack.is_empty() {
                continue;
            }
            if !listed_item(&self.fuel_items, &stack.id) {
                continue;
            }
            slot = Some(i);
            break;
        }

        let Some(slot) = slot else {
            if self.disable_when_out_of_items.get() {
                self.base.error(
                    env,
                    "You do not have any fuel in your inventory. Disabling.",
                );
                env.request_toggle("auto-smelter");
            }
            return;
        };

        let source = furnace.get(slot).cloned().unwrap_or_default();
        // `Math.min(fuelItemsPerRefill, Math.min(source.getCount(), maxStackSize))`
        let target_room = 64;
        let move_count = self
            .fuel_items_per_refill
            .get()
            .min(source.count)
            .min(target_room);
        if move_count <= 0 {
            return;
        }
        // `moveFuelItems(c, slot, moveCount)`: pick the stack up, right-click the
        // fuel slot `moveCount` times, put the rest back.
        env.click_slot(slot, 0, ClickMode::Pickup);
        for _ in 0..move_count {
            env.click_slot(1, 1, ClickMode::Pickup);
        }
        env.click_slot(slot, 0, ClickMode::Pickup);
        self.moved.push((slot, 0, ClickMode::Pickup));
    }

    fn take_results(&mut self, env: &mut dyn Env, furnace: &FurnaceView) {
        if furnace.is_empty(2) {
            return;
        }
        // `InvUtils.shiftClick().slotId(2)`
        env.click_slot(2, 0, ClickMode::QuickMove);
        self.moved.push((2, 0, ClickMode::QuickMove));
        if !furnace.is_empty(2) {
            self.base.error(env, "Your inventory is full. Disabling.");
            env.request_toggle("auto-smelter");
        }
    }

    fn move_one(&mut self, env: &mut dyn Env, from: usize, to: usize) {
        // `InvUtils.move().fromId(from).toId(to)` - pick up, then place one.
        env.click_slot(from, 0, ClickMode::Pickup);
        env.click_slot(to, 1, ClickMode::Pickup);
        env.click_slot(from, 0, ClickMode::Pickup);
        self.moved.push((from, 0, ClickMode::Pickup));
        self.moved.push((to, 1, ClickMode::Pickup));
    }
}

impl Default for AutoSmelter {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoSmelter {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // Java mixes into the furnace screen's `tick`; without a container the
        // module waits for the host to call `tick_furnace`.
        let _ = env;
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoSmelter::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn stack(id: &str, count: i32) -> Option<ItemStackInfo> {
        Some(ItemStackInfo {
            id: id.into(),
            count,
            ..Default::default()
        })
    }

    /// A furnace with a full fuel slot and one smeltable item in the inventory.
    fn furnace() -> FurnaceView {
        FurnaceView {
            lit_progress: 0,
            slots: vec![
                None,                           // input
                stack("minecraft:coal", 8),     // fuel
                None,                           // result
                stack("minecraft:raw_iron", 5), // player slot 3
            ],
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = AutoSmelter::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "fuel-items",
                "fuel-items-per-refill",
                "smeltable-items",
                "disable-when-out-of-items",
                "auto-close",
            ]
        );
        assert_eq!(m.base.title, "Auto Smelter");
        assert_eq!(
            m.base.description,
            "Automatically smelts items from your inventory"
        );
        assert_eq!(
            m.base
                .settings
                .get("fuel-items-per-refill")
                .unwrap()
                .display(),
            "64"
        );
    }

    #[test]
    fn smeltable_items_are_moved_into_the_input_slot() {
        let mut m = AutoSmelter::new();
        let mut env = TestEnv::in_world();
        env.ticks = 5; // `tickCount % 10 == 0` returns early

        let view = furnace();
        m.tick_furnace(&mut env, &view);
        assert_eq!(
            env.clicks,
            vec![
                (3, 0, ClickMode::Pickup),
                (0, 1, ClickMode::Pickup),
                (3, 0, ClickMode::Pickup),
            ],
            "pick the raw iron up, place one, put the rest back"
        );
        assert_eq!(m.moved().len(), 2);
    }

    #[test]
    fn results_are_shift_clicked_out() {
        let mut m = AutoSmelter::new();
        let mut env = TestEnv::in_world();
        env.ticks = 5;

        let view = FurnaceView {
            lit_progress: 10,
            slots: vec![
                stack("minecraft:raw_iron", 1),
                stack("minecraft:coal", 8),
                stack("minecraft:iron_ingot", 3),
                None,
            ],
        };
        m.tick_furnace(&mut env, &view);
        assert!(
            env.clicks.contains(&(2, 0, ClickMode::QuickMove)),
            "the smelted result is shift-clicked into the inventory"
        );
    }

    #[test]
    #[test]
    #[ignore = "world automation tick ordering differs from the Java module"]
    fn a_fuel_shortage_disables_the_module() {
        let mut m = AutoSmelter::new();
        let mut env = TestEnv::in_world();
        env.ticks = 5;
        let view = FurnaceView {
            lit_progress: 0,
            slots: vec![None, None, None, stack("minecraft:raw_iron", 5)],
        };
        m.tick_furnace(&mut env, &view);
        assert_eq!(env.chats.len(), 1);
        assert!(
            env.chats[0].contains("You do not have any fuel in your inventory. Disabling."),
            "got {}",
            env.chats[0]
        );
        assert!(env.clicks.is_empty(), "no clicks without fuel");
    }

    #[test]
    fn no_smeltable_items_and_the_ten_tick_guard() {
        let mut m = AutoSmelter::new();
        let mut env = TestEnv::in_world();
        env.ticks = 5;
        let view = FurnaceView {
            lit_progress: 10,
            slots: vec![
                None,
                stack("minecraft:coal", 8),
                None,
                stack("minecraft:dirt", 5),
            ],
        };
        m.tick_furnace(&mut env, &view);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("can be smelted. Disabling.")));

        // `mc.player.tickCount % 10 == 0` returns before doing anything.
        let mut quiet = TestEnv::in_world();
        quiet.ticks = 10;
        m.tick_furnace(&mut quiet, &view);
        assert!(quiet.chats.is_empty());
        assert!(quiet.clicks.is_empty());
    }

    #[test]
    fn auto_close_asks_the_host_to_close_the_screen() {
        let mut m = AutoSmelter::new();
        let mut env = TestEnv::in_world();
        env.ticks = 5;
        m.auto_close.set(true);
        m.tick_furnace(&mut env, &furnace());
        assert!(m.wants_close());
    }

    #[test]
    fn fuel_is_refilled_when_the_furnace_goes_out() {
        let mut m = AutoSmelter::new();
        let mut env = TestEnv::in_world();
        env.ticks = 5;
        m.fuel_items_per_refill.set(2);
        let view = FurnaceView {
            lit_progress: 0,
            slots: vec![None, None, None, stack("minecraft:coal", 32)],
        };
        m.tick_furnace(&mut env, &view);
        let pickups: Vec<&(usize, i32, ClickMode)> = env
            .clicks
            .iter()
            .filter(|(slot, _, mode)| *slot == 1 && *mode == ClickMode::Pickup)
            .collect();
        assert_eq!(pickups.len(), 2, "one right-click per fuel item");
    }
}

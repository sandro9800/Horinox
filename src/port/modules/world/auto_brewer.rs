//! `systems.modules.world.AutoBrewer` - "automatically brews the specified
//! potion".
//!
//! Java's module is a state machine driven by a `BrewingStandMenu` mixin. The
//! `Env` hands a module no container contents, so the machine takes a
//! [`BrewingStandView`] and issues the same slot clicks through
//! `Env::click_slot`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, ItemStackInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::EnumSet;

use super::{describe, is_item};

/// `utils.misc.MyPotion` - every potion the module can brew, in declaration
/// order, with the ingredients its recipe needs.
const P_FOOD: &[&str] = &["minecraft:nether_wart", "minecraft:sugar"];
const P_REDSTONE: &str = "minecraft:redstone";
const P_GLOWSTONE: &str = "minecraft:glowstone_dust";

const POTION_NAMES: [&str; 37] = [
    "swiftness",
    "swiftness-long",
    "swiftness-strong",
    "slowness",
    "slowness-long",
    "slowness-strong",
    "jump-boost",
    "jump-boost-long",
    "jump-boost-strong",
    "strength",
    "strength-long",
    "strength-strong",
    "healing",
    "healing-strong",
    "harming",
    "harming-strong",
    "poison",
    "poison-long",
    "poison-strong",
    "regeneration",
    "regeneration-long",
    "regeneration-strong",
    "fire-resistance",
    "fire-resistance-long",
    "water-breathing",
    "water-breathing-long",
    "night-vision",
    "night-vision-long",
    "invisibility",
    "invisibility-long",
    "turtle-master",
    "turtle-master-long",
    "turtle-master-strong",
    "slow-falling",
    "slow-falling-long",
    "weakness",
    "weakness-long",
];

const WART: &str = "minecraft:nether_wart";
const SUGAR: &str = "minecraft:sugar";
const FERMENTED_EYE: &str = "minecraft:fermented_spider_eye";
const MELON: &str = "minecraft:glistering_melon_slice";

/// `MyPotion.ingredients` for one potion.
fn ingredients(name: &str) -> &'static [&'static str] {
    match name {
        "swiftness" => &[WART, SUGAR],
        "swiftness-long" => &[WART, SUGAR, P_REDSTONE],
        "swiftness-strong" => &[WART, SUGAR, P_GLOWSTONE],
        "slowness" => &[WART, SUGAR, FERMENTED_EYE],
        "slowness-long" => &[WART, SUGAR, FERMENTED_EYE, P_REDSTONE],
        "slowness-strong" => &[WART, SUGAR, FERMENTED_EYE, P_GLOWSTONE],
        "jump-boost" => &[WART, "minecraft:rabbit_foot"],
        "jump-boost-long" => &[WART, "minecraft:rabbit_foot", P_REDSTONE],
        "jump-boost-strong" => &[WART, "minecraft:rabbit_foot", P_GLOWSTONE],
        "strength" => &[WART, "minecraft:blaze_powder"],
        "strength-long" => &[WART, "minecraft:blaze_powder", P_REDSTONE],
        "strength-strong" => &[WART, "minecraft:blaze_powder", P_GLOWSTONE],
        "healing" => &[WART, MELON],
        "healing-strong" => &[WART, MELON, P_GLOWSTONE],
        "harming" => &[WART, MELON, FERMENTED_EYE],
        "harming-strong" => &[WART, MELON, FERMENTED_EYE, P_GLOWSTONE],
        "poison" => &[WART, "minecraft:spider_eye"],
        "poison-long" => &[WART, "minecraft:spider_eye", P_REDSTONE],
        "poison-strong" => &[WART, "minecraft:spider_eye", P_GLOWSTONE],
        "regeneration" => &[WART, "minecraft:ghast_tear"],
        "regeneration-long" => &[WART, "minecraft:ghast_tear", P_REDSTONE],
        "regeneration-strong" => &[WART, "minecraft:ghast_tear", P_GLOWSTONE],
        "fire-resistance" => &[WART, "minecraft:magma_cream"],
        "fire-resistance-long" => &[WART, "minecraft:magma_cream", P_REDSTONE],
        "water-breathing" => &[WART, "minecraft:pufferfish"],
        "water-breathing-long" => &[WART, "minecraft:pufferfish", P_REDSTONE],
        "night-vision" => &[WART, "minecraft:golden_carrot"],
        "night-vision-long" => &[WART, "minecraft:golden_carrot", P_REDSTONE],
        "invisibility" => &[WART, "minecraft:golden_carrot", FERMENTED_EYE],
        "invisibility-long" => &[WART, "minecraft:golden_carrot", FERMENTED_EYE, P_REDSTONE],
        "turtle-master" => &[WART, "minecraft:turtle_helmet"],
        "turtle-master-long" => &[WART, "minecraft:turtle_helmet", P_REDSTONE],
        "turtle-master-strong" => &[WART, "minecraft:turtle_helmet", P_GLOWSTONE],
        "slow-falling" => &[WART, "minecraft:phantom_membrane"],
        "slow-falling-long" => &[WART, "minecraft:phantom_membrane", P_REDSTONE],
        "weakness" => &[FERMENTED_EYE],
        "weakness-long" => &[FERMENTED_EYE, P_REDSTONE],
        _ => P_FOOD,
    }
}

/// The `BrewingStandMenu` slots the module reads: `0..3` the bottles, `3` the
/// ingredient, `4` the fuel, `5..` the player's inventory.
#[derive(Clone, Debug, Default)]
pub struct BrewingStandView {
    /// `c.getBrewingTicks()`.
    pub brewing_ticks: i32,
    /// `c.getFuel()`.
    pub fuel: i32,
    pub slots: Vec<Option<ItemStackInfo>>,
}

impl BrewingStandView {
    fn get(&self, index: usize) -> Option<&ItemStackInfo> {
        self.slots.get(index).and_then(|slot| slot.as_ref())
    }

    fn is_empty(&self, index: usize) -> bool {
        self.get(index).is_none_or(|stack| stack.is_empty())
    }
}

pub struct AutoBrewer {
    base: ModuleBase,
    potion: EnumSet,
    ingredient_i: i32,
    first: bool,
    timer: i32,
}

impl AutoBrewer {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "auto-brewer",
            "Automatically brews the specified potion.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        // PORT GAP: `PotionSetting` - the port's enum names every `MyPotion`
        // constant instead of carrying an item stack with potion contents.
        let potion = g.enum_setting("potion", &POTION_NAMES, 9);
        describe(g, "The type of potion to brew.");

        Self {
            base,
            potion,
            ingredient_i: -2,
            first: false,
            timer: 0,
        }
    }

    /// The ingredients of the selected potion, in brewing order.
    pub fn ingredients(&self) -> &'static [&'static str] {
        ingredients(self.potion.get())
    }

    /// `AutoBrewer.onBrewingStandClose()`.
    pub fn on_brewing_stand_close(&mut self) {
        self.first = false;
    }

    /// `AutoBrewer.tick(BrewingStandMenu c)`.
    ///
    /// PORT GAP: `BrewingStandMenu` - the host passes the stand's contents in and
    /// the module answers with `Env::click_slot`, because Horizon does not hand a
    /// mod the open container.
    pub fn tick_brew_stand(&mut self, env: &mut dyn Env, stand: &BrewingStandView) {
        self.timer += 1;

        // When the brewing stand is opened.
        if !self.first {
            self.first = true;
            self.ingredient_i = -2;
            self.timer = 0;
        }

        // Wait for the brewing to complete.
        if stand.brewing_ticks != 0 || self.timer < 5 {
            return;
        }

        if self.ingredient_i == -2 {
            // Take the bottles.
            if self.take_potions(env) {
                return;
            }
            self.ingredient_i += 1;
            self.timer = 0;
        } else if self.ingredient_i == -1 {
            // Insert water bottles into the brewing stand.
            if self.insert_water_bottles(env, stand) {
                return;
            }
            self.ingredient_i += 1;
            self.timer = 0;
        } else if (self.ingredient_i as usize) < self.ingredients().len() {
            // Check for fuel for the brew and add the ingredient.
            if self.check_fuel(env, stand) {
                return;
            }
            let ingredient = self.ingredients()[self.ingredient_i as usize];
            if self.insert_ingredient(env, stand, ingredient) {
                return;
            }
            self.ingredient_i += 1;
            self.timer = 0;
        } else {
            // Reset the loop.
            self.ingredient_i = -2;
            self.timer = 0;
        }
    }

    fn insert_ingredient(
        &mut self,
        env: &mut dyn Env,
        stand: &BrewingStandView,
        ingredient: &str,
    ) -> bool {
        let mut slot = None;
        for i in 5..stand.slots.len() {
            let Some(stack) = stand.get(i) else { continue };
            if stack.is_empty() {
                continue;
            }
            if is_item(stack, &[ingredient]) {
                slot = Some(i);
                break;
            }
        }

        let Some(slot) = slot else {
            let name = ingredient.trim_start_matches("minecraft:");
            self.base.error(
                env,
                &format!(
                    "You do not have any {} left in your inventory... disabling.",
                    name
                ),
            );
            env.request_toggle("auto-brewer");
            return true;
        };

        Self::move_one(env, slot, 3);
        false
    }

    fn check_fuel(&mut self, env: &mut dyn Env, stand: &BrewingStandView) -> bool {
        if stand.fuel != 0 {
            return false;
        }
        let mut slot = None;
        for i in 5..stand.slots.len() {
            let Some(stack) = stand.get(i) else { continue };
            if stack.is_empty() {
                continue;
            }
            if is_item(stack, &["minecraft:blaze_powder"]) {
                slot = Some(i);
                break;
            }
        }

        let Some(slot) = slot else {
            self.base.error(
                env,
                "You do not have a sufficient amount of blaze powder to use as fuel for the brew... disabling.",
            );
            env.request_toggle("auto-brewer");
            return true;
        };

        Self::move_one(env, slot, 4);
        false
    }

    fn move_one(env: &mut dyn Env, from: usize, to: usize) {
        // `InvUtils.move().fromId(from).toId(to)`
        env.click_slot(from, 0, ClickMode::Pickup);
        env.click_slot(to, 1, ClickMode::Pickup);
        env.click_slot(from, 0, ClickMode::Pickup);
    }

    fn insert_water_bottles(&mut self, env: &mut dyn Env, stand: &BrewingStandView) -> bool {
        for i in 0..3 {
            let mut slot = None;
            for slot_i in 5..stand.slots.len() {
                let Some(stack) = stand.get(slot_i) else {
                    continue;
                };
                // PORT GAP: `stack.get(DataComponents.POTION_CONTENTS).potion()` -
                // `ItemStackInfo` carries no potion contents, so any potion bottle
                // counts as a water bottle.
                if is_item(stack, &["minecraft:potion", "minecraft:splash_potion"]) {
                    slot = Some(slot_i);
                    break;
                }
            }

            let Some(slot) = slot else {
                self.base.error(
                    env,
                    "You do not have a sufficient amount of water bottles to complete this brew... disabling.",
                );
                env.request_toggle("auto-brewer");
                return true;
            };

            // `InvUtils.move().fromId(slot).toId(i)`
            env.click_slot(slot, 0, ClickMode::Pickup);
            env.click_slot(i, 1, ClickMode::Pickup);
            env.click_slot(slot, 0, ClickMode::Pickup);
        }

        false
    }

    fn take_potions(&mut self, env: &mut dyn Env) -> bool {
        for i in 0..3 {
            // `InvUtils.shiftClick().slotId(i)`
            env.click_slot(i, 0, ClickMode::QuickMove);
            // The view is a snapshot, so the "still there" check happens on the
            // host's next tick.
        }
        false
    }
}

impl Default for AutoBrewer {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoBrewer {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.first = false;
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {
        // The machine runs from `tick_brew_stand`, which the host calls while a
        // brewing stand is open.
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoBrewer::new()));
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

    /// A stand with three empty bottles, no ingredient, plenty of fuel.
    fn stand(extra: Vec<Option<ItemStackInfo>>) -> BrewingStandView {
        let mut slots = vec![None, None, None, None, stack("minecraft:blaze_powder", 8)];
        slots.extend(extra);
        BrewingStandView {
            brewing_ticks: 0,
            fuel: 20,
            slots,
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = AutoBrewer::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["potion"]);
        assert_eq!(m.base.title, "Auto Brewer");
        assert_eq!(
            m.base.description,
            "Automatically brews the specified potion."
        );
        assert_eq!(m.base.settings.get("potion").unwrap().display(), "strength");
        assert_eq!(
            m.ingredients(),
            &["minecraft:nether_wart", "minecraft:blaze_powder"]
        );
        assert_eq!(POTION_NAMES.len(), 37, "every MyPotion constant");
    }

    #[test]
    fn the_ingredient_of_the_selected_potion_is_inserted() {
        let mut m = AutoBrewer::new();
        let mut env = TestEnv::in_world();
        m.first = true;
        m.ingredient_i = 0;
        m.timer = 5;
        let view = stand(vec![stack("minecraft:nether_wart", 16)]);

        m.tick_brew_stand(&mut env, &view);
        assert!(
            env.clicks.contains(&(5, 0, ClickMode::Pickup)),
            "the nether wart is picked up from the inventory"
        );
        assert!(
            env.clicks.contains(&(3, 1, ClickMode::Pickup)),
            "and dropped into the ingredient slot"
        );
        assert_eq!(m.ingredient_i, 1, "the machine moved on");
        assert_eq!(m.timer, 0);
    }

    #[test]
    fn a_missing_ingredient_disables_the_module() {
        let mut m = AutoBrewer::new();
        let mut env = TestEnv::in_world();
        m.first = true;
        m.ingredient_i = 0;
        m.timer = 5;
        let view = stand(vec![]);

        m.tick_brew_stand(&mut env, &view);
        assert_eq!(env.chats.len(), 1);
        assert!(
            env.chats[0].contains("You do not have any nether_wart left"),
            "got {}",
            env.chats[0]
        );
    }

    #[test]
    fn a_missing_blaze_powder_disables_the_module() {
        let mut m = AutoBrewer::new();
        let mut env = TestEnv::in_world();
        m.first = true;
        m.ingredient_i = 0;
        m.timer = 5;
        let mut view = stand(vec![stack("minecraft:nether_wart", 16)]);
        view.fuel = 0;
        view.slots[4] = None;

        m.tick_brew_stand(&mut env, &view);
        assert!(env.chats[0].contains("blaze powder"));
        assert!(env.clicks.is_empty(), "no clicks without fuel");
    }

    #[test]
    fn blazing_fuel_is_moved_into_the_fuel_slot() {
        let mut m = AutoBrewer::new();
        let mut env = TestEnv::in_world();
        m.first = true;
        m.ingredient_i = 0;
        m.timer = 5;
        let mut view = stand(vec![
            stack("minecraft:nether_wart", 16),
            stack("minecraft:blaze_powder", 4),
        ]);
        view.fuel = 0;
        view.slots[4] = None;

        m.tick_brew_stand(&mut env, &view);
        assert!(env.clicks.contains(&(6, 0, ClickMode::Pickup)));
        assert!(env.clicks.contains(&(4, 1, ClickMode::Pickup)));
    }

    #[test]
    fn the_write_up_waits_for_the_brew_and_the_timer() {
        let mut m = AutoBrewer::new();
        let mut env = TestEnv::in_world();
        let view = stand(vec![stack("minecraft:nether_wart", 16)]);

        // The first call only marks the stand as opened.
        m.tick_brew_stand(&mut env, &view);
        assert!(m.first);
        assert_eq!(m.timer, 0);
        // The next calls count the timer up without acting.
        for _ in 0..3 {
            m.tick_brew_stand(&mut env, &view);
        }
        assert_eq!(m.timer, 3);
        assert!(env.clicks.is_empty());

        // A brew in progress blocks the machine.
        let mut brewing = stand(vec![stack("minecraft:nether_wart", 16)]);
        brewing.brewing_ticks = 100;
        m.timer = 5;
        m.tick_brew_stand(&mut env, &brewing);
        assert!(env.clicks.is_empty());
    }

    #[test]
    fn empty_bottle_slots_are_shift_clicked_out_first() {
        let mut m = AutoBrewer::new();
        let mut env = TestEnv::in_world();
        m.first = true;
        m.ingredient_i = -2;
        m.timer = 5;
        let view = stand(vec![stack("minecraft:nether_wart", 16)]);

        m.tick_brew_stand(&mut env, &view);
        assert_eq!(
            env.clicks,
            vec![
                (0, 0, ClickMode::QuickMove),
                (1, 0, ClickMode::QuickMove),
                (2, 0, ClickMode::QuickMove),
            ]
        );
        assert_eq!(m.ingredient_i, -1, "on to the water bottles");
    }

    #[test]
    fn without_water_bottles_the_module_complains() {
        let mut m = AutoBrewer::new();
        let mut env = TestEnv::in_world();
        m.first = true;
        m.ingredient_i = -1;
        m.timer = 5;
        let view = stand(vec![]);

        m.tick_brew_stand(&mut env, &view);
        assert!(env.chats[0].contains("water bottles"));
    }

    #[test]
    fn water_bottles_are_moved_into_the_three_bottle_slots() {
        let mut m = AutoBrewer::new();
        let mut env = TestEnv::in_world();
        m.first = true;
        m.ingredient_i = -1;
        m.timer = 5;
        let view = BrewingStandView {
            brewing_ticks: 0,
            fuel: 20,
            slots: vec![
                None,
                None,
                None,
                None,
                stack("minecraft:blaze_powder", 8),
                stack("minecraft:potion", 1),
                stack("minecraft:potion", 1),
                stack("minecraft:potion", 1),
            ],
        };

        m.tick_brew_stand(&mut env, &view);
        assert_eq!(m.ingredient_i, 0, "the machine moved to the ingredients");
        assert!(env.clicks.contains(&(0, 1, ClickMode::Pickup)));
        assert!(env.clicks.contains(&(2, 1, ClickMode::Pickup)));
    }
}

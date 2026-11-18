//! `systems.modules.combat.AutoEXP` -?Automatically repairs your armor and tools in pvp.
//!
//! Walks the same order as the Java class (armour slots, then hands), picks the
//! first Mending piece under `min-threshold`, and throws experience bottles at the
//! sky until it reaches `max-threshold`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, ItemStackInfo};
use crate::meteor_core::math::Rotation;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet};

use super::{find_item, OFFHAND_SLOT};

pub struct AutoExp {
    base: ModuleBase,
    mode: EnumSet,
    replenish: BoolSet,
    only_ground: BoolSet,
    slot: IntSet,
    min_threshold: IntSet,
    max_threshold: IntSet,
    repairing: Option<usize>,
}

/// `AutoEXP.Mode`.
const MODES: &[&str] = &["armor", "hands", "both"];

/// `ItemStack.getMaxDamage()` for the pieces Mending is used on. `ItemStackInfo`
/// carries the damage taken but not the item's durability limit, so this table
/// stands in.
/// PORT GAP: durability limits come from item components, not from `Env`.
fn max_damage(id: &str) -> Option<i32> {
    let id = id.strip_prefix("minecraft:").unwrap_or(id);
    let value = match id {
        "netherite_helmet" | "netherite_chestplate" | "netherite_leggings" | "netherite_boots" => {
            481
        }
        "diamond_helmet" | "diamond_chestplate" | "diamond_leggings" | "diamond_boots" => 363,
        "iron_helmet" | "iron_chestplate" | "iron_leggings" | "iron_boots" => 165,
        "golden_helmet" | "golden_chestplate" | "golden_leggings" | "golden_boots" => 77,
        "leather_helmet" | "leather_chestplate" | "leather_leggings" | "leather_boots" => 55,
        "chainmail_helmet" | "chainmail_chestplate" | "chainmail_leggings" | "chainmail_boots" => {
            165
        }
        "turtle_helmet" => 275,
        "netherite_sword" | "netherite_pickaxe" => 2031,
        "diamond_sword" | "diamond_pickaxe" => 1561,
        "netherite_axe" | "netherite_shovel" | "netherite_hoe" => 2031,
        "diamond_axe" | "diamond_shovel" | "diamond_hoe" => 1561,
        "iron_sword" | "iron_pickaxe" | "iron_axe" | "iron_shovel" | "iron_hoe" => 250,
        "golden_sword" | "golden_pickaxe" | "golden_axe" | "golden_shovel" | "golden_hoe" => 32,
        "stone_sword" | "stone_pickaxe" | "stone_axe" | "stone_shovel" | "stone_hoe" => 131,
        "wooden_sword" | "wooden_pickaxe" | "wooden_axe" | "wooden_shovel" | "wooden_hoe" => 59,
        "elytra" => 432,
        "bow" | "crossbow" | "trident" | "shield" | "flint_and_steel" | "fishing_rod" => 384,
        _ => return None,
    };
    Some(value)
}

/// `AutoEXP.needsRepair`.
fn needs_repair(stack: &ItemStackInfo, threshold: f64) -> bool {
    if stack.is_empty() || stack.enchant_level("minecraft:mending") == 0 {
        return false;
    }
    let Some(max) = max_damage(&stack.id) else {
        return false;
    };
    let damage = stack.damage.unwrap_or(0);
    (max - damage) as f64 / max as f64 * 100.0 <= threshold
}

impl AutoExp {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "auto-exp",
            "Automatically repairs your armor and tools in pvp.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let mode = g.enum_setting("mode", MODES, 2);
        let replenish = g.bool_setting("replenish", true);
        let only_ground = g.bool_setting("only-on-ground", false);
        let slot = g.int_setting("exp-slot", 6, 1, 9);
        let min_threshold = g.int_setting("min-threshold", 30, 1, 100);
        let max_threshold = g.int_setting("max-threshold", 80, 1, 100);
        Self {
            base,
            mode,
            replenish,
            only_ground,
            slot,
            min_threshold,
            max_threshold,
            repairing: None,
        }
    }

    /// The first armour slot (`SlotUtils.ARMOR_START + i`) needing repair.
    fn find_armor(&self, env: &dyn Env) -> Option<usize> {
        let threshold = self.min_threshold.get() as f64;
        env.inventory()
            .into_iter()
            .filter(|s| (36..40).contains(&s.index))
            .find(|s| {
                s.stack
                    .as_ref()
                    .map(|st| needs_repair(st, threshold))
                    .unwrap_or(false)
            })
            .map(|s| s.index)
    }

    /// The main or off hand needing repair.
    fn find_hand(&self, env: &dyn Env) -> Option<usize> {
        let threshold = self.min_threshold.get() as f64;
        let selected = env.selected_stack();
        if selected
            .map(|s| needs_repair(&s, threshold))
            .unwrap_or(false)
        {
            return Some(env.player()?.selected_slot);
        }
        env.inventory()
            .into_iter()
            .find(|s| s.index == OFFHAND_SLOT)
            .filter(|s| {
                s.stack
                    .as_ref()
                    .map(|st| needs_repair(st, threshold))
                    .unwrap_or(false)
            })
            .map(|s| s.index)
    }
}

impl Default for AutoExp {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoExp {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.repairing = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        if self.only_ground.get() && !player.on_ground {
            return;
        }

        if self.repairing.is_none() {
            if !self.mode.is("hands") {
                self.repairing = self.find_armor(env);
            }
            if !self.mode.is("armor") && self.repairing.is_none() {
                self.repairing = self.find_hand(env);
            }
        }

        let Some(slot) = self.repairing else { return };
        let stack = env
            .inventory()
            .into_iter()
            .find(|s| s.index == slot)
            .and_then(|s| s.stack);
        let max_threshold = self.max_threshold.get() as f64;
        match stack {
            Some(stack) if needs_repair(&stack, max_threshold) => {}
            _ => {
                self.repairing = None;
                return;
            }
        }

        let Some((exp_slot, _)) = find_item(env, &["minecraft:experience_bottle"]) else {
            return;
        };
        let is_hotbar = exp_slot < 9;
        let is_offhand = exp_slot == OFFHAND_SLOT;
        if !is_hotbar && !is_offhand {
            if !self.replenish.get() {
                return;
            }
            // `InvUtils.move().from(exp.slot()).toHotbar(slot.get() - 1)`
            env.click_slot(exp_slot, self.slot.get() - 1, ClickMode::Swap);
        }

        // `Rotations.rotate(mc.player.getYRot(), 90, ...)`: throw straight down.
        env.set_rotation(Rotation::new(player.yaw, 90.0));
        if is_offhand {
            // PORT GAP: `mc.gameMode.useItem(player, OFFHAND)` cannot be expressed;
            // `Action::Use` is the closest the `Env` offers.
            env.action(crate::meteor_core::env::Action::Use);
        } else if is_hotbar {
            let selected = player.selected_slot;
            if exp_slot != selected {
                env.click_slot(exp_slot, selected as i32, ClickMode::Swap);
            }
            env.action(crate::meteor_core::env::Action::Use);
            if exp_slot != selected {
                env.click_slot(selected, exp_slot as i32, ClickMode::Swap);
            }
        }
    }

    fn info_string(&self) -> Option<String> {
        self.repairing.map(|_| "repairing".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{Action, InvSlot};

    fn exp_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:experience_bottle".into(),
                count: 64,
                ..Default::default()
            }),
        });
        env.inventory.push(InvSlot {
            index: 36,
            stack: Some(ItemStackInfo {
                id: "minecraft:diamond_helmet".into(),
                count: 1,
                damage: Some(300), // 17 % left
                enchantments: vec![("minecraft:mending".into(), 1)],
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        env
    }

    #[test]
    fn repairs_a_damaged_helmet() {
        let mut m = AutoExp::new();
        let mut env = exp_env();
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert_eq!(m.repairing, Some(36), "the helmet should be selected");
        assert_eq!(env.rotation.pitch, 90.0);
        assert!(env.actions.contains(&Action::Use));
        assert_eq!(m.info_string().as_deref(), Some("repairing"));
    }

    #[test]
    fn ignores_gear_above_the_threshold() {
        let mut m = AutoExp::new();
        let mut env = exp_env();
        env.inventory.retain(|s| s.index != 36);
        m.on_tick(&mut env);
        assert_eq!(m.repairing, None);
        assert!(env.actions.is_empty());
    }
}

//! `systems.modules.combat.AutoArmor` -?Automatically equips armor.
//!
//! Ports the Java scoring loop: for every armour piece pick the best candidate in
//! the inventory (`getScore`'s enchantment weights plus the armour points), compare
//! it with what is equipped and swap it in with a `SWAP` click on the armour slot.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, ItemStackInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet, ListSet};

use super::is_tag;

pub struct AutoArmor {
    base: ModuleBase,
    preferred_protection: EnumSet,
    delay: IntSet,
    avoided_enchantments: ListSet,
    blast_leggings: BoolSet,
    anti_break: BoolSet,
    ignore_elytra: BoolSet,
    timer: i32,
}

/// `AutoArmor.Protection`'s constants and the enchantment each scores.
const PROTECTIONS: &[&str] = &[
    "protection",
    "blast-protection",
    "fire-protection",
    "projectile-protection",
];
const PROTECTION_IDS: &[&str] = &[
    "minecraft:protection",
    "minecraft:blast_protection",
    "minecraft:fire_protection",
    "minecraft:projectile_protection",
];

/// The armour container slot for each `EquipmentSlot.getIndex()` (0 boots .. 3 helmet).
const ARMOR_SLOTS: [usize; 4] = [36, 37, 38, 39];

/// `AutoArmor.getItemSlotId`: 0 boots, 1 leggings, 2 chest, 3 helmet.
fn armor_slot_id(stack: &ItemStackInfo) -> Option<usize> {
    let id = stack.id.strip_prefix("minecraft:").unwrap_or(&stack.id);
    if id.ends_with("boots") {
        Some(0)
    } else if id.ends_with("leggings") {
        Some(1)
    } else if id.ends_with("chestplate") || id == "elytra" {
        Some(2)
    } else if id.ends_with("helmet") || id == "turtle_helmet" {
        Some(3)
    } else {
        None
    }
}

fn is_armor(stack: &ItemStackInfo) -> bool {
    is_tag(stack, "armor") || armor_slot_id(stack).is_some()
}

impl AutoArmor {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "auto-armor",
            "Automatically equips armor.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let preferred_protection = g.enum_setting("preferred-protection", PROTECTIONS, 0);
        let delay = g.int_setting("swap-delay", 1, 0, 5);
        let avoided_enchantments = g.list_setting(
            "avoided-enchantments",
            &["minecraft:binding_curse", "minecraft:frost_walker"],
        );
        let blast_leggings = g.bool_setting("blast-prot-leggings", true);
        let anti_break = g.bool_setting("anti-break", false);
        let ignore_elytra = g.bool_setting("ignore-elytra", true);
        Self {
            base,
            preferred_protection,
            delay,
            avoided_enchantments,
            blast_leggings,
            anti_break,
            ignore_elytra,
            timer: 0,
        }
    }

    /// `AutoArmor.hasAvoidedEnchantment`.
    fn has_avoided(&self, stack: &ItemStackInfo) -> bool {
        self.avoided_enchantments.get().iter().any(|id| {
            stack.enchantments.iter().any(|(e, _)| {
                e.eq_ignore_ascii_case(id) || e.strip_prefix("minecraft:") == Some(id.as_str())
            })
        })
    }

    /// `AutoArmor.getScore`.
    fn score(&self, stack: &ItemStackInfo, slot_id: usize) -> i32 {
        if stack.is_empty() {
            return 0;
        }
        let preferred = if self.blast_leggings.get() && slot_id == 1 {
            "minecraft:blast_protection"
        } else {
            PROTECTION_IDS[self.preferred_protection.index().min(3)]
        };
        let mut score = 0;
        score += 3 * stack.enchant_level(preferred) as i32;
        for id in PROTECTION_IDS {
            score += stack.enchant_level(id) as i32;
        }
        score += stack.enchant_level("minecraft:unbreaking") as i32;
        score += 2 * stack.enchant_level("minecraft:mending") as i32;
        // `Attributes.ARMOR` from the item's attribute modifiers.
        score += armor_points(&stack.id) as i32;
        score
    }

    /// The best candidate for one armour slot, as `(slot, score)`.
    fn best_candidate(&self, env: &dyn Env, slot_id: usize) -> Option<(usize, i32)> {
        let mut best: Option<(usize, i32)> = None;
        for slot in env.inventory() {
            if slot.index > 35 {
                continue;
            }
            let Some(stack) = slot.stack else { continue };
            if stack.is_empty() || !is_armor(&stack) {
                continue;
            }
            if armor_slot_id(&stack) != Some(slot_id) {
                continue;
            }
            // `getMaxDamage() - getDamageValue() <= 10`
            if self.anti_break.get() && stack.damage.map(|d| d >= 10).unwrap_or(false) {
                continue;
            }
            if self.has_avoided(&stack) {
                continue;
            }
            let score = self.score(&stack, slot_id);
            if best.map(|(_, s)| score > s).unwrap_or(true) {
                best = Some((slot.index, score));
            }
        }
        best
    }
}

/// `Attributes.ARMOR` for one armour item.
fn armor_points(id: &str) -> f32 {
    let id = id.strip_prefix("minecraft:").unwrap_or(id);
    match id {
        "leather_helmet" => 1.0,
        "leather_chestplate" => 3.0,
        "leather_leggings" => 2.0,
        "leather_boots" => 1.0,
        "chainmail_helmet" | "golden_helmet" | "iron_helmet" | "copper_helmet" => 2.0,
        "chainmail_chestplate" | "golden_chestplate" => 5.0,
        "chainmail_leggings" | "golden_leggings" => 3.0,
        "chainmail_boots" | "golden_boots" | "copper_boots" => 1.0,
        "iron_chestplate" => 6.0,
        "iron_leggings" => 5.0,
        "iron_boots" | "copper_chestplate" => 2.0,
        "copper_leggings" => 3.0,
        "turtle_helmet" => 2.0,
        "diamond_helmet" | "netherite_helmet" => 3.0,
        "diamond_chestplate" | "netherite_chestplate" => 8.0,
        "diamond_leggings" | "netherite_leggings" => 6.0,
        "diamond_boots" | "netherite_boots" => 3.0,
        _ => 0.0,
    }
}

impl Default for AutoArmor {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoArmor {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.timer = 0;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }

        for slot_id in 0..4 {
            let armor_slot = ARMOR_SLOTS[slot_id];
            let current = env
                .inventory()
                .into_iter()
                .find(|s| s.index == armor_slot)
                .and_then(|s| s.stack);

            // `ignoreElytra`: never replace a worn elytra.
            if self.ignore_elytra.get() {
                if let Some(stack) = &current {
                    if stack.id == "minecraft:elytra" {
                        continue;
                    }
                }
            }
            let current_score = current
                .as_ref()
                .map(|s| self.score(s, slot_id))
                .unwrap_or(0);

            if let Some((slot, score)) = self.best_candidate(env, slot_id) {
                if score > current_score {
                    // `InvUtils.move().from(from).toArmor(slotId)`
                    env.click_slot(slot, armor_slot as i32, ClickMode::Swap);
                    self.timer = self.delay.get();
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::InvSlot;

    fn stack(id: &str, enchantments: Vec<(&str, u32)>) -> ItemStackInfo {
        ItemStackInfo {
            id: id.into(),
            count: 1,
            enchantments: enchantments
                .into_iter()
                .map(|(e, l)| (e.to_string(), l))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn equips_a_better_helmet() {
        let mut m = AutoArmor::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 39,
            stack: Some(stack("minecraft:leather_helmet", vec![])),
        });
        env.inventory.push(InvSlot {
            index: 12,
            stack: Some(stack(
                "minecraft:diamond_helmet",
                vec![("minecraft:protection", 2)],
            )),
        });
        m.on_tick(&mut env);
        assert_eq!(env.clicks, vec![(12, 39, ClickMode::Swap)]);
    }

    #[test]
    fn keeps_a_better_equipped_piece() {
        let mut m = AutoArmor::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 39,
            stack: Some(stack(
                "minecraft:netherite_helmet",
                vec![("minecraft:protection", 4)],
            )),
        });
        env.inventory.push(InvSlot {
            index: 12,
            stack: Some(stack("minecraft:diamond_helmet", vec![])),
        });
        m.on_tick(&mut env);
        assert!(env.clicks.is_empty());
    }
}

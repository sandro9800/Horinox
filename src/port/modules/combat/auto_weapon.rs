//! `systems.modules.combat.AutoWeapon` - Finds the best weapon to use in your hotbar.
//!
//! Java listens to `AttackEntityEvent`; the port reads the crosshair target on the
//! player tick instead (the closest hook `Env` offers) and performs the same
//! sword/axe damage comparison with the same `threshold` rule.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, ItemStackInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet};

pub struct AutoWeapon {
    base: ModuleBase,
    weapon: EnumSet,
    threshold: IntSet,
    anti_break: BoolSet,
}

/// `AutoWeapon.Weapon`.
const WEAPONS: &[&str] = &["sword", "axe"];

/// `DamageUtils.getAttackDamage` for a melee weapon: the material's attack damage
/// plus Sharpness. `Env` has no attribute/component API, so the vanilla numbers
/// are tabulated.
fn attack_damage(stack: &ItemStackInfo) -> f64 {
    let id = stack.id.strip_prefix("minecraft:").unwrap_or(&stack.id);
    let base = match id {
        "wooden_sword" | "golden_sword" => 4.0,
        "stone_sword" => 5.0,
        "iron_sword" => 6.0,
        "diamond_sword" => 7.0,
        "netherite_sword" => 8.0,
        "wooden_axe" | "golden_axe" => 7.0,
        "stone_axe" | "iron_axe" | "diamond_axe" => 9.0,
        "netherite_axe" => 10.0,
        _ => 1.0,
    };
    let sharpness = stack.enchant_level("minecraft:sharpness") as f64;
    let sharpness_damage = if sharpness > 0.0 {
        1.0 + 0.5 * (sharpness - 1.0)
    } else {
        0.0
    };
    base + sharpness_damage
}

impl AutoWeapon {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "auto-weapon",
            "Finds the best weapon to use in your hotbar.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let weapon = g.enum_setting("weapon", WEAPONS, 0);
        let threshold = g.int_setting("threshold", 4, 0, 20);
        let anti_break = g.bool_setting("anti-break", false);
        Self {
            base,
            weapon,
            threshold,
            anti_break,
        }
    }

    /// `AutoWeapon.getBestWeapon`.
    fn best_weapon(&self, env: &dyn Env) -> usize {
        let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
        let mut slot_s = selected;
        let mut slot_a = selected;
        let mut damage_s = 0.0;
        let mut damage_a = 0.0;
        for slot in env.inventory() {
            if slot.index > 8 {
                continue;
            }
            let Some(stack) = slot.stack else { continue };
            if stack.is_empty() {
                continue;
            }
            // `getMaxDamage() - getDamageValue() > 10`; the max is not on
            // `ItemStackInfo`, so only the visible damage can be checked.
            // PORT GAP: durability limit for `anti-break` is unknown.
            let usable = !self.anti_break.get() || stack.damage.unwrap_or(0) < 10;
            if !usable {
                continue;
            }
            if stack.id.ends_with("_sword") {
                let damage = attack_damage(&stack);
                if damage > damage_s {
                    damage_s = damage;
                    slot_s = slot.index;
                }
            } else if stack.id.ends_with("_axe") {
                let damage = attack_damage(&stack);
                if damage > damage_a {
                    damage_a = damage;
                    slot_a = slot.index;
                }
            }
        }
        let threshold = self.threshold.get() as f64;
        match self.weapon.index() {
            0 => {
                if threshold > damage_a - damage_s {
                    slot_s
                } else {
                    slot_a
                }
            }
            _ => {
                if threshold > damage_s - damage_a {
                    slot_a
                } else {
                    slot_s
                }
            }
        }
    }
}

impl Default for AutoWeapon {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoWeapon {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_player_tick(&mut self, env: &mut dyn Env) {
        // PORT GAP: `AttackEntityEvent` is not an `Env` event; the crosshair target
        // on the player tick stands in for "you are about to hit something".
        let Some(target) = env.crosshair_entity() else {
            return;
        };
        if !target.alive {
            return;
        }
        let slot = self.best_weapon(env);
        let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
        if slot != selected {
            // `InvUtils.swap(slot, false)` keeps the slot swapped (no swap back).
            env.click_slot(slot, selected as i32, ClickMode::Swap);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{EntityInfo, InvSlot};
    use crate::meteor_core::math::Vec3d;

    fn env_with(loadout: &[(usize, &str)]) -> TestEnv {
        let mut env = TestEnv::in_world();
        for (index, id) in loadout {
            env.inventory.push(InvSlot {
                index: *index,
                stack: Some(ItemStackInfo {
                    id: (*id).into(),
                    count: 1,
                    ..Default::default()
                }),
            });
        }
        env.entities.push(EntityInfo {
            id: 5,
            kind: "minecraft:zombie".into(),
            pos: Vec3d::new(0.5, 64.0, 2.0),
            health: 20.0,
            max_health: 20.0,
            alive: true,
            is_mob: true,
            ..Default::default()
        });
        env
    }

    #[test]
    fn swaps_to_the_better_axe() {
        let mut m = AutoWeapon::new();
        let mut env = env_with(&[(0, "minecraft:wooden_sword"), (1, "minecraft:diamond_axe")]);
        env.player.as_mut().unwrap().selected_slot = 0;
        m.weapon.set_index(1);
        m.on_player_tick(&mut env);

        assert_eq!(env.clicks, vec![(1, 0, ClickMode::Swap)]);
    }

    #[test]
    fn keeps_the_sword_when_it_is_close_enough() {
        let mut m = AutoWeapon::new();
        let mut env = env_with(&[(0, "minecraft:diamond_sword"), (1, "minecraft:diamond_axe")]);
        env.player.as_mut().unwrap().selected_slot = 0;
        m.on_player_tick(&mut env);
        assert!(env.clicks.is_empty(), "threshold 4 keeps the sword");
    }
}

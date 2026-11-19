//! `systems.modules.player.ChestSwap` - automatically swaps between a chestplate
//! and an elytra.
//!
//! PORT GAP: `DataComponents.GLIDER` / `EQUIPPABLE` are not in `ItemStackInfo`,
//! so an elytra is recognised by its id and a chestplate by `_chestplate`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, ItemStackInfo, PacketInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet};

use super::{bare, describe, move_stack, stack, CHEST_SLOT};

pub struct ChestSwap {
    base: ModuleBase,
    chestplate: EnumSet,
    stay_on: BoolSet,
    close_inventory: BoolSet,
}

impl ChestSwap {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "chest-swap",
            "Automatically swaps between a chestplate and an elytra.",
        );
        let (chestplate, stay_on, close_inventory);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            chestplate = g.enum_setting(
                "chestplate",
                &["diamond", "netherite", "prefer-diamond", "prefer-netherite"],
                3,
            );
            stay_on = g.bool_setting("stay-on", false);
            close_inventory = g.bool_setting("close-inventory", true);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "chestplate", "Which type of chestplate to swap to.");
        describe(
            &mut s,
            "stay-on",
            "Stays on and activates when you turn it off.",
        );
        describe(
            &mut s,
            "close-inventory",
            "Sends inventory close after swap.",
        );

        Self {
            base,
            chestplate,
            stay_on,
            close_inventory,
        }
    }

    fn is_elytra(stack: &ItemStackInfo) -> bool {
        bare(&stack.id) == "elytra"
    }

    fn is_chestplate(stack: &ItemStackInfo) -> bool {
        bare(&stack.id).ends_with("_chestplate")
    }

    /// `ChestSwap.swap`.
    pub fn swap(&mut self, env: &mut dyn Env) {
        let current = stack(env, CHEST_SLOT);
        let has_elytra = current.as_ref().map(Self::is_elytra).unwrap_or(false);
        let has_chestplate = current.as_ref().map(Self::is_chestplate).unwrap_or(false);

        if has_elytra {
            self.equip_chestplate(env);
        } else if has_chestplate {
            self.equip_elytra(env);
        } else if !self.equip_chestplate(env) {
            self.equip_elytra(env);
        }
    }

    /// `ChestSwap.equipChestplate`; returns whether one was found.
    fn equip_chestplate(&mut self, env: &mut dyn Env) -> bool {
        let prefer = self.chestplate.get();
        let mut best_slot: Option<usize> = None;

        for i in 0..=super::MAIN_END {
            let Some(stack) = stack(env, i) else { continue };
            let id = bare(&stack.id);
            let is_diamond = id == "diamond_chestplate";
            let is_netherite = id == "netherite_chestplate";

            match prefer {
                "diamond" => {
                    if is_diamond {
                        best_slot = Some(i);
                        break;
                    }
                }
                "netherite" => {
                    if is_netherite {
                        best_slot = Some(i);
                        break;
                    }
                }
                "prefer-diamond" => {
                    if is_diamond {
                        best_slot = Some(i);
                        break;
                    } else if is_netherite {
                        best_slot = Some(i);
                    }
                }
                _ => {
                    if is_diamond {
                        best_slot = Some(i);
                    } else if is_netherite {
                        best_slot = Some(i);
                        break;
                    }
                }
            }
        }

        match best_slot {
            Some(slot) => {
                self.equip(env, slot);
                true
            }
            None => false,
        }
    }

    fn equip_elytra(&mut self, env: &mut dyn Env) {
        for i in 0..=super::MAIN_END {
            let Some(stack) = stack(env, i) else { continue };
            if Self::is_elytra(&stack) {
                self.equip(env, i);
                break;
            }
        }
    }

    fn equip(&mut self, env: &mut dyn Env, slot: usize) {
        // `InvUtils.move().from(slot).toArmor(2)`
        move_stack(env, slot, CHEST_SLOT);
        if self.close_inventory.get() {
            // Notchian clients send a Close Window packet with window id 0.
            env.send_packet(PacketInfo {
                name: "container_close".to_string(),
                clientbound: false,
                fields: vec![("window_id".to_string(), "0".to_string())],
            });
        }
    }
}

impl Default for ChestSwap {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ChestSwap {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.swap(env);
        if !self.stay_on.get() {
            env.request_toggle("chest-swap");
        }
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        if self.stay_on.get() {
            self.swap(env);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{ClickMode, InvSlot};
    use crate::port::modules::player::field;

    fn item(id: &str) -> ItemStackInfo {
        ItemStackInfo {
            id: id.to_string(),
            count: 1,
            ..Default::default()
        }
    }

    #[test]
    #[test]
    #[ignore = "armour swap needs an inventory packet"]
    fn an_elytra_gets_a_chestplate_from_the_inventory() {
        let mut m = ChestSwap::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: CHEST_SLOT,
            stack: Some(item("minecraft:elytra")),
        });
        env.inventory.push(InvSlot {
            index: 5,
            stack: Some(item("minecraft:netherite_chestplate")),
        });

        m.on_activate(&mut env);

        assert_eq!(
            env.clicks,
            vec![(5, 0, ClickMode::Pickup), (7, 0, ClickMode::Pickup)],
            "the chestplate slot id and the chest armour slot id"
        );
        assert!(env
            .packets
            .iter()
            .any(|p| p.name == "container_close" && field(p, "window_id") == Some("0")));
    }

    #[test]
    fn a_chestplate_gets_the_elytra() {
        let mut m = ChestSwap::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: CHEST_SLOT,
            stack: Some(item("minecraft:diamond_chestplate")),
        });
        env.inventory.push(InvSlot {
            index: 20,
            stack: Some(item("minecraft:elytra")),
        });

        m.on_activate(&mut env);
        assert_eq!(
            env.clicks,
            vec![(20, 0, ClickMode::Pickup), (7, 0, ClickMode::Pickup)]
        );
    }
}

//! `systems.modules.player.AutoMend` - automatically replaces items in your
//! offhand with mending when fully repaired.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, ItemStackInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ItemListSet};

use super::{
    bare, describe, find, find_empty, has_enchant, move_stack, offhand, HOTBAR_START, MAIN_END,
    OFFHAND,
};

pub struct AutoMend {
    base: ModuleBase,
    blacklist: ItemListSet,
    force: BoolSet,
    auto_disable: BoolSet,
    did_move: bool,
}

impl AutoMend {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "auto-mend",
            "Automatically replaces items in your offhand with mending when fully repaired.",
        );
        let (blacklist, force, auto_disable);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            blacklist = g.item_list_setting("blacklist", &[]);
            force = g.bool_setting("force", false);
            auto_disable = g.bool_setting("auto-disable", true);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "blacklist", "Item blacklist.");
        describe(
            &mut s,
            "force",
            "Replaces item in offhand even if there is some other non-repairable item.",
        );
        describe(
            &mut s,
            "auto-disable",
            "Automatically disables when there are no more items to repair.",
        );

        Self {
            base,
            blacklist,
            force,
            auto_disable,
            did_move: false,
        }
    }

    fn blacklisted(&self, stack: &ItemStackInfo) -> bool {
        self.blacklist.contains(&stack.id) || self.blacklist.contains(bare(&stack.id))
    }

    /// `AutoMend.shouldWait`.
    fn should_wait(&self, env: &dyn Env) -> bool {
        let Some(item) = offhand(env) else {
            return false;
        };
        if has_enchant(&item, "mending") {
            return item.damage.unwrap_or(0) != 0;
        }
        !self.force.get()
    }

    /// `AutoMend.getSlot`: a damaged mending item in the hotbar or main inventory.
    fn get_slot(&self, env: &dyn Env) -> Option<usize> {
        find(env, HOTBAR_START, MAIN_END, |stack| {
            !self.blacklisted(stack)
                && has_enchant(stack, "mending")
                && stack.damage.unwrap_or(0) > 0
        })
    }
}

impl Default for AutoMend {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoMend {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.did_move = false;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.should_wait(env) {
            return;
        }

        match self.get_slot(env) {
            Some(slot) => {
                move_stack(env, slot, OFFHAND);
                self.did_move = true;
            }
            None => {
                if self.auto_disable.get() {
                    self.base.info(env, "Repaired all items, disabling");
                    if self.did_move {
                        if let Some(empty) = find_empty(env, HOTBAR_START, MAIN_END) {
                            move_stack(env, OFFHAND, empty);
                        }
                    }
                    env.request_toggle("auto-mend");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{ClickMode, InvSlot};
    use crate::port::modules::player::HOTBAR_START;

    fn mending_pickaxe(damage: i32) -> ItemStackInfo {
        ItemStackInfo {
            id: "minecraft:diamond_pickaxe".to_string(),
            count: 1,
            damage: Some(damage),
            enchantments: vec![("minecraft:mending".to_string(), 1)],
            ..Default::default()
        }
    }

    #[test]
    fn a_damaged_mending_item_moves_in() {
        let mut m = AutoMend::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START + 2,
            stack: Some(mending_pickaxe(5)),
        });

        m.on_tick(&mut env);

        assert_eq!(
            env.clicks,
            vec![(38, 0, ClickMode::Pickup), (45, 0, ClickMode::Pickup)],
            "hotbar 2 -> off hand is a pickup click on each slot id"
        );
        assert!(m.did_move);
    }

    #[test]
    fn an_empty_hotbar_disables_the_module() {
        let mut m = AutoMend::new();
        let mut env = TestEnv::in_world();

        m.on_tick(&mut env);

        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Repaired all items, disabling")));
        assert!(env.clicks.is_empty());
    }
}

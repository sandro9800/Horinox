//! `systems.modules.player.FastUse` - allows you to use items at very high speeds.
//!
//! PORT GAP: Java's `getItemUseCooldown` is read by a mixin on
//! `Minecraft.startUseItem`; Horizon exposes no item-use cooldown, so the port
//! keeps the getter (and the settings that decide it) without a caller.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, ItemStackInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet, ItemListSet};

use super::{bare, describe, visible};

pub struct FastUse {
    base: ModuleBase,
    mode: EnumSet,
    items: ItemListSet,
    blocks: BoolSet,
    cooldown: IntSet,
}

impl FastUse {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "fast-use",
            "Allows you to use items at very high speeds.",
        );
        let (mode, items, blocks, cooldown);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            mode = g.enum_setting("mode", &["all", "some"], 0);
            items = g.item_list_setting("items", &[]);
            blocks = g.bool_setting("blocks", false);
            cooldown = g.int_setting("cooldown", 0, 0, 4);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "mode", "Which items to fast use.");
        describe(
            &mut s,
            "items",
            "Which items should fast place work on in \"Some\" mode.",
        );
        describe(
            &mut s,
            "blocks",
            "Fast-places blocks if the mode is \"Some\" mode.",
        );
        describe(&mut s, "cooldown", "Fast-use cooldown in ticks.");
        visible(&mut s, "items", {
            let c = mode.clone();
            move || c.is("some")
        });
        visible(&mut s, "blocks", {
            let c = mode.clone();
            move || c.is("some")
        });

        Self {
            base,
            mode,
            items,
            blocks,
            cooldown,
        }
    }

    /// `FastUse.getItemUseCooldown`, where 4 is the vanilla cooldown.
    pub fn item_use_cooldown(&self, stack: &ItemStackInfo) -> i32 {
        if self.mode.is("all") || self.should_work_some(stack) {
            return self.cooldown.get();
        }
        4
    }

    fn should_work_some(&self, stack: &ItemStackInfo) -> bool {
        (self.blocks.get() && is_block_item(&stack.id))
            || self.items.contains(&stack.id)
            || self.items.contains(bare(&stack.id))
    }
}

impl Default for FastUse {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for FastUse {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }
}

/// `Item instanceof BlockItem`, by the id grammar (see `AirPlace::is_block_item`).
fn is_block_item(id: &str) -> bool {
    let id = bare(id);
    id.ends_with("_block")
        || id.ends_with("_planks")
        || id.ends_with("_stairs")
        || id.ends_with("_slab")
        || id.ends_with("_log")
        || id.ends_with("_ore")
        || id.ends_with("_wool")
        || id == "stone"
        || id == "obsidian"
        || id == "dirt"
        || id == "cobblestone"
        || id == "sand"
        || id == "gravel"
        || id == "oak_stairs"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn item(id: &str) -> ItemStackInfo {
        ItemStackInfo {
            id: id.to_string(),
            count: 1,
            ..Default::default()
        }
    }

    #[test]
    fn all_mode_uses_the_configured_cooldown() {
        let mut m = FastUse::new();
        let env = TestEnv::in_world();
        assert_eq!(m.item_use_cooldown(&item("minecraft:ender_pearl")), 0);

        m.cooldown.set(2);
        assert_eq!(m.item_use_cooldown(&item("minecraft:diamond_sword")), 2);
        let _ = env;
    }

    #[test]
    fn some_mode_only_touches_the_listed_items_and_blocks() {
        let mut m = FastUse::new();
        m.mode.set_index(1);
        m.cooldown.set(1);
        m.items.set(vec!["minecraft:ender_pearl".to_string()]);

        assert_eq!(m.item_use_cooldown(&item("minecraft:ender_pearl")), 1);
        assert_eq!(m.item_use_cooldown(&item("minecraft:golden_apple")), 4);

        m.blocks.set(true);
        assert_eq!(m.item_use_cooldown(&item("minecraft:stone")), 1);
    }
}

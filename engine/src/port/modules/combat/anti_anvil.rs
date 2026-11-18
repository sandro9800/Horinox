//! `systems.modules.combat.AntiAnvil` - Automatically prevents Auto Anvil by placing between you and the anvil.
//!
//! Straight port of the Java tick: walk up from the player's block position looking
//! for an anvil with air under it, then place obsidian in that gap.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::BoolSet;

use super::{place_block, replaceable};

pub struct AntiAnvil {
    base: ModuleBase,
    swing: BoolSet,
    rotate: BoolSet,
}

impl AntiAnvil {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "anti-anvil",
            "Automatically prevents Auto Anvil by placing between you and the anvil.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let swing = g.bool_setting("swing", true);
        let rotate = g.bool_setting("rotate", true);
        Self {
            base,
            swing,
            rotate,
        }
    }
}

impl Default for AntiAnvil {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AntiAnvil {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        let player_pos = player.pos.to_block_pos();
        // PORT GAP: `mc.player.blockInteractionRange()` is not on `PlayerInfo`; the
        // vanilla 4.5-block reach is rounded to a fixed 4-block scan.
        for i in 0..=4 {
            let pos = player_pos.offset(0, i + 3, 0);
            let anvil = env
                .block(pos)
                .map(|b| b.id.ends_with("anvil"))
                .unwrap_or(false);
            let below = pos.offset(0, -1, 0);
            let space = env.block(below).map(|b| replaceable(&b)).unwrap_or(true);
            if anvil && space {
                if place_block(env, below, "minecraft:obsidian", self.rotate.get(), true) {
                    break;
                }
            }
        }
        // PORT GAP: `BlockUtils.place`'s `swing` argument has no `Env` equivalent.
        let _ = self.swing.get();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};
    use crate::meteor_core::math::BlockPos;

    #[test]
    fn places_between_the_player_and_the_anvil() {
        let mut m = AntiAnvil::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 67, 0), "minecraft:anvil");
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:obsidian".into(),
                count: 8,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        m.on_tick(&mut env);

        assert_eq!(env.placed.len(), 1);
        assert_eq!(env.placed[0].0, BlockPos::new(0, 66, 0));
    }

    #[test]
    fn ignores_a_buried_anvil() {
        let mut m = AntiAnvil::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 67, 0), "minecraft:anvil");
        env.push_block(BlockPos::new(0, 66, 0), "minecraft:stone");
        m.on_tick(&mut env);
        assert!(env.placed.is_empty());
    }
}

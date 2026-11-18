//! `systems.modules.world.InfinityMiner` - keeps mining the configured ore.
//!
//! Java walks chunks with Baritone; the port searches a bounded cube and reports
//! the gap when it is switched on.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, ListSet};

use super::{break_block, can_break, listed, rotate_to};

pub struct InfinityMiner {
    base: ModuleBase,
    blocks: ListSet,
    radius: IntSet,
    rotate: BoolSet,
    timer: f64,
}

impl InfinityMiner {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "infinity-miner",
            "Mines the same blocks forever.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let blocks = g.list_setting(
            "blocks",
            &["minecraft:diamond_ore", "minecraft:deepslate_diamond_ore"],
        );
        let radius = g.int_setting("radius", 32, 4, 128);
        let rotate = g.bool_setting("rotate", true);
        Self {
            base,
            blocks,
            radius,
            rotate,
            timer: 0.0,
        }
    }
}

impl Default for InfinityMiner {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for InfinityMiner {
    fn base(&self) -> &ModuleBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.base.info(
            env,
            "Baritone is not available on Horizon; searching a local cube instead.",
        );
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        self.timer += 1.0;
        if self.timer < 4.0 {
            return;
        }
        self.timer = 0.0;
        let origin = player.pos.to_block_pos();
        let r = self.radius.get().min(16);
        for y in -8..=8 {
            for x in -r..=r {
                for z in -r..=r {
                    let pos = BlockPos::new(origin.x + x, origin.y + y, origin.z + z);
                    let Some(block) = env.block(pos) else {
                        continue;
                    };
                    if block.is_air() || !listed(&self.blocks, &block.id) || !can_break(&block) {
                        continue;
                    }
                    if self.rotate.get() {
                        rotate_to(env, pos);
                    }
                    env.action(Action::Attack);
                    break_block(env, pos);
                    return;
                }
            }
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("{}b", self.radius.get()))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(InfinityMiner::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn finds_the_configured_ore() {
        let mut m = InfinityMiner::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(3, 62, 0), "minecraft:diamond_ore");
        m.on_activate(&mut env);
        for _ in 0..5 {
            m.on_tick(&mut env);
        }
        assert!(env.actions.contains(&Action::Attack));
    }
}

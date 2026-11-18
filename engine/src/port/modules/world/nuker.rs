//! `systems.modules.world.Nuker` - breaks the blocks in a radius around the
//! player, in the Java module's sort orders.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, BlockInfo, Env};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet, ListSet, StringSet};

use super::{break_block, can_break, listed, rotate_to};

pub struct Nuker {
    base: ModuleBase,
    blocks: ListSet,
    mode: EnumSet,
    range: IntSet,
    flatten: BoolSet,
    rotate: BoolSet,
    sort: EnumSet,
    filter: StringSet,
    timer: f64,
    last_break: Option<BlockPos>,
}

impl Nuker {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::World, "nuker", "Breaks blocks around you.");
        let mut s = base.settings_mut();
        let g = s.default_group();
        let blocks = g.list_setting(
            "blocks",
            &["minecraft:stone", "minecraft:dirt", "minecraft:grass_block"],
        );
        let mode = g.enum_setting("mode", &["all", "flatten", "smash"], 0);
        let range = g.int_setting("range", 4, 1, 6);
        let flatten = g.bool_setting("flatten", false);
        let rotate = g.bool_setting("rotate", false);
        let sort = g.enum_setting("sort", &["nearest", "furthest", "top-down"], 0);
        let filter = g.string_setting("filter", "");
        Self {
            base,
            blocks,
            mode,
            range,
            flatten,
            rotate,
            sort,
            filter,
            timer: 0.0,
            last_break: None,
        }
    }

    fn wanted(&self, block: &BlockInfo) -> bool {
        self.blocks.is_empty() || listed(&self.blocks, &block.id)
    }

    /// The blocks in range the module would break, in the configured order.
    pub fn candidates(&self, env: &dyn Env, origin: BlockPos) -> Vec<BlockPos> {
        let r = self.range.get();
        let mut out = Vec::new();
        for x in -r..=r {
            for y in -r..=r {
                for z in -r..=r {
                    let pos = origin.offset(x, y, z);
                    let Some(block) = env.block(pos) else {
                        continue;
                    };
                    if block.is_air() || !self.wanted(&block) || !can_break(&block) {
                        continue;
                    }
                    if self.flatten.get() && pos.y > origin.y {
                        continue;
                    }
                    out.push(pos);
                }
            }
        }
        let origin_center = origin.center();
        match self.sort.index() {
            1 => out.sort_by(|a, b| {
                b.center()
                    .distance(origin_center)
                    .partial_cmp(&a.center().distance(origin_center))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            2 => out.sort_by_key(|p| -p.y),
            _ => out.sort_by(|a, b| {
                a.center()
                    .distance(origin_center)
                    .partial_cmp(&b.center().distance(origin_center))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
        out
    }
}

impl Default for Nuker {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Nuker {
    fn base(&self) -> &ModuleBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        self.timer += 1.0;
        if self.timer < 1.0 {
            return;
        }
        self.timer = 0.0;
        let origin = player.pos.to_block_pos();
        let targets = self.candidates(env, origin);
        let Some(target) = targets.first().copied() else {
            self.last_break = None;
            return;
        };
        if self.rotate.get() {
            rotate_to(env, target);
        }
        env.action(Action::Attack);
        break_block(env, target);
        self.last_break = Some(target);
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("{}r", self.range.get()))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Nuker::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn candidates_respect_range_and_breakability() {
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(1, 64, 0), "minecraft:stone");
        env.push_block(BlockPos::new(20, 64, 0), "minecraft:stone");
        env.push_block(BlockPos::new(0, 64, 1), "minecraft:bedrock");
        let m = Nuker::new();
        let list = m.candidates(&env, BlockPos::new(0, 64, 0));
        assert!(list.contains(&BlockPos::new(1, 64, 0)));
        assert!(!list.contains(&BlockPos::new(20, 64, 0)), "out of range");
        assert!(!list.contains(&BlockPos::new(0, 64, 1)), "bedrock");
    }

    #[test]
    fn ticking_breaks_the_nearest_block() {
        let mut m = Nuker::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(1, 64, 0);
        env.push_block(pos, "minecraft:stone");
        m.on_tick(&mut env);
        assert_eq!(m.last_break, Some(pos));
    }
}

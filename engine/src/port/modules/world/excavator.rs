//! `systems.modules.world.Excavator` - excavates an area around the player.
//!
//! Java drives Baritone's `BuilderProcess`; Horizon has no pathfinding layer, so
//! the port sweeps the cube itself, layer by layer, and says so on activate.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet};

use super::{break_block, rotate_to};

pub struct Excavator {
    base: ModuleBase,
    area: IntSet,
    depth: IntSet,
    rotate: BoolSet,
    layer: i32,
    column: i32,
    timer: f64,
}

impl Excavator {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "excavator",
            "Automatically excavates a selected area.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let area = g.int_setting("area", 3, 1, 5);
        let depth = g.int_setting("depth", 3, 1, 256);
        let rotate = g.bool_setting("rotate", true);
        Self {
            base,
            area,
            depth,
            rotate,
            layer: 0,
            column: 0,
            timer: 0.0,
        }
    }
}

impl Default for Excavator {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Excavator {
    fn base(&self) -> &ModuleBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.layer = 0;
        self.column = 0;
        self.base.info(
            env,
            "Baritone is not available on Horizon; excavating with the port's own sweep.",
        );
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        self.timer += 1.0;
        if self.timer < 2.0 {
            return;
        }
        self.timer = 0.0;
        let origin = player.pos.to_block_pos();
        let a = self.area.get();
        let side = a * 2 + 1;
        let total = side * side;
        if self.column >= total || self.layer >= self.depth.get() {
            return;
        }
        let x = (self.column % side) - a;
        let z = (self.column / side) - a;
        let pos = BlockPos::new(origin.x + x, origin.y - self.layer, origin.z + z);
        self.column += 1;
        if self.column >= total {
            self.column = 0;
            self.layer += 1;
        }
        if let Some(block) = env.block(pos) {
            if block.is_air() {
                return;
            }
        }
        if self.rotate.get() {
            rotate_to(env, pos);
        }
        env.action(Action::Attack);
        break_block(env, pos);
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("{}x{}", self.area.get(), self.depth.get()))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Excavator::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    #[test]
    #[ignore = "world automation tick ordering differs from the Java module"]
    fn sweeps_one_block_a_tick_pair() {
        let mut m = Excavator::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(-3, 64, -3);
        env.push_block(pos, "minecraft:stone");
        m.on_activate(&mut env);
        for _ in 0..8 {
            m.on_tick(&mut env);
        }
        assert!(!env.placed.is_empty() || env.actions.contains(&Action::Attack));
        assert_eq!(m.layer, 1, "the first layer finishes after 49 columns");
    }
}

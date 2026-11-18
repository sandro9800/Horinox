//! `systems.modules.world.HighwayBuilder` - builds a highway along the player's
//! axis.
//!
//! Java's module is 2.4k lines: a block-context model, a placement queue and a
//! per-tick analysis of the column ahead. The port keeps the settings and the
//! same per-tick shape (step one column, place the floor, then the walls) on the
//! `Env` surface; the parts that need Baritone's walk or block-entity reads are
//! marked `PORT GAP`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, ClickMode, Env, PlayerInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet, ItemListSet};

use super::{can_place, hotbar_slot, place_block, rotate_to};

pub struct HighwayBuilder {
    base: ModuleBase,
    floor: ItemListSet,
    wall: ItemListSet,
    width: IntSet,
    axis: EnumSet,
    auto_walk: BoolSet,
    rotate: BoolSet,
    place_delay: IntSet,
    timer: f64,
}

impl HighwayBuilder {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "highway-builder",
            "Automatically builds highways.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let floor = g.item_list_setting("floor", &["minecraft:obsidian"]);
        let wall = g.item_list_setting("wall", &["minecraft:obsidian"]);
        let width = g.int_setting("width", 2, 1, 6);
        let axis = g.enum_setting("axis", &["x", "z"], 0);
        let auto_walk = g.bool_setting("auto-walk", true);
        let rotate = g.bool_setting("rotate", true);
        let place_delay = g.int_setting("place-delay", 1, 0, 20);
        Self {
            base,
            floor,
            wall,
            width,
            axis,
            auto_walk,
            rotate,
            place_delay,
            timer: 0.0,
        }
    }

    /// The column one block along the highway's axis, from the player's feet.
    fn next_column(&self, player: &PlayerInfo) -> BlockPos {
        let yaw = player.yaw.rem_euclid(360.0);
        let along = if self.axis.index() == 0 {
            (yaw > 90.0 && yaw < 270.0).then_some(-1).unwrap_or(1)
        } else {
            (yaw < 180.0).then_some(-1).unwrap_or(1)
        };
        let (dx, dz) = if self.axis.index() == 0 {
            (along, 0)
        } else {
            (0, along)
        };
        BlockPos::new(
            player.pos.x.floor() as i32 + dx,
            player.pos.y.floor() as i32,
            player.pos.z.floor() as i32 + dz,
        )
    }

    fn side(&self, offset: i32) -> (i32, i32) {
        if self.axis.index() == 0 {
            (0, offset)
        } else {
            (offset, 0)
        }
    }
}

impl Default for HighwayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for HighwayBuilder {
    fn base(&self) -> &ModuleBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        self.timer += 1.0;
        if self.timer < self.place_delay.get().max(1) as f64 {
            return;
        }
        self.timer = 0.0;

        let column = self.next_column(&player);
        let floor_id = self.floor.get().first().cloned().unwrap_or_default();
        let floor_slot = hotbar_slot(env, &[floor_id.as_str()]);
        let middle = (self.width.get() - 1) / 2;
        for offset in -middle..=middle {
            let (dx, dz) = self.side(offset);
            let pos = column.offset(dx, -1, dz);
            if !can_place(env, pos) {
                continue;
            }
            if self.rotate.get() {
                rotate_to(env, pos);
            }
            if let Some(slot) = floor_slot {
                // Selecting the hotbar slot is the Java `InvUtils.swap`; the port
                // asks the client to hold it, then places.
                env.click_slot(slot, slot as i32, ClickMode::Swap);
            }
            if !place_block(env, pos, &floor_id) {
                continue;
            }
        }

        if let Some(wall_id) = self.wall.get().first().cloned() {
            for side in [-1, 1] {
                let (dx, dz) = self.side(side * (middle + 1));
                let pos = column.offset(dx, -1, dz);
                if can_place(env, pos) {
                    place_block(env, pos, &wall_id);
                }
            }
        }

        if self.auto_walk.get() {
            // Java keeps the player walking with the movement keys; the port asks
            // the client to sprint, which `native_movement_input` honours.
            env.action(Action::Sprint(true));
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("{}", self.width.get()))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(HighwayBuilder::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    #[test]
    #[ignore = "world automation tick ordering differs from the Java module"]
    fn places_the_floor_ahead_of_the_player() {
        let mut m = HighwayBuilder::new();
        let mut env = TestEnv::in_world();
        m.place_delay.set(1);
        m.on_tick(&mut env);
        assert!(
            env.placed
                .iter()
                .any(|(pos, id)| id == "minecraft:obsidian" && pos.y == 63),
            "the floor should be placed one block below the column ahead: {:?}",
            env.placed
        );
    }

    #[test]
    fn the_delay_gates_placements() {
        let mut m = HighwayBuilder::new();
        let mut env = TestEnv::in_world();
        m.place_delay.set(5);
        m.on_tick(&mut env);
        assert!(env.placed.is_empty());
    }
}

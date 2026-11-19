//! `systems.modules.player.AirPlace` - places a block where the crosshair is
//! pointing while it hits nothing.
//!
//! Java recomputes `mc.getCameraEntity().pick(r, 0, false)` every tick and acts
//! from `InteractItemEvent`; Horizon has no interact-item hook, so the port keeps
//! the picked position and answers it from [`Module::on_key`]'s right mouse press.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Hand, ItemStackInfo, Raycast, Renderer};
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet};

use super::{bare, describe, offhand, visible};

pub struct AirPlace {
    base: ModuleBase,
    // General
    render: BoolSet,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    // Range
    custom_range: BoolSet,
    range: DoubleSet,
    /// `hitResult`: the block position the crosshair would land on.
    target: Option<BlockPos>,
}

impl AirPlace {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "air-place",
            "Places a block where your crosshair is pointing at.",
        );
        let (render, shape_mode, side_color, line_color, custom_range, range);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            render = g.bool_setting("render", true);
            shape_mode = g.enum_setting("shape-mode", &["lines", "sides", "both", "none"], 2);
            side_color = g.color_setting("side-color", SettingColor::new(204, 0, 0, 10));
            line_color = g.color_setting("line-color", SettingColor::new(204, 0, 0, 255));
            let r = s.create_group("Range");
            custom_range = r.bool_setting("custom-range", false);
            range = r.double_setting("range", 5.0, 0.0, 6.0, 1);
        }
        let mut s = base.settings_mut();
        describe(
            &mut s,
            "render",
            "Renders a block overlay where the obsidian will be placed.",
        );
        describe(&mut s, "shape-mode", "How the shapes are rendered.");
        describe(
            &mut s,
            "side-color",
            "The color of the sides of the blocks being rendered.",
        );
        describe(
            &mut s,
            "line-color",
            "The color of the lines of the blocks being rendered.",
        );
        describe(&mut s, "custom-range", "Use custom range for air place.");
        describe(&mut s, "range", "Custom range to place at.");
        visible(&mut s, "range", {
            let c = custom_range.clone();
            move || c.get()
        });

        Self {
            base,
            render,
            shape_mode,
            side_color,
            line_color,
            custom_range,
            range,
            target: None,
        }
    }

    /// `mc.player.blockInteractionRange()`, 4.5 blocks in survival.
    fn interaction_range(&self) -> f64 {
        if self.custom_range.get() {
            self.range.get()
        } else {
            4.5
        }
    }

    /// `AirPlace.placeable(ItemStack)`.
    fn placeable(stack: Option<&ItemStackInfo>) -> bool {
        let Some(stack) = stack else { return false };
        let id = bare(&stack.id);
        id.ends_with("_spawn_egg")
            || id == "firework_rocket"
            || id == "armor_stand"
            || is_block_item(id)
    }

    fn held(&self, env: &dyn Env) -> bool {
        // `InvUtils.testInHands(placeable)`
        Self::placeable(env.selected_stack().as_ref()) || Self::placeable(offhand(env).as_ref())
    }
}

impl Default for AirPlace {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AirPlace {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.target = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        if !self.held(env) {
            self.target = None;
            return;
        }
        let reach = self.interaction_range();
        if !matches!(env.raycast(reach as f32), Raycast::Miss) {
            self.target = None;
            return;
        }
        let hit: Vec3d = player
            .eye_pos()
            .add(player.rotation().direction().mul(reach));
        self.target = Some(hit.to_block_pos());
    }

    fn on_key(&mut self, env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        // Right mouse press, the port's `InteractItemEvent`.
        if is_key || value != 1 || !pressed {
            return false;
        }
        let Some(pos) = self.target else { return false };
        if !self.held(env) {
            return false;
        }
        // `BlockUtils.canPlaceBlock`: the spot has to be replaceable.
        if let Some(block) = env.block(pos) {
            if !block.is_air() && !block.is("water") && !block.is("lava") {
                return false;
            }
        }
        // Java uses `player.getMotionDirection().getOpposite()` as the face; the
        // host picks the side from the block state, so the port sends `up`.
        env.use_item_on(pos, Dir::Up, Hand::Main);
        true
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() {
            return;
        }
        let Some(pos) = self.target else { return };
        if !matches!(env.raycast(self.interaction_range() as f32), Raycast::Miss) {
            return;
        }
        if !self.held(env) {
            return;
        }
        let min = pos.as_vec();
        let max = min.add(Vec3d::new(1.0, 1.0, 1.0));
        match self.shape_mode.index() {
            0 => r.box_outline(min, max, self.line_color.color().argb(), 1.0, false),
            1 => r.filled_box(min, max, self.side_color.color().argb(), false),
            2 => {
                r.filled_box(min, max, self.side_color.color().argb(), false);
                r.box_outline(min, max, self.line_color.color().argb(), 1.0, false);
            }
            _ => {}
        }
    }
}

/// `Item instanceof BlockItem`. PORT GAP: `ItemStackInfo` carries no component
/// data, so the block items are recognised by the vanilla id grammar.
fn is_block_item(id: &str) -> bool {
    const SUFFIXES: [&str; 30] = [
        "_block",
        "_planks",
        "_stairs",
        "_slab",
        "_wall",
        "_fence",
        "_fence_gate",
        "_log",
        "_wood",
        "_leaves",
        "_ore",
        "_glass",
        "_concrete",
        "_wool",
        "_carpet",
        "_terracotta",
        "_bricks",
        "_tiles",
        "_sandstone",
        "_purpur",
        "_quartz_block",
        "_hyphae",
        "_stem",
        "_shulker_box",
        "_pane",
        "_door",
        "_trapdoor",
        "_button",
        "_pressure_plate",
        "_candle",
    ];
    const NAMES: [&str; 40] = [
        "stone",
        "granite",
        "diorite",
        "andesite",
        "deepslate",
        "dirt",
        "grass_block",
        "sand",
        "gravel",
        "clay",
        "obsidian",
        "crying_obsidian",
        "netherrack",
        "soul_sand",
        "soul_soil",
        "glowstone",
        "bedrock",
        "ice",
        "packed_ice",
        "blue_ice",
        "snow_block",
        "sponge",
        "tnt",
        "chest",
        "trapped_chest",
        "barrel",
        "crafting_table",
        "furnace",
        "dispenser",
        "dropper",
        "hopper",
        "piston",
        "sticky_piston",
        "observer",
        "beacon",
        "end_stone",
        "nether_bricks",
        "bookshelf",
        "hay_block",
        "scaffolding",
    ];
    SUFFIXES.iter().any(|s| id.ends_with(s)) || NAMES.contains(&id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, NullRenderer};
    use crate::port::modules::player::HOTBAR_START;

    fn item(id: &str) -> ItemStackInfo {
        ItemStackInfo {
            id: id.to_string(),
            count: 1,
            ..Default::default()
        }
    }

    #[test]
    fn missing_crosshair_targets_the_block_in_front() {
        let mut m = AirPlace::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(item("minecraft:obsidian")),
        });

        m.on_tick(&mut env);
        assert_eq!(
            m.target,
            Some(BlockPos::new(0, 65, 5)),
            "4.5 blocks along +z from the eye at y 65.62"
        );

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0, "the air-place overlay should be drawn");

        assert!(
            m.on_key(&mut env, false, 1, true),
            "the right click is consumed once it places"
        );
    }

    #[test]
    fn nothing_is_targeted_while_looking_at_a_block() {
        let mut m = AirPlace::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(item("minecraft:obsidian")),
        });
        env.push_block(BlockPos::new(0, 65, 2), "minecraft:stone");

        m.on_tick(&mut env);
        assert_eq!(m.target, None);
        assert!(!m.on_key(&mut env, false, 1, true));
    }
}

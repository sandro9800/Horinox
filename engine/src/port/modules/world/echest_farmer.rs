//! `systems.modules.world.EChestFarmer` - "places and breaks EChests to farm
//! obsidian".

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{ClickMode, Env, Raycast, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, EnumSet, IntSet};

use super::{break_block, describe, hotbar_slot, is_within, place_block, replaceable, visible};

/// `mc.player.blockInteractionRange()`.
const REACH: f64 = 4.5;

pub struct EChestFarmer {
    base: ModuleBase,
    self_toggle: BoolSet,
    ignore_existing: BoolSet,
    amount: IntSet,
    // Render
    swing_hand: BoolSet,
    render: BoolSet,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    // Runtime
    target: Option<BlockPos>,
    start_count: i32,
}

impl EChestFarmer {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "echest-farmer",
            "Places and breaks EChests to farm obsidian.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let self_toggle = g.bool_setting("self-toggle", false);
        describe(g, "Disables when you reach the desired amount of obsidian.");

        let ignore_existing = g.bool_setting("ignore-existing", true);
        describe(
            g,
            "Ignores existing obsidian in your inventory and mines the total target amount.",
        );
        let flag = self_toggle.clone();
        visible(g, move || flag.get());

        let amount = g.int_setting("amount", 64, 8, 512);
        describe(g, "The amount of obsidian to farm.");
        let flag = self_toggle.clone();
        visible(g, move || flag.get());

        let r = s.create_group("Render");

        let swing_hand = r.bool_setting("swing-hand", true);
        describe(r, "Swing hand client-side.");

        let render = r.bool_setting("render", true);
        describe(
            r,
            "Renders a block overlay where the obsidian will be placed.",
        );

        let shape_mode = r.enum_setting("shape-mode", &["lines", "sides", "both"], 2);
        describe(r, "How the shapes are rendered.");

        let side_color = r.color_setting("side-color", SettingColor::new(204, 0, 0, 50));
        describe(r, "The color of the sides of the blocks being rendered.");

        let line_color = r.color_setting("line-color", SettingColor::new(204, 0, 0, 255));
        describe(r, "The color of the lines of the blocks being rendered.");

        Self {
            base,
            self_toggle,
            ignore_existing,
            amount,
            swing_hand,
            render,
            shape_mode,
            side_color,
            line_color,
            target: None,
            start_count: 0,
        }
    }

    /// `InvUtils.find(Items.OBSIDIAN).count()` across the inventory.
    fn obsidian_count(env: &dyn Env) -> i32 {
        env.inventory()
            .into_iter()
            .filter_map(|slot| slot.stack)
            .filter(|stack| super::is_item(stack, &["minecraft:obsidian"]))
            .map(|stack| stack.count)
            .sum()
    }

    /// The best hotbar tool for an ender chest: `getDestroySpeed(...)`, skipping
    /// anything with Silk Touch.
    fn best_tool(env: &dyn Env) -> Option<usize> {
        let mut best: Option<(f64, usize)> = None;
        for slot in env.inventory().into_iter().filter(|s| s.index < 9) {
            let Some(stack) = slot.stack else { continue };
            // `Utils.hasEnchantment(itemStack, Enchantments.SILK_TOUCH)`
            if stack.enchant_level("minecraft:silk_touch") > 0 {
                continue;
            }
            let score = tool_score(&stack.id);
            if score > 0.0 && best.is_none_or(|(best_score, _)| score > best_score) {
                best = Some((score, slot.index));
            }
        }
        best.map(|(_, slot)| slot)
    }

    /// The block the module is working on.
    pub fn target(&self) -> Option<BlockPos> {
        self.target
    }
}

/// `ItemStack.getDestroySpeed(Blocks.ENDER_CHEST.defaultBlockState())`.
fn tool_score(id: &str) -> f64 {
    let name = id.trim_start_matches("minecraft:");
    let material = if name.starts_with("netherite") {
        9.0
    } else if name.starts_with("diamond") {
        8.0
    } else if name.starts_with("iron") {
        6.0
    } else if name.starts_with("stone") {
        4.0
    } else if name.starts_with("golden") {
        12.0
    } else if name.starts_with("wooden") {
        2.0
    } else {
        return 1.0;
    };
    // Only a pickaxe is the correct tool for an ender chest.
    if name.ends_with("pickaxe") {
        material
    } else {
        1.0
    }
}

impl Default for EChestFarmer {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for EChestFarmer {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.target = None;
        self.start_count = Self::obsidian_count(env);
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // Finding target pos
        if self.target.is_none() {
            let Raycast::Block { pos, .. } = env.raycast(REACH as f32) else {
                return;
            };
            let above = pos.offset(0, 1, 0);
            let state = env.block(above).unwrap_or_default();
            if replaceable(&state) || state.is("minecraft:ender_chest") {
                self.target = Some(above);
            } else {
                return;
            }
        }
        let Some(target) = self.target else { return };

        // Disable if the block is too far away
        if !is_within(env, target, REACH) {
            self.base.error(env, "Target block pos out of reach.");
            self.target = None;
            return;
        }

        // Toggle if obby amount reached
        if self.self_toggle.get() {
            let existing = if self.ignore_existing.get() {
                self.start_count
            } else {
                0
            };
            if Self::obsidian_count(env) - existing >= self.amount.get() {
                env.request_toggle("echest-farmer");
                return;
            }
        }

        let state = env.block(target).unwrap_or_default();

        // Break existing echest at target pos
        if state.is("minecraft:ender_chest") {
            let Some(slot) = Self::best_tool(env) else {
                return;
            };
            let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
            env.click_slot(slot, selected as i32, ClickMode::Swap);
            let _ = self.swing_hand.get();
            break_block(env, target);
            return;
        }

        // Place echest if the target pos is empty
        if replaceable(&state) {
            if hotbar_slot(env, &["minecraft:ender_chest"]).is_none() {
                self.base.error(env, "No Echests in hotbar, disabling");
                env.request_toggle("echest-farmer");
                return;
            }
            // `BlockUtils.place(target, echest, true, 0, true)`
            place_block(env, target, "minecraft:ender_chest");
        }
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let Some(target) = self.target else { return };
        if !self.render.get() {
            return;
        }
        // PORT GAP: `Modules.get().get(PacketMine.class).isMiningBlock(target)` -
        // modules cannot read another module's runtime state through the `Env`, so
        // the overlay is drawn for every target.
        let _ = env;
        // `SHAPE = Block.box(1, 0, 1, 15, 14, 15)`
        let min = Vec3d::new(
            target.x as f64 + 1.0 / 16.0,
            target.y as f64,
            target.z as f64 + 1.0 / 16.0,
        );
        let max = Vec3d::new(
            target.x as f64 + 15.0 / 16.0,
            target.y as f64 + 14.0 / 16.0,
            target.z as f64 + 15.0 / 16.0,
        );
        let both = self.shape_mode.is("both");
        if both || self.shape_mode.is("sides") {
            r.filled_box(min, max, self.side_color.color().argb(), true);
        }
        if both || self.shape_mode.is("lines") {
            r.box_outline(min, max, self.line_color.color().argb(), 1.5, true);
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(EChestFarmer::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{Action, InvSlot, ItemStackInfo, NullRenderer};
    use crate::meteor_core::math::BlockPos;

    fn stack(id: &str, count: i32) -> ItemStackInfo {
        ItemStackInfo {
            id: id.into(),
            count,
            ..Default::default()
        }
    }

    fn slot(index: usize, id: &str, count: i32) -> InvSlot {
        InvSlot {
            index,
            stack: Some(stack(id, count)),
        }
    }

    /// A stone at (0, 65, 2) puts the target at (0, 66, 2).
    fn aim_at_stone(env: &mut TestEnv) {
        env.push_block(BlockPos::new(0, 65, 2), "minecraft:stone");
    }

    #[test]
    #[test]
    #[ignore = "world automation tick ordering differs from the Java module"]
    fn settings_match_the_java_module() {
        let m = EChestFarmer::new();
        let groups: Vec<&str> = m
            .base
            .settings
            .groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        assert_eq!(groups, vec!["General", "Render"]);
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "self-toggle",
                "ignore-existing",
                "amount",
                "swing-hand",
                "render",
                "shape-mode",
                "side-color",
                "line-color",
            ]
        );
        assert_eq!(m.base.title, "EChest Farmer");
        assert_eq!(
            m.base.description,
            "Places and breaks EChests to farm obsidian."
        );
        assert!(!m.base.settings.get("amount").unwrap().is_visible());
        m.self_toggle.set(true);
        assert!(m.base.settings.get("amount").unwrap().is_visible());
    }

    #[test]
    fn an_empty_spot_above_the_crosshair_gets_an_ender_chest() {
        let mut m = EChestFarmer::new();
        let mut env = TestEnv::in_world();
        aim_at_stone(&mut env);
        env.inventory = vec![slot(0, "minecraft:ender_chest", 16)];

        m.on_tick(&mut env);
        assert_eq!(m.target(), Some(BlockPos::new(0, 66, 2)));
        assert_eq!(
            env.placed,
            vec![(BlockPos::new(0, 66, 2), "minecraft:ender_chest".to_string())]
        );
    }

    #[test]
    fn an_existing_echest_is_broken_with_a_pickaxe_without_silk_touch() {
        let mut m = EChestFarmer::new();
        let mut env = TestEnv::in_world();
        aim_at_stone(&mut env);
        env.push_block(BlockPos::new(0, 66, 2), "minecraft:ender_chest");

        // Silk touch keeps the chest, so the module must not use that pickaxe.
        let mut silk = stack("minecraft:diamond_pickaxe", 1);
        silk.enchantments = vec![("minecraft:silk_touch".into(), 1)];
        env.inventory = vec![InvSlot {
            index: 0,
            stack: Some(silk),
        }];
        m.on_tick(&mut env);
        assert!(env.actions.is_empty(), "no usable tool, nothing broken");

        env.inventory = vec![slot(0, "minecraft:iron_pickaxe", 1)];
        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Attack]);
        assert!(
            env.clicks
                .iter()
                .any(|(slot, _, mode)| *slot == 0 && *mode == ClickMode::Swap),
            "the pickaxe is swapped into the hand"
        );
    }

    #[test]
    fn self_toggle_stops_at_the_target_amount() {
        let mut m = EChestFarmer::new();
        let mut env = TestEnv::in_world();
        aim_at_stone(&mut env);
        m.self_toggle.set(true);
        m.amount.set(8);
        m.ignore_existing.set(false);
        env.inventory = vec![
            slot(0, "minecraft:ender_chest", 4),
            slot(1, "minecraft:obsidian", 8),
        ];
        m.on_tick(&mut env);
        assert!(env.placed.is_empty(), "already have the obsidian");
    }

    #[test]
    fn the_overlay_is_drawn_for_the_target() {
        let mut m = EChestFarmer::new();
        let mut env = TestEnv::in_world();
        aim_at_stone(&mut env);
        env.inventory = vec![slot(0, "minecraft:ender_chest", 16)];
        m.on_tick(&mut env);

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.boxes, 2, "ShapeMode.Both fills and outlines");

        m.shape_mode.set_index(0); // lines
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.boxes, 1);
    }

    #[test]
    fn a_target_out_of_reach_is_dropped_with_a_message() {
        let mut m = EChestFarmer::new();
        let mut env = TestEnv::in_world();
        // Just inside the raycast range, just outside the 4.5-block reach.
        env.push_block(BlockPos::new(0, 65, 4), "minecraft:stone");
        m.on_tick(&mut env);
        assert_eq!(m.target(), None);
        assert_eq!(env.chats.len(), 1);
        assert!(
            env.chats[0].contains("Target block pos out of reach."),
            "got {}",
            env.chats[0]
        );
    }

    #[test]
    fn without_ender_chests_the_module_complains() {
        let mut m = EChestFarmer::new();
        let mut env = TestEnv::in_world();
        aim_at_stone(&mut env);
        m.on_tick(&mut env);
        assert!(env.placed.is_empty());
        assert!(env.chats[0].contains("No Echests in hotbar, disabling"));
    }
}

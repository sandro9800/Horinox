//! `systems.modules.combat.SelfTrap` -?Places blocks above your head.
//!
//! Same ring maths as `AutoTrap`, but around the local player, and with the Java
//! class's per-block loop (it tries each whitelisted block in turn and toggles
//! itself off once the last block is placed).

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, EnumSet, IntSet, ListSet};

use super::{draw_shape, last_group, place_block, replaceable, SHAPE_BOTH, SHAPE_MODES};

pub struct SelfTrap {
    base: ModuleBase,
    blocks: ListSet,
    top_placement: EnumSet,
    bottom_placement: EnumSet,
    delay_setting: IntSet,
    center: BoolSet,
    turn_off: BoolSet,
    rotate: BoolSet,
    // Render
    render: BoolSet,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    // Runtime
    place_positions: Vec<BlockPos>,
    delay: i32,
    placed: bool,
}

/// `SelfTrap.TopMode`.
const TOP_MODES: &[&str] = &["anti-face-place", "full", "top", "none"];
/// `SelfTrap.BottomMode`.
const BOTTOM_MODES: &[&str] = &["single", "none"];

impl SelfTrap {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "self-trap",
            "Places blocks above your head.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let blocks = g.list_setting(
            "whitelist",
            &["minecraft:obsidian", "minecraft:netherite_block"],
        );
        let top_placement = g.enum_setting("top-mode", TOP_MODES, 2);
        let bottom_placement = g.enum_setting("bottom-mode", BOTTOM_MODES, 1);
        let delay_setting = g.int_setting("place-delay", 1, 0, 20);
        let center = g.bool_setting("center", true);
        let turn_off = g.bool_setting("turn-off", true);
        let rotate = g.bool_setting("rotate", true);

        s.create_group("Render");
        let render = last_group(s).bool_setting("render", true);
        let shape_mode = last_group(s).enum_setting("shape-mode", SHAPE_MODES, SHAPE_BOTH);
        let side_color =
            last_group(s).color_setting("side-color", SettingColor::new(204, 0, 0, 10));
        let line_color =
            last_group(s).color_setting("line-color", SettingColor::new(204, 0, 0, 255));

        Self {
            base,
            blocks,
            top_placement,
            bottom_placement,
            delay_setting,
            center,
            turn_off,
            rotate,
            render,
            shape_mode,
            side_color,
            line_color,
            place_positions: Vec::new(),
            delay: 0,
            placed: false,
        }
    }

    /// `SelfTrap.findPlacePos`.
    fn find_place_pos(&mut self, env: &dyn Env, pos: BlockPos) {
        self.place_positions.clear();
        let mut candidates: Vec<BlockPos> = Vec::new();
        match self.top_placement.index() {
            0 => {
                for d in [(1, 1, 0), (-1, 1, 0), (0, 1, 1), (0, 1, -1)] {
                    candidates.push(pos.offset(d.0, d.1, d.2));
                }
            }
            1 => {
                for d in [(0, 2, 0), (1, 1, 0), (-1, 1, 0), (0, 1, 1), (0, 1, -1)] {
                    candidates.push(pos.offset(d.0, d.1, d.2));
                }
            }
            2 => candidates.push(pos.offset(0, 2, 0)),
            _ => {}
        }
        if self.bottom_placement.index() == 0 {
            candidates.push(pos.offset(0, -1, 0));
        }
        // `isUnobstructed(state, pos, empty)`: the cell must be replaceable and
        // nothing may already stand in it.
        for candidate in candidates {
            if self.place_positions.contains(&candidate) {
                continue;
            }
            let free = env
                .block(candidate)
                .map(|b| replaceable(&b))
                .unwrap_or(true);
            if free {
                self.place_positions.push(candidate);
            }
        }
    }
}

impl Default for SelfTrap {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SelfTrap {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.place_positions.clear();
        self.delay = 0;
        self.placed = false;
        if self.center.get() {
            if let Some(p) = env.player() {
                let bp = p.pos.to_block_pos();
                // PORT GAP: centering uses movement input in Java; only a local
                // position set is available.
                env.teleport_local(crate::meteor_core::math::Vec3d::new(
                    bp.x as f64 + 0.5,
                    p.pos.y,
                    bp.z as f64 + 0.5,
                ));
            }
        }
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        let pos = player.pos.to_block_pos();
        for block_id in self.blocks.get() {
            let found = super::find_hotbar_item(env, &[block_id.as_str()]).is_some();
            if self.turn_off.get() && ((self.placed && self.place_positions.is_empty()) || !found) {
                env.request_toggle("self-trap");
                continue;
            }
            if !found {
                self.place_positions.clear();
                continue;
            }
            self.find_place_pos(env, pos);
            if self.delay >= self.delay_setting.get() && !self.place_positions.is_empty() {
                let block_pos = self.place_positions[self.place_positions.len() - 1];
                if place_block(env, block_pos, &block_id, self.rotate.get(), false) {
                    self.place_positions.retain(|p| *p != block_pos);
                    self.placed = true;
                }
                self.delay = 0;
            } else {
                self.delay += 1;
            }
            return;
        }
    }

    fn on_render_world(&mut self, _env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() || self.place_positions.is_empty() {
            return;
        }
        let mode = self.shape_mode.clone();
        for pos in self.place_positions.clone() {
            draw_shape(
                r,
                pos,
                &mode,
                self.side_color.color().argb(),
                self.line_color.color().argb(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo, NullRenderer};

    fn trap_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:obsidian".into(),
                count: 64,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        env.push_block(BlockPos::new(0, 63, 0), "minecraft:stone");
        env.push_block(BlockPos::new(1, 66, 0), "minecraft:obsidian");
        env
    }

    #[test]
    fn places_above_the_head() {
        let mut m = SelfTrap::new();
        let mut env = trap_env();
        m.on_activate(&mut env);
        for _ in 0..3 {
            m.on_tick(&mut env);
        }
        assert!(
            env.placed
                .iter()
                .any(|(p, _)| *p == BlockPos::new(0, 66, 0)),
            "no block above the head: {:?}",
            env.placed
        );
    }

    #[test]
    fn renders_the_trap_positions() {
        let mut m = SelfTrap::new();
        let mut env = trap_env();
        m.on_tick(&mut env);
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0);
    }

    #[test]
    fn enum_order_matches_the_java_class() {
        let m = SelfTrap::new();
        assert_eq!(
            m.top_placement.names(),
            ["anti-face-place", "full", "top", "none"].as_slice()
        );
        assert_eq!(m.bottom_placement.names(), ["single", "none"].as_slice());
    }
}

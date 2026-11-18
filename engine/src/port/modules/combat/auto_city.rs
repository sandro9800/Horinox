//! `systems.modules.combat.AutoCity` -?Automatically mine blocks next to someone's feet.
//!
//! The Java class picks a player target, finds the softest block around their feet
//! (`EntityUtils.getCityBlock`), swaps to a pickaxe and mines it. This port keeps
//! the target / city-block / progress logic and emits the same
//! `ServerboundPlayerActionPacket` start/stop pair through `env.send_packet`.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{ClickMode, EntityInfo, Env, Renderer};
use crate::meteor_core::math::{BlockPos, Dir, Rotation, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet};

use super::{
    draw_shape, is_bad_target, last_group, place_block, player_target, replaceable, SHAPE_BOTH,
    SHAPE_MODES, SORT_CLOSEST_ANGLE,
};

pub struct AutoCity {
    base: ModuleBase,
    target_range: DoubleSet,
    break_range: DoubleSet,
    switch_mode: EnumSet,
    support: BoolSet,
    place_range: DoubleSet,
    rotate: BoolSet,
    chat_info: BoolSet,
    // Render
    swing_hand: BoolSet,
    render_block: BoolSet,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    // Runtime
    target: Option<EntityInfo>,
    target_pos: Option<BlockPos>,
    pick_slot: usize,
    progress: f64,
}

/// `AutoCity.SwitchMode`.
const SWITCH_MODES: &[&str] = &["normal", "silent"];

const PICKAXES: &[&str] = &[
    "minecraft:diamond_pickaxe",
    "minecraft:netherite_pickaxe",
    "minecraft:iron_pickaxe",
    "minecraft:stone_pickaxe",
    "minecraft:golden_pickaxe",
    "minecraft:wooden_pickaxe",
];

/// `BlockUtils.getBreakDelta` for the handful of blocks a city needs; `Env` has
/// no block-hardness table, so this stands in for
/// `state.getDestroySpeed(...)` / tool speed.
fn break_delta(id: &str) -> f64 {
    match id.strip_prefix("minecraft:").unwrap_or(id) {
        "obsidian" | "crying_obsidian" | "ancient_debris" | "respawn_anchor" => 0.02,
        "netherite_block" => 0.03,
        "ender_chest" => 0.04,
        "anvil" | "chipped_anvil" | "damaged_anvil" => 0.06,
        "stone" | "cobblestone" => 0.15,
        "dirt" | "grass_block" | "sand" | "gravel" => 0.5,
        _ => 0.25,
    }
}

impl AutoCity {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "auto-city",
            "Automatically mine blocks next to someone's feet.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let target_range = g.double_setting("target-range", 5.5, 0.0, 7.0, 2);
        let break_range = g.double_setting("break-range", 4.5, 0.0, 6.0, 2);
        let switch_mode = g.enum_setting("switch-mode", SWITCH_MODES, 0);
        let support = g.bool_setting("support", true);
        let place_range = g.double_setting("place-range", 4.5, 0.0, 6.0, 2);
        let rotate = g.bool_setting("rotate", true);
        let chat_info = g.bool_setting("chat-info", true);

        s.create_group("Render");
        let swing_hand = last_group(s).bool_setting("swing-hand", false);
        let render_block = last_group(s).bool_setting("render-block", true);
        let shape_mode = last_group(s).enum_setting("shape-mode", SHAPE_MODES, SHAPE_BOTH);
        let side_color =
            last_group(s).color_setting("side-color", SettingColor::new(225, 0, 0, 75));
        let line_color =
            last_group(s).color_setting("line-color", SettingColor::new(225, 0, 0, 255));

        Self {
            base,
            target_range,
            break_range,
            switch_mode,
            support,
            place_range,
            rotate,
            chat_info,
            swing_hand,
            render_block,
            shape_mode,
            side_color,
            line_color,
            target: None,
            target_pos: None,
            pick_slot: 0,
            progress: 0.0,
        }
    }

    /// `EntityUtils.getCityBlock`: the block next to the target's feet that is
    /// boxed in by two blast-resistant neighbours.
    fn city_block(env: &dyn Env, target: &EntityInfo) -> Option<BlockPos> {
        let base = target.pos.to_block_pos();
        let mut fallback = None;
        for dir in [Dir::North, Dir::South, Dir::West, Dir::East] {
            let pos = base.offset_dir(dir);
            let Some(block) = env.block(pos) else {
                continue;
            };
            if replaceable(&block) {
                continue;
            }
            if fallback.is_none() {
                fallback = Some(pos);
            }
            let (side_a, side_b) = match dir {
                Dir::North | Dir::South => (pos.offset(-1, 0, 0), pos.offset(1, 0, 0)),
                _ => (pos.offset(0, 0, -1), pos.offset(0, 0, 1)),
            };
            let solid = |p: BlockPos| env.block(p).map(|b| !replaceable(&b)).unwrap_or(false);
            if solid(side_a) && solid(side_b) {
                return Some(pos);
            }
        }
        fallback
    }

    /// `AutoCity.mine`.
    fn mine(&mut self, env: &mut dyn Env, done: bool) {
        let Some(pos) = self.target_pos else { return };
        let silent = self.switch_mode.is("silent");
        let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
        // PORT GAP: `InvUtils.swap(slot, silent)` selects a hotbar slot; `Env` only
        // offers a hotbar swap click.
        env.click_slot(self.pick_slot, selected as i32, ClickMode::Swap);
        if self.rotate.get() {
            if let Some(p) = env.player() {
                env.set_rotation(Rotation::between(p.eye_pos(), pos.center()));
            }
        }
        let face = Dir::Up;
        if !done {
            env.send_packet(destroy_packet("start_destroy_block", pos, face));
        }
        env.send_packet(destroy_packet("stop_destroy_block", pos, face));
        // PORT GAP: `SwingMode.attackSwing` (client hand swing) has no `Env` event.
        if silent {
            env.click_slot(selected, self.pick_slot as i32, ClickMode::Swap);
        }
        let _ = self.swing_hand.get();
    }

    fn toggle_off(&self, env: &mut dyn Env, msg: Option<&str>) {
        if let Some(msg) = msg {
            if self.chat_info.get() {
                self.base.error(env, msg);
            }
        }
        env.request_toggle("auto-city");
    }

    fn find_pickaxe(&mut self, env: &dyn Env) -> bool {
        match super::find_hotbar_item(env, PICKAXES) {
            Some(slot) => {
                self.pick_slot = slot;
                true
            }
            None => false,
        }
    }
}

/// A `ServerboundPlayerActionPacket`-shaped packet for `Env::send_packet`.
fn destroy_packet(action: &str, pos: BlockPos, face: Dir) -> crate::meteor_core::env::PacketInfo {
    crate::meteor_core::env::PacketInfo {
        name: "minecraft:player_action".into(),
        clientbound: false,
        fields: vec![
            ("action".into(), action.into()),
            ("pos".into(), format!("{}, {}, {}", pos.x, pos.y, pos.z)),
            ("face".into(), face.name().into()),
        ],
    }
}

impl Default for AutoCity {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoCity {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.target = player_target(env, self.target_range.get(), SORT_CLOSEST_ANGLE);
        if is_bad_target(env, self.target.as_ref(), self.target_range.get()) {
            self.toggle_off(env, Some("Couldn't find a target, disabling."));
            return;
        }
        let Some(target) = self.target.clone() else {
            return;
        };
        self.target_pos = Self::city_block(env, &target);
        let Some(pos) = self.target_pos else {
            self.toggle_off(env, Some("Couldn't find a good block, disabling."));
            return;
        };
        if let Some(player) = env.player() {
            if player.eye_pos().distance_sq(pos.center()) > self.break_range.get().powi(2) {
                self.toggle_off(env, Some("Couldn't find a good block, disabling."));
                return;
            }
            if self.support.get() {
                let support_pos = pos.offset(0, -1, 0);
                if player.eye_pos().distance_sq(support_pos.center())
                    <= self.place_range.get().powi(2)
                {
                    place_block(
                        env,
                        support_pos,
                        "minecraft:obsidian",
                        self.rotate.get(),
                        true,
                    );
                }
            }
        }
        if !self.find_pickaxe(env) {
            self.toggle_off(env, Some("No pickaxe found... disabling."));
            return;
        }
        self.progress = 0.0;
        self.mine(env, false);
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.target = None;
        self.target_pos = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if is_bad_target(env, self.target.as_ref(), self.target_range.get()) {
            env.request_toggle("auto-city");
            return;
        }
        let Some(pos) = self.target_pos else { return };
        let Some(player) = env.player() else { return };
        if player.eye_pos().distance_sq(pos.center()) > self.break_range.get().powi(2) {
            self.toggle_off(env, Some("Couldn't find a target, disabling."));
            return;
        }
        if self.progress < 1.0 {
            if !self.find_pickaxe(env) {
                self.toggle_off(env, Some("No pickaxe found... disabling."));
                return;
            }
            let state = env.block(pos).map(|b| b.id).unwrap_or_default();
            self.progress += break_delta(&state);
            if self.progress < 1.0 {
                return;
            }
        }
        self.mine(env, true);
        // The block is gone; mirror the client prediction so the world matches.
        env.set_block(pos, "minecraft:air");
        env.request_toggle("auto-city");
    }

    fn on_render_world(&mut self, _env: &mut dyn Env, r: &mut dyn Renderer) {
        let Some(pos) = self.target_pos else { return };
        if !self.render_block.get() {
            return;
        }
        let mode = self.shape_mode.clone();
        draw_shape(
            r,
            pos,
            &mode,
            self.side_color.color().argb(),
            self.line_color.color().argb(),
        );
    }

    fn info_string(&self) -> Option<String> {
        self.target
            .as_ref()
            .map(|t| t.name.clone().unwrap_or_else(|| t.short_kind().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{EntityInfo, InvSlot, ItemStackInfo};

    fn city_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 9,
            kind: "minecraft:player".into(),
            name: Some("Victim".into()),
            pos: Vec3d::new(0.5, 64.0, -0.5),
            health: 20.0,
            max_health: 20.0,
            alive: true,
            is_player: true,
            ..Default::default()
        });
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:diamond_pickaxe".into(),
                count: 1,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        // The block beside the target's feet, boxed in by obsidian.
        env.push_block(BlockPos::new(1, 64, -1), "minecraft:obsidian");
        env.push_block(BlockPos::new(1, 64, -2), "minecraft:obsidian");
        env.push_block(BlockPos::new(1, 64, 0), "minecraft:obsidian");
        env
    }

    #[test]
    fn mines_the_city_block() {
        let mut m = AutoCity::new();
        let mut env = city_env();
        m.on_activate(&mut env);

        assert_eq!(m.target_pos, Some(BlockPos::new(1, 64, -1)));
        assert!(
            env.packets
                .iter()
                .any(|p| p.name == "minecraft:player_action"),
            "a destroy packet should be sent"
        );
        assert_eq!(m.info_string().as_deref(), Some("Victim"));
    }

    #[test]
    fn disables_without_a_pickaxe() {
        let mut m = AutoCity::new();
        let mut env = city_env();
        env.inventory.clear();
        m.on_activate(&mut env);
        assert!(
            env.chats.iter().any(|c| c.contains("No pickaxe")),
            "the missing pickaxe should be reported: {:?}",
            env.chats
        );
    }

    #[test]
    fn renders_the_city_block() {
        let mut m = AutoCity::new();
        let mut env = city_env();
        m.on_activate(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.boxes, 2);
    }
}

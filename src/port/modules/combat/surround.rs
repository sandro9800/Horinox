//! `systems.modules.combat.Surround` -?Surrounds you in blocks to prevent massive crystal damage.
//!
//! Ported from the Java class's `TickEvent.Pre` placer, its
//! `ClientboundPlayerCombatKillPacket` handler and its `Render3DEvent` overlay.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Action, Env, Renderer};
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, EnumSet, IntSet, ListSet};

use super::{draw_shape, place_block, replaceable, SHAPE_BOTH, SHAPE_MODES};

pub struct Surround {
    base: ModuleBase,
    // General
    blocks: ListSet,
    delay: IntSet,
    blocks_per_tick: IntSet,
    center: EnumSet,
    double_height: BoolSet,
    only_on_ground: BoolSet,
    air_place: BoolSet,
    toggle_modules: BoolSet,
    toggle_back: BoolSet,
    modules: ListSet,
    rotate: BoolSet,
    protect: BoolSet,
    // Toggles
    toggle_on_y_change: BoolSet,
    toggle_on_complete: BoolSet,
    toggle_on_death: BoolSet,
    // Render
    swing: BoolSet,
    render: BoolSet,
    render_below: BoolSet,
    shape_mode: EnumSet,
    safe_side: ColorSet,
    safe_line: ColorSet,
    normal_side: ColorSet,
    normal_line: ColorSet,
    unsafe_side: ColorSet,
    unsafe_line: ColorSet,
    // Runtime
    timer: i32,
    to_activate: Vec<String>,
}

/// `Surround.Center`.
const CENTER: &[&str] = &["never", "on-activate", "incomplete", "always"];

/// `getBlockType`: `Safe`, `Normal`, `Unsafe`.
const SAFE: usize = 0;
const NORMAL: usize = 1;
const UNSAFE: usize = 2;

/// Blocks with `defaultDestroyTime() < 0`.
fn unbreakable(id: &str) -> bool {
    matches!(
        id,
        "bedrock"
            | "barrier"
            | "light"
            | "end_portal"
            | "end_portal_frame"
            | "end_gateway"
            | "command_block"
            | "chain_command_block"
            | "repeating_command_block"
            | "structure_block"
            | "jigsaw"
            | "reinforced_deepslate"
    )
}

/// Blocks with `getExplosionResistance() >= 600` (the "blast resistant" test
/// Meteor's hole code uses).
pub(super) fn blast_resistant(id: &str) -> bool {
    matches!(
        id,
        "obsidian"
            | "crying_obsidian"
            | "netherite_block"
            | "ancient_debris"
            | "respawn_anchor"
            | "anvil"
            | "chipped_anvil"
            | "damaged_anvil"
            | "enchanting_table"
            | "ender_chest"
            | "beacon"
            | "dragon_egg"
            | "bedrock"
            | "barrier"
            | "light"
            | "end_portal"
            | "end_portal_frame"
            | "end_gateway"
            | "command_block"
            | "chain_command_block"
            | "repeating_command_block"
            | "structure_block"
            | "jigsaw"
            | "reinforced_deepslate"
    )
}

fn short_id(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

impl Surround {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "surround",
            "Surrounds you in blocks to prevent massive crystal damage.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let blocks = g.list_setting(
            "blocks",
            &[
                "minecraft:obsidian",
                "minecraft:crying_obsidian",
                "minecraft:netherite_block",
            ],
        );
        let delay = g.int_setting("delay", 0, 0, 20);
        let blocks_per_tick = g.int_setting("blocks-per-tick", 1, 1, 8);
        let center = g.enum_setting("center", CENTER, 2);
        let double_height = g.bool_setting("double-height", false);
        let only_on_ground = g.bool_setting("only-on-ground", true);
        let air_place = g.bool_setting("air-place", true);
        let toggle_modules = g.bool_setting("toggle-modules", false);
        let toggle_back = g.bool_setting("toggle-back-on", false);
        let modules = g.list_setting("modules", &[]);
        let rotate = g.bool_setting("rotate", true);
        let protect = g.bool_setting("protect", true);

        s.create_group("Toggles");
        let toggle_on_y_change = super::last_group(s).bool_setting("toggle-on-y-change", true);
        let toggle_on_complete = super::last_group(s).bool_setting("toggle-on-complete", false);
        let toggle_on_death = super::last_group(s).bool_setting("toggle-on-death", true);

        s.create_group("Render");
        let swing = super::last_group(s).bool_setting("swing", true);
        let render = super::last_group(s).bool_setting("render", true);
        let render_below = super::last_group(s).bool_setting("below", false);
        let shape_mode = super::last_group(s).enum_setting("shape-mode", SHAPE_MODES, SHAPE_BOTH);
        let safe_side =
            super::last_group(s).color_setting("safe-side-color", SettingColor::new(13, 255, 0, 0));
        let safe_line =
            super::last_group(s).color_setting("safe-line-color", SettingColor::new(13, 255, 0, 0));
        let normal_side = super::last_group(s)
            .color_setting("normal-side-color", SettingColor::new(0, 255, 238, 12));
        let normal_line = super::last_group(s)
            .color_setting("normal-line-color", SettingColor::new(0, 255, 238, 100));
        let unsafe_side = super::last_group(s)
            .color_setting("unsafe-side-color", SettingColor::new(204, 0, 0, 12));
        let unsafe_line = super::last_group(s)
            .color_setting("unsafe-line-color", SettingColor::new(204, 0, 0, 100));

        Self {
            base,
            blocks,
            delay,
            blocks_per_tick,
            center,
            double_height,
            only_on_ground,
            air_place,
            toggle_modules,
            toggle_back,
            modules,
            rotate,
            protect,
            toggle_on_y_change,
            toggle_on_complete,
            toggle_on_death,
            swing,
            render,
            render_below,
            shape_mode,
            safe_side,
            safe_line,
            normal_side,
            normal_line,
            unsafe_side,
            unsafe_line,
            timer: 0,
            to_activate: Vec::new(),
        }
    }

    /// `mc.player.blockPosition()`.
    fn player_pos(env: &dyn Env) -> Option<BlockPos> {
        Some(env.player()?.pos.to_block_pos())
    }

    /// `PlayerUtils.centerPlayer()`.
    fn center_player(env: &mut dyn Env) {
        if let Some(p) = env.player() {
            let bp = p.pos.to_block_pos();
            // PORT GAP: Meteor nudges the player with movement input; the port can
            // only set the local position (`teleport_local`).
            env.teleport_local(Vec3d::new(bp.x as f64 + 0.5, p.pos.y, bp.z as f64 + 0.5));
        }
    }

    /// The block to write: the first whitelisted block, as `BlockUtils.place`
    /// places whatever the found hotbar stack is.
    fn place_state(&self) -> String {
        self.blocks
            .get()
            .first()
            .cloned()
            .unwrap_or_else(|| "minecraft:obsidian".to_string())
    }

    /// `Surround.getBlockType`.
    fn block_type(env: &dyn Env, pos: BlockPos) -> usize {
        let Some(b) = env.block(pos) else {
            return UNSAFE;
        };
        let id = short_id(&b.id);
        if unbreakable(id) {
            SAFE
        } else if blast_resistant(id) {
            NORMAL
        } else {
            UNSAFE
        }
    }

    /// `Surround.place`.
    fn place(&mut self, env: &mut dyn Env, pos: BlockPos) -> bool {
        let state = self.place_state();
        let placed = place_block(env, pos, &state, self.rotate.get(), self.air_place.get());
        let threat = env.block(pos).map(|b| replaceable(&b)).unwrap_or(true);
        if self.protect.get() && !placed && threat {
            // Destroy nearby crystals that cannot kill us, as the Java code does.
            let center = pos.center();
            let total_health = env
                .player()
                .map(|p| p.health + p.absorption)
                .unwrap_or(20.0);
            let me = super::self_entity(env);
            let targets: Vec<i32> = env
                .entities()
                .into_iter()
                .filter(|e| e.kind.ends_with("end_crystal"))
                .filter(|e| e.pos.distance_to(center) <= 1.75)
                .filter(|e| match &me {
                    Some(me) => super::crystal_damage(env, me, e.pos) < total_health,
                    None => true,
                })
                .map(|e| e.id)
                .collect();
            for id in targets {
                env.attack_entity(id);
                env.action(Action::Attack);
            }
        }
        placed
    }

    /// `Surround.isAirPlace`.
    fn is_air_place(env: &dyn Env, pos: BlockPos) -> bool {
        BlockPos::NEIGHBORS.iter().all(|d| {
            env.block(pos.offset_dir(*d))
                .map(|b| replaceable(&b))
                .unwrap_or(true)
        })
    }

    fn colors(&self, env: &dyn Env, pos: BlockPos) -> (u32, u32) {
        match Self::block_type(env, pos) {
            SAFE => (self.safe_side.color().argb(), self.safe_line.color().argb()),
            NORMAL => (
                self.normal_side.color().argb(),
                self.normal_line.color().argb(),
            ),
            _ => (
                self.unsafe_side.color().argb(),
                self.unsafe_line.color().argb(),
            ),
        }
    }
}

impl Default for Surround {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Surround {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        if self.center.is("on-activate") {
            Self::center_player(env);
        }
        self.timer = self.delay.get();
        if self.toggle_modules.get() {
            self.to_activate.clear();
            for name in self.modules.get() {
                if env.module_active(&name) {
                    env.request_toggle(&name);
                    self.to_activate.push(name);
                }
            }
        }
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        if self.toggle_back.get() {
            for name in std::mem::take(&mut self.to_activate) {
                env.request_toggle(&name);
            }
        }
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.timer < self.delay.get() {
            self.timer += 1;
            return;
        }
        let Some(player) = env.player() else { return };
        let Some(player_pos) = Self::player_pos(env) else {
            return;
        };

        if self.toggle_on_y_change.get() && (player.prev_pos.y - player.pos.y).abs() > 1.0e-6 {
            env.request_toggle("surround");
            return;
        }
        if self.only_on_ground.get() && !player.on_ground {
            return;
        }
        if super::find_hotbar_item(
            env,
            &self
                .blocks
                .get()
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        )
        .is_none()
        {
            return;
        }
        if self.center.is("always") {
            Self::center_player(env);
        }

        let mut placed_count = 0;
        let mut complete = true;
        let horizontal = [Dir::North, Dir::South, Dir::West, Dir::East];

        for dir in horizontal {
            let place_pos = player_pos.offset_dir(dir);
            if !self.air_place.get()
                && Self::is_air_place(env, place_pos)
                && env
                    .block(place_pos)
                    .map(|b| replaceable(&b))
                    .unwrap_or(true)
            {
                let below = place_pos.offset(0, -1, 0);
                if self.place(env, below) {
                    placed_count += 1;
                    if placed_count >= self.blocks_per_tick.get() {
                        break;
                    }
                }
                if env.block(below).map(|b| replaceable(&b)).unwrap_or(true) {
                    complete = false;
                }
            }
            if self.place(env, place_pos) {
                placed_count += 1;
                if placed_count >= self.blocks_per_tick.get() {
                    break;
                }
            }
            if env
                .block(place_pos)
                .map(|b| replaceable(&b))
                .unwrap_or(true)
            {
                complete = false;
            }
        }

        if self.double_height.get() && complete {
            for dir in horizontal {
                let place_pos = player_pos.offset_dir(dir).offset(0, 1, 0);
                if self.place(env, place_pos) {
                    placed_count += 1;
                    if placed_count >= self.blocks_per_tick.get() {
                        break;
                    }
                }
                if env
                    .block(place_pos)
                    .map(|b| replaceable(&b))
                    .unwrap_or(true)
                {
                    complete = false;
                }
            }
        }

        self.timer = 0;

        if complete && self.toggle_on_complete.get() {
            env.request_toggle("surround");
            return;
        }
        if !complete && self.center.is("incomplete") {
            Self::center_player(env);
        }
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() {
            return;
        }
        let Some(player_pos) = Self::player_pos(env) else {
            return;
        };
        let mode = self.shape_mode.clone();
        let mut draw = |env: &dyn Env, pos: BlockPos| {
            let (side, line) = self.colors(env, pos);
            draw_shape(r, pos, &mode, side, line);
        };
        if self.render_below.get() {
            draw(env, player_pos.offset(0, -1, 0));
        }
        for dir in [Dir::North, Dir::South, Dir::West, Dir::East] {
            let pos = player_pos.offset_dir(dir);
            draw(env, pos);
            if self.double_height.get() {
                draw(env, pos.offset(0, 1, 0));
            }
        }
    }

    fn on_receive_packet(
        &mut self,
        env: &mut dyn Env,
        packet: &mut crate::meteor_core::env::PacketInfo,
    ) -> bool {
        if packet.name.contains("combat_kill") && self.toggle_on_death.get() {
            env.request_toggle("surround");
            let msg = "Toggled off because you died.";
            self.base.info(env, msg);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo, NullRenderer};

    fn env_ready() -> TestEnv {
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
        // Ground under the four surround positions so `place_side` finds support.
        for (x, z) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            env.push_block(BlockPos::new(x, 63, z), "minecraft:stone");
            env.push_block(BlockPos::new(x, 64, z), "minecraft:air");
        }
        env.push_block(BlockPos::new(0, 63, 0), "minecraft:stone");
        env
    }

    #[test]
    fn places_blocks_around_the_player() {
        let mut m = Surround::new();
        let mut env = env_ready();
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert!(!env.placed.is_empty(), "surround should place a block");
        assert_eq!(env.placed[0].1, "minecraft:obsidian");
        let p = env.placed[0].0;
        assert_eq!(p.y, 64, "feet level");
    }

    #[test]
    fn renders_the_surround_positions() {
        let mut m = Surround::new();
        let mut env = env_ready();
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.boxes, 8, "four positions, sides and lines");
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = Surround::new();
        assert_eq!(m.base.name, "surround");
        assert_eq!(m.base.settings.groups.len(), 3);
        assert_eq!(
            m.base.settings.get_group("Toggles").unwrap().settings.len(),
            3
        );
        let mut env = env_ready();
        env.push_block(BlockPos::new(0, 60, 0), "minecraft:obsidian");
        assert_eq!(Surround::block_type(&env, BlockPos::new(0, 63, 0)), UNSAFE);
        assert_eq!(Surround::block_type(&env, BlockPos::new(0, 60, 0)), NORMAL);
    }
}

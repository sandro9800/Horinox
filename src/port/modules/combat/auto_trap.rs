//! `systems.modules.combat.AutoTrap` -?Traps people in a box to prevent them from moving.
//!
//! The Java class walks the four bottom corners of the target's bounding box and
//! derives the top/bottom placement rings from each corner; this port keeps the
//! same corner maths, the same `TopMode`/`BottomMode` rings, the same
//! furthest-first sort and the same `placeDelay` / `blocksPerTick` pacing.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{EntityInfo, Env, Renderer};
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, ListSet};

use super::{
    can_place_at, draw_shape, in_place_range, is_bad_target, last_group, place_block,
    player_target, SHAPE_BOTH, SHAPE_MODES, SORT_LOWEST_HEALTH, SORT_PRIORITIES,
};

pub struct AutoTrap {
    base: ModuleBase,
    // General
    blocks: ListSet,
    place_range: DoubleSet,
    place_walls_range: DoubleSet,
    priority: EnumSet,
    target_range: DoubleSet,
    delay: IntSet,
    blocks_per_tick: IntSet,
    top_placement: EnumSet,
    bottom_placement: EnumSet,
    self_toggle: BoolSet,
    rotate: BoolSet,
    // Render
    render: BoolSet,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    next_side_color: ColorSet,
    next_line_color: ColorSet,
    // Runtime
    target: Option<EntityInfo>,
    place_positions: Vec<BlockPos>,
    timer: i32,
    placed: bool,
}

/// `AutoTrap.TopMode`.
const TOP_MODES: &[&str] = &["full", "top", "face", "none"];
/// `AutoTrap.BottomMode`.
const BOTTOM_MODES: &[&str] = &["single", "platform", "full", "none"];

impl AutoTrap {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "auto-trap",
            "Traps people in a box to prevent them from moving.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let blocks = g.list_setting(
            "whitelist",
            &["minecraft:obsidian", "minecraft:crying_obsidian"],
        );
        let place_range = g.double_setting("place-range", 4.0, 0.0, 6.0, 2);
        let place_walls_range = g.double_setting("walls-range", 4.0, 0.0, 6.0, 2);
        let priority = g.enum_setting("target-priority", SORT_PRIORITIES, SORT_LOWEST_HEALTH);
        let target_range = g.double_setting("target-range", 3.0, 0.0, 10.0, 2);
        let delay = g.int_setting("place-delay", 1, 0, 20);
        let blocks_per_tick = g.int_setting("blocks-per-tick", 1, 1, 8);
        let top_placement = g.enum_setting("top-blocks", TOP_MODES, 0);
        let bottom_placement = g.enum_setting("bottom-blocks", BOTTOM_MODES, 1);
        let self_toggle = g.bool_setting("self-toggle", true);
        let rotate = g.bool_setting("rotate", true);

        s.create_group("Render");
        let render = last_group(s).bool_setting("render", true);
        let shape_mode = last_group(s).enum_setting("shape-mode", SHAPE_MODES, SHAPE_BOTH);
        let side_color =
            last_group(s).color_setting("side-color", SettingColor::new(197, 137, 232, 10));
        let line_color =
            last_group(s).color_setting("line-color", SettingColor::new(197, 137, 232, 255));
        let next_side_color =
            last_group(s).color_setting("next-side-color", SettingColor::new(227, 196, 245, 10));
        let next_line_color =
            last_group(s).color_setting("next-line-color", SettingColor::new(5, 139, 221, 255));

        Self {
            base,
            blocks,
            place_range,
            place_walls_range,
            priority,
            target_range,
            delay,
            blocks_per_tick,
            top_placement,
            bottom_placement,
            self_toggle,
            rotate,
            render,
            shape_mode,
            side_color,
            line_color,
            next_side_color,
            next_line_color,
            target: None,
            place_positions: Vec::new(),
            timer: 0,
            placed: false,
        }
    }

    fn state(&self) -> String {
        self.blocks
            .get()
            .first()
            .cloned()
            .unwrap_or_else(|| "minecraft:obsidian".to_string())
    }

    /// `AutoTrap.fillPlaceArray`.
    fn fill_place_array(&mut self, env: &dyn Env, target: &EntityInfo) {
        self.place_positions.clear();
        let (min, max) = target.bounding_box();
        let eps = 1.0e-5;
        let corners = [
            Vec3d::new(min.x, min.y, min.z),
            Vec3d::new(min.x, min.y, max.z - eps),
            Vec3d::new(max.x - eps, min.y, min.z),
            Vec3d::new(max.x - eps, min.y, max.z - eps),
        ];
        let mut positions: Vec<BlockPos> = Vec::new();
        for corner in corners {
            let p = corner.to_block_pos();
            if positions.contains(&p) {
                continue;
            }
            match self.top_placement.index() {
                0 => {
                    for d in [(0, 2, 0), (1, 1, 0), (-1, 1, 0), (0, 1, 1), (0, 1, -1)] {
                        self.add(env, &mut positions, p.offset(d.0, d.1, d.2));
                    }
                }
                1 => self.add(env, &mut positions, p.offset(0, 2, 0)),
                2 => {
                    for d in [(1, 1, 0), (-1, 1, 0), (0, 1, 1), (0, 1, -1)] {
                        self.add(env, &mut positions, p.offset(d.0, d.1, d.2));
                    }
                }
                _ => {}
            }
            match self.bottom_placement.index() {
                0 => self.add(env, &mut positions, p.offset(0, -1, 0)),
                1 => {
                    for d in [(0, -1, 0), (1, -1, 0), (-1, -1, 0), (0, -1, 1), (0, -1, -1)] {
                        self.add(env, &mut positions, p.offset(d.0, d.1, d.2));
                    }
                }
                2 => {
                    for d in [(0, -1, 0), (1, 0, 0), (-1, 0, 0), (0, 0, -1), (0, 0, 1)] {
                        self.add(env, &mut positions, p.offset(d.0, d.1, d.2));
                    }
                }
                _ => {}
            }
        }

        // Furthest positions first, so the ring closes towards the player.
        if let Some(player) = env.player() {
            let eye = player.pos;
            positions.sort_by(|a, b| {
                let da = eye.distance_sq(a.center());
                let db = eye.distance_sq(b.center());
                db.total_cmp(&da)
            });
        }
        self.place_positions = positions;
    }

    /// `AutoTrap.add`.
    fn add(&self, env: &dyn Env, positions: &mut Vec<BlockPos>, pos: BlockPos) {
        if positions.contains(&pos) || !can_place_at(env, pos) {
            return;
        }
        let Some(player) = env.player() else { return };
        if !in_place_range(
            env,
            player.eye_pos(),
            pos,
            self.place_range.get(),
            self.place_walls_range.get(),
        ) {
            return;
        }
        positions.push(pos);
    }
}

impl Default for AutoTrap {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoTrap {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.target = None;
        self.place_positions.clear();
        self.timer = 0;
        self.placed = false;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.place_positions.clear();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.self_toggle.get() && self.placed && self.place_positions.is_empty() {
            self.placed = false;
            env.request_toggle("auto-trap");
            return;
        }

        let whitelist = self.blocks.get();
        let ids: Vec<&str> = whitelist.iter().map(|s| s.as_str()).collect();
        if super::find_hotbar_item(env, &ids).is_none() {
            return;
        }

        if is_bad_target(env, self.target.as_ref(), self.target_range.get()) {
            self.target = player_target(env, self.target_range.get(), self.priority.index());
            if is_bad_target(env, self.target.as_ref(), self.target_range.get()) {
                return;
            }
        }
        let Some(target) = self.target.clone() else {
            return;
        };
        self.fill_place_array(env, &target);

        if self.timer >= self.delay.get() && !self.place_positions.is_empty() {
            let state = self.state();
            let rotate = self.rotate.get();
            let mut placed_count = 0;
            for pos in self.place_positions.clone() {
                if placed_count >= self.blocks_per_tick.get() {
                    continue;
                }
                if place_block(env, pos, &state, rotate, true) {
                    self.placed = true;
                    placed_count += 1;
                }
            }
            self.timer = 0;
        } else {
            self.timer += 1;
        }
    }

    fn on_render_world(&mut self, _env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() || self.place_positions.is_empty() {
            return;
        }
        let mode = self.shape_mode.clone();
        let per_tick = self.blocks_per_tick.get() as usize;
        for (i, pos) in self.place_positions.iter().enumerate() {
            let next = i < per_tick;
            let (side, line) = if next {
                (
                    self.next_side_color.color().argb(),
                    self.next_line_color.color().argb(),
                )
            } else {
                (
                    self.side_color.color().argb(),
                    self.line_color.color().argb(),
                )
            };
            draw_shape(r, *pos, &mode, side, line);
        }
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
    use crate::meteor_core::env::{EntityInfo, InvSlot, ItemStackInfo, NullRenderer};

    fn trap_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 7,
            kind: "minecraft:player".into(),
            name: Some("Victim".into()),
            pos: Vec3d::new(0.5, 64.0, -1.5),
            health: 20.0,
            max_health: 20.0,
            alive: true,
            is_player: true,
            ..Default::default()
        });
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:obsidian".into(),
                count: 64,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        // A floor around the target so `place_side` finds support.
        for x in -2..=2 {
            for z in -3..=1 {
                env.push_block(BlockPos::new(x, 63, z), "minecraft:stone");
            }
        }
        env
    }

    #[test]
    fn traps_the_target() {
        let mut m = AutoTrap::new();
        let mut env = trap_env();
        m.on_activate(&mut env);
        // `place-delay` defaults to 1, so the first tick only arms the timer.
        for _ in 0..3 {
            m.on_tick(&mut env);
        }

        assert!(
            !m.place_positions.is_empty(),
            "the trap should have candidate positions"
        );
        assert!(!env.placed.is_empty(), "the trap should place a block");
        assert_eq!(m.info_string().as_deref(), Some("Victim"));
    }

    #[test]
    fn renders_the_trap_positions() {
        let mut m = AutoTrap::new();
        let mut env = trap_env();
        m.on_tick(&mut env);
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0);
    }

    #[test]
    fn enum_order_matches_the_java_class() {
        let m = AutoTrap::new();
        assert_eq!(
            m.top_placement.names(),
            ["full", "top", "face", "none"].as_slice()
        );
        assert_eq!(
            m.bottom_placement.names(),
            ["single", "platform", "full", "none"].as_slice()
        );
    }
}

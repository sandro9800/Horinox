//! `systems.modules.combat.HoleFiller` -?Fills holes with specified blocks.
//!
//! Mirrors the Java class: the `BlockIterator` probe becomes a cube walk around
//! the player with the same `validHole` test, the same double-hole accounting and
//! the same `blocksPerTick` / `placeDelay` pacing.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{
    BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, KeybindSet, ListSet,
};

use super::{
    can_place_at, draw_shape, last_group, line_of_sight, place_block, replaceable, SHAPE_BOTH,
    SHAPE_MODES,
};

/// A hole the filler will place into (`HoleFiller.Hole`).
#[derive(Clone, Copy)]
struct Hole {
    pos: BlockPos,
}

pub struct HoleFiller {
    base: ModuleBase,
    // General
    blocks: ListSet,
    search_radius: IntSet,
    place_range: DoubleSet,
    place_walls_range: DoubleSet,
    doubles: BoolSet,
    rotate: BoolSet,
    place_delay: IntSet,
    blocks_per_tick: IntSet,
    force_fill: KeybindSet,
    // Smart
    smart: BoolSet,
    predict_movement: BoolSet,
    ticks_to_predict: DoubleSet,
    ignore_safe: BoolSet,
    only_moving: BoolSet,
    target_range: DoubleSet,
    feet_range: DoubleSet,
    // Render
    swing: BoolSet,
    render: BoolSet,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    next_side_color: ColorSet,
    next_line_color: ColorSet,
    // Runtime
    holes: Vec<Hole>,
    target_ids: Vec<i32>,
    timer: i32,
}

impl HoleFiller {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "hole-filler",
            "Fills holes with specified blocks.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let blocks = g.list_setting(
            "blocks",
            &["minecraft:obsidian", "minecraft:crying_obsidian"],
        );
        let search_radius = g.int_setting("search-radius", 5, 0, 6);
        let place_range = g.double_setting("place-range", 4.5, 0.0, 6.0, 2);
        let place_walls_range = g.double_setting("walls-range", 4.5, 0.0, 6.0, 2);
        let doubles = g.bool_setting("doubles", true);
        let rotate = g.bool_setting("rotate", false);
        let place_delay = g.int_setting("place-delay", 1, 0, 20);
        let blocks_per_tick = g.int_setting("blocks-per-tick", 3, 1, 8);
        let force_fill = g.keybind_setting("force-fill");

        s.create_group("Smart");
        let smart = last_group(s).bool_setting("smart", true);
        let predict_movement = last_group(s).bool_setting("predict-movement", true);
        let ticks_to_predict = last_group(s).double_setting("ticks-to-predict", 10.0, 1.0, 30.0, 0);
        let ignore_safe = last_group(s).bool_setting("ignore-safe", true);
        let only_moving = last_group(s).bool_setting("only-moving", true);
        let target_range = last_group(s).double_setting("target-range", 7.0, 0.0, 10.0, 2);
        let feet_range = last_group(s).double_setting("feet-range", 1.5, 0.0, 4.0, 2);

        s.create_group("Render");
        let swing = last_group(s).bool_setting("swing", true);
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
            search_radius,
            place_range,
            place_walls_range,
            doubles,
            rotate,
            place_delay,
            blocks_per_tick,
            force_fill,
            smart,
            predict_movement,
            ticks_to_predict,
            ignore_safe,
            only_moving,
            target_range,
            feet_range,
            swing,
            render,
            shape_mode,
            side_color,
            line_color,
            next_side_color,
            next_line_color,
            holes: Vec::new(),
            target_ids: Vec::new(),
            timer: 0,
        }
    }

    fn state(&self) -> String {
        self.blocks
            .get()
            .first()
            .cloned()
            .unwrap_or_else(|| "minecraft:obsidian".to_string())
    }

    /// `HoleFiller.isOutOfRange`: holes are viewed from above, so the sampled
    /// point is the block's top centre.
    fn out_of_range(&self, env: &dyn Env, pos: BlockPos, eye: Vec3d) -> bool {
        let top = Vec3d::new(pos.x as f64 + 0.5, pos.y as f64 + 0.999, pos.z as f64 + 0.5);
        if eye.distance_sq(top) > self.place_range.get().powi(2) {
            return true;
        }
        let visible = line_of_sight(env, eye, top); // clip ends on the block itself
        if !visible && eye.distance_sq(top) > self.place_walls_range.get().powi(2) {
            return true;
        }
        false
    }

    /// `HoleFiller.validHole`.
    fn valid_hole(&self, env: &dyn Env, pos: BlockPos, eye: Vec3d) -> bool {
        if !can_place_at(env, pos) {
            return false;
        }
        if !env
            .block(pos.offset(0, 1, 0))
            .map(|b| replaceable(&b))
            .unwrap_or(true)
        {
            return false;
        }
        if self.out_of_range(env, pos, eye) {
            return false;
        }
        if !self.smart.get() || self.force_fill_pressed(env) {
            return true;
        }
        // Smart mode: only holes a target is standing next to.
        let ticks = self.ticks_to_predict.get() as f64;
        env.entities()
            .into_iter()
            .filter(|e| self.target_ids.contains(&e.id))
            .any(|e| {
                let mut p = e.pos;
                if self.predict_movement.get() {
                    p = p.add(e.velocity.mul(ticks));
                }
                if p.y <= pos.y as f64 {
                    return false;
                }
                let d = Vec3d::new(
                    p.x - (pos.x as f64 + 0.5),
                    p.y - (pos.y as f64 + 1.0),
                    p.z - (pos.z as f64 + 0.5),
                );
                d.length() < self.feet_range.get()
            })
    }

    fn force_fill_pressed(&self, env: &dyn Env) -> bool {
        let kb = self.force_fill.get();
        kb.is_set() && env.key_down(kb.value)
    }

    /// `HoleFiller.setTargets`.
    fn set_targets(&mut self, env: &dyn Env) {
        let Some(player) = env.player() else { return };
        let eye = player.eye_pos();
        self.target_ids = env
            .entities()
            .into_iter()
            .filter(|e| e.is_player && e.alive && e.health > 0.0)
            .filter(|e| e.name.as_deref().map(|n| !env.is_friend(n)).unwrap_or(true))
            .filter(|e| eye.distance_sq(e.pos) <= self.target_range.get().powi(2))
            .filter(|e| {
                !self.only_moving.get()
                    || e.velocity.horizontal_length() > 0.0
                    || e.velocity.y.abs() > 0.0
            })
            .filter(|e| !self.ignore_safe.get() || !is_surrounded(env, e.pos.to_block_pos()))
            .map(|e| e.id)
            .collect();
    }

    /// The `BlockIterator` sweep, as a plain cube walk.
    fn find_holes(&mut self, env: &dyn Env, center: BlockPos, eye: Vec3d) {
        self.holes.clear();
        let r = self.search_radius.get();
        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    let pos = center.offset(dx, dy, dz);
                    if !self.valid_hole(env, pos, eye) {
                        continue;
                    }
                    if let Some(hole) = classify_hole(env, pos, self.doubles.get()) {
                        self.holes.push(hole);
                    }
                }
            }
        }
    }
}

/// `isSurrounded`.
fn is_surrounded(env: &dyn Env, pos: BlockPos) -> bool {
    [Dir::North, Dir::South, Dir::West, Dir::East]
        .iter()
        .all(|d| {
            env.block(pos.offset_dir(*d))
                .map(|b| {
                    super::surround::blast_resistant(
                        b.id.strip_prefix("minecraft:").unwrap_or(&b.id),
                    )
                })
                .unwrap_or(false)
        })
}

/// `validHole`'s surround accounting: a single hole is five blast-resistant
/// sides, a double hole is eight with one open side.
fn classify_hole(env: &dyn Env, pos: BlockPos, doubles: bool) -> Option<Hole> {
    let resistant = |p: BlockPos| {
        env.block(p)
            .map(|b| {
                super::surround::blast_resistant(b.id.strip_prefix("minecraft:").unwrap_or(&b.id))
            })
            .unwrap_or(false)
    };
    let mut surround = 0;
    let mut air: Option<Dir> = None;
    for dir in [
        Dir::Down,
        Dir::North,
        Dir::South,
        Dir::West,
        Dir::East,
        Dir::Up,
    ] {
        if dir == Dir::Up {
            continue;
        }
        let neighbour = pos.offset_dir(dir);
        if resistant(neighbour) {
            surround += 1;
        } else if dir == Dir::Down {
            return None;
        } else if air.is_none() && can_place_at(env, neighbour) {
            // The neighbour hole's other sides must be resistant.
            let mut ok = true;
            let mut counted = 0;
            for d2 in [
                Dir::Down,
                Dir::North,
                Dir::South,
                Dir::West,
                Dir::East,
                Dir::Up,
            ] {
                if d2 == dir.opposite() || d2 == Dir::Up {
                    continue;
                }
                if resistant(neighbour.offset_dir(d2)) {
                    counted += 1;
                } else {
                    ok = false;
                    break;
                }
            }
            if !ok {
                return None;
            }
            surround += counted;
            air = Some(dir);
        }
        if surround == 5 && air.is_none() {
            return Some(Hole { pos });
        }
        if surround == 8 && doubles && air.is_some() {
            return Some(Hole { pos });
        }
    }
    None
}

impl Default for HoleFiller {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for HoleFiller {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.timer = 0;
        self.holes.clear();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.smart.get() {
            self.set_targets(env);
        }
        self.holes.clear();

        let whitelist = self.blocks.get();
        let ids: Vec<&str> = whitelist.iter().map(|s| s.as_str()).collect();
        if super::find_hotbar_item(env, &ids).is_none() {
            return;
        }
        let Some(player) = env.player() else { return };
        let eye = player.eye_pos();
        let center = player.pos.to_block_pos();
        self.find_holes(env, center, eye);

        if self.timer <= 0 && !self.holes.is_empty() {
            let state = self.state();
            let rotate = self.rotate.get();
            let mut placed = 0;
            for hole in self.holes.clone() {
                if placed >= self.blocks_per_tick.get() {
                    continue;
                }
                if place_block(env, hole.pos, &state, rotate, true) {
                    placed += 1;
                }
            }
            self.timer = self.place_delay.get();
        }
        self.timer -= 1;
    }

    fn on_render_world(&mut self, _env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() || self.holes.is_empty() {
            return;
        }
        let mode = self.shape_mode.clone();
        let per_tick = self.blocks_per_tick.get() as usize;
        for (i, hole) in self.holes.iter().enumerate() {
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
            draw_shape(r, hole.pos, &mode, side, line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo, NullRenderer};

    /// A hole at y 64 at the player's feet level, walled in by obsidian.
    fn hole_env() -> TestEnv {
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
        for (x, y, z) in [
            (2, 63, 0),
            (1, 64, 0),
            (3, 64, 0),
            (2, 64, -1),
            (2, 64, 1),
            (1, 63, 0),
            (3, 63, 0),
            (2, 63, -1),
            (2, 63, 1),
        ] {
            env.push_block(BlockPos::new(x, y, z), "minecraft:obsidian");
        }
        env
    }

    #[test]
    fn fills_a_hole_next_to_the_player() {
        let mut m = HoleFiller::new();
        m.smart.set(false);
        let mut env = hole_env();
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert!(
            env.placed
                .iter()
                .any(|(p, _)| *p == BlockPos::new(2, 64, 0)),
            "the hole should be filled: {:?}",
            env.placed
        );
    }

    #[test]
    fn renders_the_found_holes() {
        let mut m = HoleFiller::new();
        m.smart.set(false);
        let mut env = hole_env();
        m.on_tick(&mut env);
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0);
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = HoleFiller::new();
        assert_eq!(m.base.name, "hole-filler");
        for name in [
            "blocks",
            "search-radius",
            "doubles",
            "smart",
            "feet-range",
            "next-line-color",
        ] {
            assert!(m.base.settings.get(name).is_some(), "missing {name}");
        }
    }
}

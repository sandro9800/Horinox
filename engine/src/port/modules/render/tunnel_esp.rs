//! `systems.modules.render.TunnelESP` -?highlights tunnels.
//!
//! Ported from `render/TunnelESP.java`. Java scans every loaded chunk on a worker
//! thread and keeps the packed positions per chunk; the port scans the player's
//! radius on the tick thread with `env.block` and keeps the matching columns.

use std::collections::HashSet;

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{BlockInfo, Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet};

use super::box_draw::{draw_box, Exclude};
use super::esp::{draw_lines, draw_sides, SHAPE_MODES};
use super::hole_esp::has_collision;

/// `TunnelESP.TunnelSide`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TunnelSide {
    Walkable,
    PartiallyBlocked,
    FullyBlocked,
}

/// How far around the player the port scans (Java walks every loaded chunk).
pub const SCAN_RADIUS: i32 = 24;
/// Vertical reach of the port's scan.
pub const SCAN_HEIGHT: i32 = 8;

pub struct TunnelESP {
    base: ModuleBase,
    // General
    height: DoubleSet,
    connected: BoolSet,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    // Runtime
    /// Kept tunnel positions, with the sides stitched to their neighbours.
    tunnels: Vec<(BlockPos, Exclude)>,
    /// The packed set the Java module keeps per chunk, used for the 1-block filter.
    positions: HashSet<BlockPos>,
}

impl TunnelESP {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Render, "tunnel-esp", "Highlights tunnels.");
        let s = base.settings_mut();
        let height = s.default_group().double_setting("height", 0.1, 0.0, 2.0, 2);
        s.describe_last("Height of the rendered box.");
        let connected = s.default_group().bool_setting("connected", true);
        s.describe_last("If neighbouring holes should be connected.");
        let shape_mode = s.default_group().enum_setting("shape-mode", SHAPE_MODES, 2);
        s.describe_last("How the shapes are rendered.");
        let side_color = s
            .default_group()
            .color_setting("side-color", SettingColor::new(255, 175, 25, 50));
        s.describe_last("The side color.");
        let line_color = s
            .default_group()
            .color_setting("line-color", SettingColor::new(255, 175, 25, 255));
        s.describe_last("The line color.");

        Self {
            base,
            height,
            connected,
            shape_mode,
            side_color,
            line_color,
            tunnels: Vec::new(),
            positions: HashSet::new(),
        }
    }

    pub fn tunnels(&self) -> &[(BlockPos, Exclude)] {
        &self.tunnels
    }

    /// `TunnelESP.canWalkOn`.
    fn can_walk_on(env: &dyn Env, pos: BlockPos) -> bool {
        match env.block(pos) {
            Some(b) => !b.is_air() && !is_fluid(&b) && has_collision(&b),
            None => false,
        }
    }

    /// `TunnelESP.canWalkThrough`.
    fn can_walk_through(env: &dyn Env, pos: BlockPos) -> bool {
        match env.block(pos) {
            Some(b) => b.is_air() || (!is_fluid(&b) && !has_collision(&b)),
            None => true,
        }
    }

    /// `TunnelESP.canWalkIn`: a floor, a two-block gap, and a ceiling.
    fn can_walk_in(env: &dyn Env, pos: BlockPos) -> bool {
        if !Self::can_walk_on(env, pos.offset(0, -1, 0)) {
            return false;
        }
        if !Self::can_walk_through(env, pos) {
            return false;
        }
        if Self::can_walk_through(env, pos.offset(0, 2, 0)) {
            return false;
        }
        Self::can_walk_through(env, pos.offset(0, 1, 0))
    }

    /// `TunnelESP.getTunnelSide`.
    fn tunnel_side(env: &dyn Env, pos: BlockPos) -> TunnelSide {
        if Self::can_walk_in(env, pos) {
            return TunnelSide::Walkable;
        }
        if !Self::can_walk_through(env, pos) && !Self::can_walk_through(env, pos.offset(0, 1, 0)) {
            return TunnelSide::FullyBlocked;
        }
        TunnelSide::PartiallyBlocked
    }

    /// `TunnelESP.isTunnel`: two walkable sides opposite each other, the other axis
    /// fully blocked.
    fn is_tunnel(env: &dyn Env, pos: BlockPos) -> bool {
        if !Self::can_walk_in(env, pos) {
            return false;
        }
        let s1 = Self::tunnel_side(env, pos.offset(1, 0, 0));
        let s2 = Self::tunnel_side(env, pos.offset(-1, 0, 0));
        let s3 = Self::tunnel_side(env, pos.offset(0, 0, 1));
        let s4 = Self::tunnel_side(env, pos.offset(0, 0, -1));
        for s in [s1, s2, s3, s4] {
            if s == TunnelSide::PartiallyBlocked {
                return false;
            }
        }
        (s1 == TunnelSide::Walkable
            && s2 == TunnelSide::Walkable
            && s3 == TunnelSide::FullyBlocked
            && s4 == TunnelSide::FullyBlocked)
            || (s1 == TunnelSide::FullyBlocked
                && s2 == TunnelSide::FullyBlocked
                && s3 == TunnelSide::Walkable
                && s4 == TunnelSide::Walkable)
    }

    /// `TunnelESP.searchChunk` + `onTick`: collect, then drop 1-block tunnels.
    pub fn scan(&mut self, env: &mut dyn Env) {
        self.positions.clear();
        let Some(player) = env.player() else { return };
        let origin = player.pos.to_block_pos();
        for x in (origin.x - SCAN_RADIUS)..=(origin.x + SCAN_RADIUS) {
            for z in (origin.z - SCAN_RADIUS)..=(origin.z + SCAN_RADIUS) {
                for y in (origin.y - SCAN_HEIGHT)..=(origin.y + SCAN_HEIGHT) {
                    let pos = BlockPos::new(x, y, z);
                    if Self::is_tunnel(env, pos) {
                        self.positions.insert(pos);
                    }
                }
            }
        }

        let mut tunnels = Vec::new();
        for pos in &self.positions {
            let mut exclude = Exclude::NONE;
            let mut has_neighbour = false;
            for dir in [Dir::East, Dir::West, Dir::North, Dir::South] {
                let (dx, _, dz) = dir.delta();
                let neighbour = pos.offset(dx, 0, dz);
                if self.positions.contains(&neighbour) {
                    has_neighbour = true;
                    if self.connected.get() {
                        exclude = exclude.merge(Exclude::of(dir));
                    }
                }
            }
            // Java drops tunnels that are one block long.
            if has_neighbour {
                tunnels.push((*pos, exclude));
            }
        }
        tunnels.sort_by_key(|(p, _)| (p.y, p.x, p.z));
        self.tunnels = tunnels;
    }

    fn render_tunnel(&self, r: &mut dyn Renderer, pos: BlockPos, exclude: Exclude) {
        let min = Vec3d::new(pos.x as f64, pos.y as f64, pos.z as f64);
        let max = Vec3d::new(
            pos.x as f64 + 1.0,
            pos.y as f64 + self.height.get(),
            pos.z as f64 + 1.0,
        );
        draw_box(
            r,
            min,
            max,
            self.side_color.color().argb(),
            self.line_color.color().argb(),
            draw_sides(&self.shape_mode),
            draw_lines(&self.shape_mode),
            exclude,
        );
    }
}

impl Default for TunnelESP {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for TunnelESP {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.tunnels.clear();
        self.positions.clear();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.scan(env);
    }

    fn on_render_world(&mut self, _env: &mut dyn Env, r: &mut dyn Renderer) {
        let tunnels = self.tunnels.clone();
        for (pos, exclude) in &tunnels {
            self.render_tunnel(r, *pos, *exclude);
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.tunnels.len().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(TunnelESP::new()));
}

fn is_fluid(block: &BlockInfo) -> bool {
    block.is("minecraft:water")
        || block.is("minecraft:lava")
        || block.is("minecraft:bubble_column")
        || block.is("minecraft:flowing_water")
        || block.is("minecraft:flowing_lava")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    /// A 2-high corridor running along X at y 64/65, z 0, from x -3 to 3.
    fn corridor(env: &mut TestEnv) {
        for x in -4..=4 {
            for z in -1..=1 {
                env.push_block(BlockPos::new(x, 63, z), "minecraft:stone"); // floor
                env.push_block(BlockPos::new(x, 66, z), "minecraft:stone"); // ceiling
            }
            env.push_block(BlockPos::new(x, 64, -1), "minecraft:stone"); // walls
            env.push_block(BlockPos::new(x, 65, -1), "minecraft:stone");
            env.push_block(BlockPos::new(x, 64, 1), "minecraft:stone");
            env.push_block(BlockPos::new(x, 65, 1), "minecraft:stone");
        }
        // The two ends are walls as well.
        for y in 64..=65 {
            env.push_block(BlockPos::new(-4, y, 0), "minecraft:stone");
            env.push_block(BlockPos::new(4, y, 0), "minecraft:stone");
        }
    }

    #[test]
    fn settings_match_the_java_table() {
        let m = TunnelESP::new();
        for name in [
            "height",
            "connected",
            "shape-mode",
            "side-color",
            "line-color",
        ] {
            assert!(
                m.base.settings.get(name).is_some(),
                "setting {name} missing"
            );
        }
        assert_eq!(m.base.settings.groups.len(), 1);
        assert_eq!(m.height.get(), 0.1);
        assert!(m.connected.get());
    }

    #[test]
    fn a_corridor_is_a_tunnel_and_renders() {
        let mut m = TunnelESP::new();
        let mut env = TestEnv::in_world();
        corridor(&mut env);
        m.on_tick(&mut env);

        assert!(
            m.tunnels()
                .iter()
                .any(|(p, _)| *p == BlockPos::new(0, 64, 0)),
            "the corridor middle should be a tunnel: {:?}",
            m.tunnels()
        );
        let (_, exclude) = m
            .tunnels()
            .iter()
            .find(|(p, _)| *p == BlockPos::new(0, 64, 0))
            .copied()
            .unwrap();
        assert_eq!(
            exclude,
            Exclude {
                east: true,
                west: true,
                ..Exclude::NONE
            },
            "connected stitches the two running sides"
        );

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0 && r.lines > 0);
        assert_eq!(m.info_string(), Some(m.tunnels().len().to_string()));
    }

    #[test]
    fn open_air_is_not_a_tunnel() {
        let mut m = TunnelESP::new();
        let mut env = TestEnv::in_world();
        // A floor but no ceiling: not a tunnel.
        for x in -2..=2 {
            for z in -2..=2 {
                env.push_block(BlockPos::new(x, 63, z), "minecraft:stone");
            }
        }
        m.on_tick(&mut env);
        assert!(m.tunnels().is_empty());
    }

    #[test]
    fn one_block_long_tunnels_are_dropped() {
        let mut m = TunnelESP::new();
        let mut env = TestEnv::in_world();
        // Floor, ceiling and four walls around a single 1x1 cell at (0,64,0).
        env.push_block(BlockPos::new(0, 63, 0), "minecraft:stone");
        env.push_block(BlockPos::new(0, 66, 0), "minecraft:stone");
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            env.push_block(BlockPos::new(dx, 64, dz), "minecraft:stone");
            env.push_block(BlockPos::new(dx, 65, dz), "minecraft:stone");
        }
        m.on_tick(&mut env);
        assert!(
            !m.tunnels()
                .iter()
                .any(|(p, _)| *p == BlockPos::new(0, 64, 0)),
            "single cells are filtered out"
        );
    }
}

//! `systems.modules.render.VoidESP` -?renders holes in the bedrock layer.
//!
//! Ported from `render/VoidESP.java`: the Java module walks the bedrock layer of
//! every loaded chunk looking for columns that are *not* bedrock (or, with
//! `air-only`, columns that are not air), plus the nether roof when the setting is
//! on. The port scans the player's radius with `env.block`.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::Env;
use crate::meteor_core::env::Renderer;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, EnumSet, IntSet};

use super::box_draw::{draw_box, Exclude};
use super::esp::{draw_lines, draw_sides, SHAPE_MODES};

/// One hole in the void floor: the Java `Void` record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoidHole {
    pub pos: BlockPos,
    pub exclude: Exclude,
    pub nether: bool,
}

pub struct VoidESP {
    base: ModuleBase,
    // General
    air_only: BoolSet,
    horizontal_radius: IntSet,
    hole_height: IntSet,
    nether_roof: BoolSet,
    // Render
    shape_mode: EnumSet,
    fill_color: ColorSet,
    line_color: ColorSet,
    // Runtime
    holes: Vec<VoidHole>,
}

impl VoidESP {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "void-esp",
            "Renders holes in bedrock layers that lead to the void.",
        );
        let s = base.settings_mut();
        let air_only = s.default_group().bool_setting("air-only", false);
        s.describe_last("Checks bedrock only for air blocks.");
        let horizontal_radius = s
            .default_group()
            .int_setting("horizontal-radius", 64, 0, 256);
        s.describe_last("Horizontal radius in which to search for holes.");
        let hole_height = s.default_group().int_setting("hole-height", 1, 1, 5);
        s.describe_last("The minimum hole height to be rendered.");
        let nether_roof = s.default_group().bool_setting("nether-roof", true);
        s.describe_last("Check for holes in nether roof.");

        s.create_group("Render");
        let shape_mode = s
            .groups
            .last_mut()
            .unwrap()
            .enum_setting("shape-mode", SHAPE_MODES, 2);
        s.describe_last("How the shapes are rendered.");
        let fill_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("fill-color", SettingColor::new(225, 25, 25, 50));
        s.describe_last("The color that fills holes in the void.");
        let line_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("line-color", SettingColor::new(225, 25, 25, 255));
        s.describe_last("The color to draw lines of holes to the void.");

        Self {
            base,
            air_only,
            horizontal_radius,
            hole_height,
            nether_roof,
            shape_mode,
            fill_color,
            line_color,
            holes: Vec::new(),
        }
    }

    pub fn holes(&self) -> &[VoidHole] {
        &self.holes
    }

    /// `VoidESP.isBlockWrong(BlockPos)`.
    fn is_block_wrong(&self, env: &dyn Env, pos: BlockPos) -> bool {
        let Some(block) = env.block(pos) else {
            // PORT GAP: `Env::block` is empty in the live Horizon env, so the
            // bedrock layer cannot be read there yet.
            return true;
        };
        if self.air_only.get() {
            !block.is_air()
        } else {
            // A hole in the bedrock layer is any column that is not bedrock.
            block.is("minecraft:bedrock")
        }
    }

    /// `VoidESP.isHole(BlockPos, boolean nether)`.
    fn is_hole(&self, env: &dyn Env, pos: BlockPos, nether: bool) -> bool {
        let (base_y, min_y) = match env.world() {
            Some(w) => (w.min_y, w.min_y),
            None => (-64, -64),
        };
        for i in 0..self.hole_height.get() {
            let y = if nether { 127 - i } else { base_y + i };
            if self.is_block_wrong(env, BlockPos::new(pos.x, y, pos.z)) {
                return false;
            }
        }
        let _ = min_y;
        true
    }

    /// `VoidESP.onTick`.
    pub fn scan(&mut self, env: &mut dyn Env) {
        self.holes.clear();
        let (dimension, min_y) = match env.world() {
            Some(w) => (w.dimension.clone(), w.min_y),
            None => (String::new(), -64),
        };
        if dimension.contains("the_end") {
            return;
        }
        let Some(player) = env.player() else { return };
        let origin = player.pos.to_block_pos();
        let radius = self.horizontal_radius.get();
        let nether = dimension.contains("the_nether");

        for x in (origin.x - radius)..=(origin.x + radius) {
            for z in (origin.z - radius)..=(origin.z + radius) {
                let bottom = BlockPos::new(x, min_y, z);
                if self.is_hole(env, bottom, false) {
                    self.holes.push(self.make_hole(env, bottom, false));
                }
                // Check for the nether roof.
                if self.nether_roof.get() && nether {
                    let roof = BlockPos::new(x, 127, z);
                    if self.is_hole(env, roof, true) {
                        self.holes.push(self.make_hole(env, roof, true));
                    }
                }
            }
        }
    }

    /// `Void.set(BlockPos, nether)`: byte up the sides whose neighbour is a hole.
    fn make_hole(&self, env: &dyn Env, pos: BlockPos, nether: bool) -> VoidHole {
        let mut exclude = Exclude::NONE;
        for side in [Dir::East, Dir::North, Dir::South, Dir::West] {
            let (dx, _, dz) = side.delta();
            let neighbour = BlockPos::new(pos.x + dx, pos.y, pos.z + dz);
            if self.is_hole(env, neighbour, nether) {
                exclude = exclude.merge(Exclude::of(side));
            }
        }
        VoidHole {
            pos,
            exclude,
            nether,
        }
    }

    fn render_hole(&self, r: &mut dyn Renderer, hole: &VoidHole) {
        let min = Vec3d::new(hole.pos.x as f64, hole.pos.y as f64, hole.pos.z as f64);
        let max = Vec3d::new(
            hole.pos.x as f64 + 1.0,
            hole.pos.y as f64 + 1.0,
            hole.pos.z as f64 + 1.0,
        );
        draw_box(
            r,
            min,
            max,
            self.fill_color.color().argb(),
            self.line_color.color().argb(),
            draw_sides(&self.shape_mode),
            draw_lines(&self.shape_mode),
            hole.exclude,
        );
    }
}

impl Default for VoidESP {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for VoidESP {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.holes.clear();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.scan(env);
    }

    fn on_render_world(&mut self, _env: &mut dyn Env, r: &mut dyn Renderer) {
        let holes = self.holes.clone();
        for hole in &holes {
            self.render_hole(r, hole);
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.holes.len().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(VoidESP::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn settings_match_the_java_table() {
        let m = VoidESP::new();
        for name in [
            "air-only",
            "horizontal-radius",
            "hole-height",
            "nether-roof",
            "shape-mode",
            "fill-color",
            "line-color",
        ] {
            assert!(
                m.base.settings.get(name).is_some(),
                "setting {name} missing"
            );
        }
        assert_eq!(m.base.settings.groups.len(), 2);
        assert_eq!(m.base.settings.groups[1].name, "Render");
        assert_eq!(m.horizontal_radius.get(), 64);
        assert_eq!(m.hole_height.get(), 1);
        assert!(m.nether_roof.get());
    }

    #[test]
    fn bedrock_floor_holes_are_found_and_rendered() {
        let mut m = VoidESP::new();
        m.horizontal_radius.set(2);
        let mut env = TestEnv::in_world();
        let min_y = env.world.as_ref().unwrap().min_y;
        for x in -2..=2 {
            for z in -2..=2 {
                env.push_block(BlockPos::new(x, min_y, z), "minecraft:bedrock");
            }
        }
        // One hole punched in the middle of the floor.
        env.push_block(BlockPos::new(0, min_y, 0), "minecraft:air");

        m.on_tick(&mut env);
        assert!(
            m.holes()
                .iter()
                .any(|h| h.pos == BlockPos::new(0, min_y, 0)),
            "the air column should be a void hole: {:?}",
            m.holes()
        );
        // Every bedrock column is "wrong" (bedrock is what a hole is missing),
        // so the punched air column is the only hole in the patch.
        let centre = m
            .holes()
            .iter()
            .find(|h| h.pos == BlockPos::new(0, min_y, 0))
            .copied()
            .unwrap();
        assert_eq!(
            centre.exclude,
            Exclude::NONE,
            "no neighbouring hole to stitch to"
        );

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0 && r.lines > 0);
        assert_eq!(m.info_string(), Some(m.holes().len().to_string()));
    }

    #[test]
    fn air_only_inverts_the_test() {
        let mut m = VoidESP::new();
        m.horizontal_radius.set(1);
        let mut env = TestEnv::in_world();
        let min_y = env.world.as_ref().unwrap().min_y;
        // A stone floor: not bedrock, so it counts as a hole in the bedrock layer.
        for x in -1..=1 {
            for z in -1..=1 {
                env.push_block(BlockPos::new(x, min_y, z), "minecraft:stone");
            }
        }
        m.on_tick(&mut env);
        assert!(
            m.holes()
                .iter()
                .any(|h| h.pos == BlockPos::new(0, min_y, 0)),
            "stone is not bedrock"
        );
        m.air_only.set(true);
        m.on_tick(&mut env);
        assert!(
            !m.holes()
                .iter()
                .any(|h| h.pos == BlockPos::new(0, min_y, 0)),
            "with air-only on only air counts"
        );
    }

    #[test]
    fn the_end_skips_the_scan() {
        let mut m = VoidESP::new();
        let mut env = TestEnv::in_world();
        if let Some(w) = env.world.as_mut() {
            w.dimension = "minecraft:the_end".into();
        }
        m.on_tick(&mut env);
        assert!(m.holes().is_empty());
    }
}

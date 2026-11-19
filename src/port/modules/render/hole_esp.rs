//! `systems.modules.render.HoleESP` -?displays holes you take less damage in.
//!
//! Ported from `render/HoleESP.java`: a radius scan that classifies every 1x1
//! (and optionally 2x1) air pocket by whether its five (or eight) sides are
//! bedrock, obsidian or a mix, then draws the Java module's line/quad shell.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::{Color, SettingColor};
use crate::meteor_core::env::{BlockInfo, Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, IntSet};

use super::esp::{draw_lines, draw_sides, SHAPE_MODES};

/// `HoleESP.Hole.Type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoleKind {
    Bedrock,
    Obsidian,
    Mixed,
}

/// One matched hole: its bottom block position, kind and the side towards the
/// second half of a double hole (`Hole.exclude`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hole {
    pub pos: BlockPos,
    pub kind: HoleKind,
    pub exclude: Option<Dir>,
}

/// How a surrounding block counts towards the bedrock/obsidian tally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Bedrock,
    Obsidian,
    Other,
}

pub struct HoleESP {
    base: ModuleBase,
    // General
    horizontal_radius: IntSet,
    vertical_radius: IntSet,
    min_height: IntSet,
    doubles: BoolSet,
    ignore_own: BoolSet,
    webs: BoolSet,
    // Render
    shape_mode: EnumSet,
    height: DoubleSet,
    top_quad: BoolSet,
    bottom_quad: BoolSet,
    bedrock_top: ColorSet,
    bedrock_bottom: ColorSet,
    obsidian_top: ColorSet,
    obsidian_bottom: ColorSet,
    mixed_top: ColorSet,
    mixed_bottom: ColorSet,
    // Runtime
    holes: Vec<Hole>,
}

impl HoleESP {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "hole-esp",
            "Displays holes that you will take less damage in.",
        );
        let s = base.settings_mut();
        let horizontal_radius = s
            .default_group()
            .int_setting("horizontal-radius", 10, 0, 32);
        s.describe_last("Horizontal radius in which to search for holes.");
        let vertical_radius = s.default_group().int_setting("vertical-radius", 5, 0, 32);
        s.describe_last("Vertical radius in which to search for holes.");
        let min_height = s.default_group().int_setting("min-height", 3, 1, 32);
        s.describe_last("Minimum hole height required to be rendered.");
        let doubles = s.default_group().bool_setting("doubles", true);
        s.describe_last("Highlights double holes that can be stood across.");
        let ignore_own = s.default_group().bool_setting("ignore-own", false);
        s.describe_last("Ignores rendering the hole you are currently standing in.");
        let webs = s.default_group().bool_setting("webs", false);
        s.describe_last("Whether to show holes that have webs inside of them.");

        s.create_group("Render");
        let shape_mode = s
            .groups
            .last_mut()
            .unwrap()
            .enum_setting("shape-mode", SHAPE_MODES, 2);
        s.describe_last("How the shapes are rendered.");
        let height = s
            .groups
            .last_mut()
            .unwrap()
            .double_setting("height", 0.2, 0.0, 1.0, 2);
        s.describe_last("The height of rendering.");
        let top_quad = s.groups.last_mut().unwrap().bool_setting("top-quad", true);
        s.describe_last("Whether to render a quad at the top of the hole.");
        let bottom_quad = s
            .groups
            .last_mut()
            .unwrap()
            .bool_setting("bottom-quad", false);
        s.describe_last("Whether to render a quad at the bottom of the hole.");

        let bedrock_top = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("bedrock-top", SettingColor::new(100, 255, 0, 200));
        s.describe_last("The top color for holes that are completely bedrock.");
        let bedrock_bottom = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("bedrock-bottom", SettingColor::new(100, 255, 0, 0));
        s.describe_last("The bottom color for holes that are completely bedrock.");
        let obsidian_top = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("obsidian-top", SettingColor::new(255, 0, 0, 200));
        s.describe_last("The top color for holes that are completely obsidian.");
        let obsidian_bottom = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("obsidian-bottom", SettingColor::new(255, 0, 0, 0));
        s.describe_last("The bottom color for holes that are completely obsidian.");
        let mixed_top = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("mixed-top", SettingColor::new(255, 127, 0, 200));
        s.describe_last("The top color for holes that have mixed bedrock and obsidian.");
        let mixed_bottom = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("mixed-bottom", SettingColor::new(255, 127, 0, 0));
        s.describe_last("The bottom color for holes that have mixed bedrock and obsidian.");

        Self {
            base,
            horizontal_radius,
            vertical_radius,
            min_height,
            doubles,
            ignore_own,
            webs,
            shape_mode,
            height,
            top_quad,
            bottom_quad,
            bedrock_top,
            bedrock_bottom,
            obsidian_top,
            obsidian_bottom,
            mixed_top,
            mixed_bottom,
            holes: Vec::new(),
        }
    }

    /// The holes the last tick found (the `holes` list in Java).
    pub fn holes(&self) -> &[Hole] {
        &self.holes
    }

    /// `HoleESP.validHole(BlockPos)`.
    pub fn valid_hole(&self, env: &dyn Env, pos: BlockPos) -> bool {
        if self.ignore_own.get() {
            if let Some(p) = env.player() {
                if p.pos.to_block_pos() == pos {
                    return false;
                }
            }
        }
        let Some(block) = env.block(pos) else {
            // PORT GAP: Horizon's `Env::block` answers `None` until the host
            // exposes block reads, so the scan finds nothing there yet.
            return false;
        };
        if !self.webs.get() && block.is("minecraft:cobweb") {
            return false;
        }
        if has_collision(&block) {
            return false;
        }
        for i in 0..self.min_height.get() {
            let Some(above) = env.block(pos.offset(0, i, 0)) else {
                return false;
            };
            if has_collision(&above) {
                return false;
            }
        }
        true
    }

    /// One `BlockIterator` visit: `HoleESP.onTick`'s probe body.
    fn probe(&self, env: &dyn Env, pos: BlockPos, out: &mut Vec<Hole>) {
        if !self.valid_hole(env, pos) {
            return;
        }

        let mut bedrock = 0;
        let mut obsidian = 0;
        let mut air: Option<Dir> = None;

        for direction in [Dir::Down, Dir::North, Dir::South, Dir::West, Dir::East] {
            let offset_pos = pos.offset_dir(direction);
            let Some(block) = env.block(offset_pos) else {
                return;
            };
            match classify(&block) {
                Side::Bedrock => bedrock += 1,
                Side::Obsidian => obsidian += 1,
                Side::Other => {
                    if direction == Dir::Down {
                        return;
                    }
                    if self.doubles.get() && air.is_none() && self.valid_hole(env, offset_pos) {
                        for dir in [Dir::Down, Dir::North, Dir::South, Dir::West, Dir::East] {
                            if dir == direction.opposite() || dir == Dir::Up {
                                continue;
                            }
                            let Some(block) = env.block(offset_pos.offset_dir(dir)) else {
                                return;
                            };
                            match classify(&block) {
                                Side::Bedrock => bedrock += 1,
                                Side::Obsidian => obsidian += 1,
                                Side::Other => return,
                            }
                        }
                        air = Some(direction);
                    } else {
                        return;
                    }
                }
            }
        }

        if obsidian + bedrock == 5 && air.is_none() {
            out.push(Hole {
                pos,
                kind: kind_of(bedrock, obsidian),
                exclude: None,
            });
        } else if obsidian + bedrock == 8 && self.doubles.get() && air.is_some() {
            out.push(Hole {
                pos,
                kind: kind_of(bedrock, obsidian),
                exclude: air,
            });
        }
    }

    /// `HoleESP.onTick`: rescan the player's radius.
    pub fn scan(&mut self, env: &mut dyn Env) {
        self.holes.clear();
        let Some(player) = env.player() else { return };
        let origin = player.pos.to_block_pos();
        let hr = self.horizontal_radius.get();
        let vr = self.vertical_radius.get();
        let mut found = Vec::new();
        for dx in -hr..=hr {
            for dz in -hr..=hr {
                for dy in -vr..=vr {
                    self.probe(env, origin.offset(dx, dy, dz), &mut found);
                }
            }
        }
        self.holes = found;
    }

    fn colors(&self, kind: HoleKind) -> (Color, Color) {
        match kind {
            HoleKind::Bedrock => (self.bedrock_top.color(), self.bedrock_bottom.color()),
            HoleKind::Obsidian => (self.obsidian_top.color(), self.obsidian_bottom.color()),
            HoleKind::Mixed => (self.mixed_top.color(), self.mixed_bottom.color()),
        }
    }

    /// `Hole.render(Renderer3D, ...)`.
    pub fn render_hole(&self, r: &mut dyn Renderer, hole: &Hole) {
        let (top, bottom) = self.colors(hole.kind);
        let h = self.height.get();
        let x = hole.pos.x as f64;
        let y = hole.pos.y as f64;
        let z = hole.pos.z as f64;
        let ex = hole.exclude;

        let not = |d: Dir| ex != Some(d);

        if draw_lines(&self.shape_mode) {
            // The four vertical edges, skipping the side shared with the second hole.
            if not(Dir::West) && not(Dir::North) {
                r.line(
                    Vec3d::new(x, y, z),
                    Vec3d::new(x, y + h, z),
                    top.argb(),
                    1.5,
                    true,
                );
            }
            if not(Dir::West) && not(Dir::South) {
                r.line(
                    Vec3d::new(x, y, z + 1.0),
                    Vec3d::new(x, y + h, z + 1.0),
                    top.argb(),
                    1.5,
                    true,
                );
            }
            if not(Dir::East) && not(Dir::North) {
                r.line(
                    Vec3d::new(x + 1.0, y, z),
                    Vec3d::new(x + 1.0, y + h, z),
                    top.argb(),
                    1.5,
                    true,
                );
            }
            if not(Dir::East) && not(Dir::South) {
                r.line(
                    Vec3d::new(x + 1.0, y, z + 1.0),
                    Vec3d::new(x + 1.0, y + h, z + 1.0),
                    top.argb(),
                    1.5,
                    true,
                );
            }

            // The bottom and top rings.
            if not(Dir::North) {
                r.line(
                    Vec3d::new(x, y, z),
                    Vec3d::new(x + 1.0, y, z),
                    bottom.argb(),
                    1.5,
                    true,
                );
                r.line(
                    Vec3d::new(x, y + h, z),
                    Vec3d::new(x + 1.0, y + h, z),
                    top.argb(),
                    1.5,
                    true,
                );
            }
            if not(Dir::South) {
                r.line(
                    Vec3d::new(x, y, z + 1.0),
                    Vec3d::new(x + 1.0, y, z + 1.0),
                    bottom.argb(),
                    1.5,
                    true,
                );
                r.line(
                    Vec3d::new(x, y + h, z + 1.0),
                    Vec3d::new(x + 1.0, y + h, z + 1.0),
                    top.argb(),
                    1.5,
                    true,
                );
            }
            if not(Dir::West) {
                r.line(
                    Vec3d::new(x, y, z),
                    Vec3d::new(x, y, z + 1.0),
                    bottom.argb(),
                    1.5,
                    true,
                );
                r.line(
                    Vec3d::new(x, y + h, z),
                    Vec3d::new(x, y + h, z + 1.0),
                    top.argb(),
                    1.5,
                    true,
                );
            }
            if not(Dir::East) {
                r.line(
                    Vec3d::new(x + 1.0, y, z),
                    Vec3d::new(x + 1.0, y, z + 1.0),
                    bottom.argb(),
                    1.5,
                    true,
                );
                r.line(
                    Vec3d::new(x + 1.0, y + h, z),
                    Vec3d::new(x + 1.0, y + h, z + 1.0),
                    top.argb(),
                    1.5,
                    true,
                );
            }
        }

        if draw_sides(&self.shape_mode) {
            let top_half = top.with_alpha(top.a / 2);
            let bottom_half = bottom.with_alpha(bottom.a / 2);
            // The Java renderer draws a top/bottom quad plus four gradient side
            // quads; the port's renderer has no gradient, so the sides are one
            // box with the (halved) bottom colour and the caps keep their own.
            if not(Dir::Up) && self.top_quad.get() && top_half.a > 0 {
                r.filled_box(
                    Vec3d::new(x, y + h, z),
                    Vec3d::new(x + 1.0, y + h, z + 1.0),
                    top_half.argb(),
                    true,
                );
            }
            if not(Dir::Down) && self.bottom_quad.get() && bottom_half.a > 0 {
                r.filled_box(
                    Vec3d::new(x, y, z),
                    Vec3d::new(x + 1.0, y, z + 1.0),
                    bottom_half.argb(),
                    true,
                );
            }
            if bottom_half.a > 0 {
                r.filled_box(
                    Vec3d::new(x, y, z),
                    Vec3d::new(x + 1.0, y + h, z + 1.0),
                    bottom_half.argb(),
                    true,
                );
            }
        }
    }
}

impl Default for HoleESP {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for HoleESP {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.holes.clear();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.scan(env);
    }

    fn on_render_world(&mut self, _env: &mut dyn Env, r: &mut dyn Renderer) {
        for hole in &self.holes.clone() {
            self.render_hole(r, hole);
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.holes.len().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(HoleESP::new()));
}

fn kind_of(bedrock: i32, obsidian: i32) -> HoleKind {
    if obsidian == 5 || obsidian == 8 {
        HoleKind::Obsidian
    } else if bedrock == 5 || bedrock == 8 {
        HoleKind::Bedrock
    } else {
        HoleKind::Mixed
    }
}

/// `block.getExplosionResistance() >= 600 && breakable` / `hasCollision && !breakable`.
fn classify(block: &BlockInfo) -> Side {
    if is_unbreakable(block) && has_collision(block) {
        Side::Bedrock
    } else if is_blast_resistant(block) && !is_unbreakable(block) {
        Side::Obsidian
    } else {
        Side::Other
    }
}

/// `Block.defaultDestroyTime() < 0` -?the blocks that cannot be broken at all.
fn is_unbreakable(block: &BlockInfo) -> bool {
    const IDS: &[&str] = &[
        "minecraft:bedrock",
        "minecraft:barrier",
        "minecraft:light",
        "minecraft:command_block",
        "minecraft:chain_command_block",
        "minecraft:repeating_command_block",
        "minecraft:structure_block",
        "minecraft:jigsaw",
        "minecraft:end_portal",
        "minecraft:end_portal_frame",
        "minecraft:end_gateway",
        "minecraft:nether_portal",
    ];
    IDS.iter().any(|id| block.is(id))
}

/// `Block.getExplosionResistance() >= 600`.
fn is_blast_resistant(block: &BlockInfo) -> bool {
    const IDS: &[&str] = &[
        "minecraft:obsidian",
        "minecraft:crying_obsidian",
        "minecraft:netherite_block",
        "minecraft:ancient_debris",
        "minecraft:respawn_anchor",
        "minecraft:enchanting_table",
        "minecraft:anvil",
        "minecraft:chipped_anvil",
        "minecraft:damaged_anvil",
        "minecraft:ender_chest",
    ];
    IDS.iter().any(|id| block.is(id))
}

/// `BlockBehaviour.hasCollision` -?the port reads the state string, so the
/// non-solid states are listed instead of asked.
pub(crate) fn has_collision(block: &BlockInfo) -> bool {
    const NON_SOLID: &[&str] = &[
        "minecraft:air",
        "minecraft:cave_air",
        "minecraft:void_air",
        "minecraft:water",
        "minecraft:lava",
        "minecraft:fire",
        "minecraft:soul_fire",
        "minecraft:torch",
        "minecraft:wall_torch",
        "minecraft:soul_torch",
        "minecraft:soul_wall_torch",
        "minecraft:redstone_torch",
        "minecraft:redstone_wall_torch",
        "minecraft:redstone_wire",
        "minecraft:tripwire",
        "minecraft:tripwire_hook",
        "minecraft:lever",
        "minecraft:vine",
        "minecraft:ladder",
        "minecraft:rail",
        "minecraft:powered_rail",
        "minecraft:detector_rail",
        "minecraft:activator_rail",
        "minecraft:grass",
        "minecraft:tall_grass",
        "minecraft:fern",
        "minecraft:large_fern",
        "minecraft:dead_bush",
        "minecraft:seagrass",
        "minecraft:tall_seagrass",
        "minecraft:kelp",
        "minecraft:kelp_plant",
        "minecraft:sugar_cane",
        "minecraft:crimson_roots",
        "minecraft:warped_roots",
        "minecraft:nether_sprouts",
        "minecraft:snow",
        "minecraft:light",
        "minecraft:structure_void",
        "minecraft:barrier",
        "minecraft:flower_pot",
        "minecraft:torchflower",
        "minecraft:potted_torchflower",
        "minecraft:cobweb",
        "minecraft:sweet_berry_bush",
        "minecraft:cave_vines",
        "minecraft:cave_vines_plant",
        "minecraft:weeping_vines",
        "minecraft:twisting_vines",
        "minecraft:spore_blossom",
        "minecraft:azalea",
        "minecraft:flowering_azalea",
        "minecraft:short_grass",
    ];
    if !NON_SOLID.iter().any(|id| block.is(id)) {
        // Flowers, saplings, buttons, pressure plates and signs are all
        // collision-free too; they carry a known id suffix.
        let short = short_id(block);
        if short.ends_with("_sign")
            || short.ends_with("_button")
            || short.ends_with("_pressure_plate")
            || short.ends_with("_sapling")
            || short.ends_with("_flower")
            || short.ends_with("_tulip")
            || short.ends_with("_coral")
            || short.ends_with("_banner")
            || short.ends_with("_carpet")
            || short.contains("candle")
        {
            return false;
        }
        return true;
    }
    false
}

/// The id without its namespace, for the suffix tests above.
pub(crate) fn short_id(block: &BlockInfo) -> &str {
    block.id.strip_prefix("minecraft:").unwrap_or(&block.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    /// A 1x1 obsidian hole at (3, 63, 3) with room to stand above it.
    fn obsidian_hole(env: &mut TestEnv) {
        for (dx, dy, dz) in [(0, -1, 0), (-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1)] {
            env.push_block(BlockPos::new(3 + dx, 63 + dy, 3 + dz), "minecraft:obsidian");
        }
        for y in 63..66 {
            env.push_block(BlockPos::new(3, y, 3), "minecraft:air");
        }
    }

    #[test]
    fn settings_match_the_java_table() {
        let m = HoleESP::new();
        for name in [
            "horizontal-radius",
            "vertical-radius",
            "min-height",
            "doubles",
            "ignore-own",
            "webs",
            "shape-mode",
            "height",
            "top-quad",
            "bottom-quad",
            "bedrock-top",
            "bedrock-bottom",
            "obsidian-top",
            "obsidian-bottom",
            "mixed-top",
            "mixed-bottom",
        ] {
            assert!(
                m.base.settings.get(name).is_some(),
                "setting {name} missing"
            );
        }
        assert_eq!(m.base.settings.groups.len(), 2);
        assert_eq!(m.base.settings.groups[1].name, "Render");
        assert_eq!(m.horizontal_radius.get(), 10);
        assert_eq!(m.min_height.get(), 3);
        assert!(m.doubles.get());
        assert_eq!(m.height.get(), 0.2);
    }

    #[test]
    fn scan_finds_the_obsidian_hole_and_renders_it() {
        let mut m = HoleESP::new();
        let mut env = TestEnv::in_world();
        obsidian_hole(&mut env);
        m.on_tick(&mut env);

        let found = m
            .holes()
            .iter()
            .find(|h| h.pos == BlockPos::new(3, 63, 3))
            .copied();
        assert!(
            found.is_some(),
            "hole at (3,63,3) not found: {:?}",
            m.holes()
        );
        assert_eq!(found.unwrap().kind, HoleKind::Obsidian);
        assert_eq!(found.unwrap().exclude, None);

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.lines > 0, "the shell's edges should be drawn");
        assert!(r.boxes > 0, "the side fill should be drawn");
        assert_eq!(m.info_string(), Some(m.holes().len().to_string()));
    }

    #[test]
    fn a_non_blast_resistant_wall_is_not_a_hole() {
        let mut m = HoleESP::new();
        let mut env = TestEnv::in_world();
        for (dx, dy, dz) in [(0, -1, 0), (-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1)] {
            env.push_block(BlockPos::new(3 + dx, 63 + dy, 3 + dz), "minecraft:dirt");
        }
        m.on_tick(&mut env);
        assert!(m.holes().is_empty(), "dirt walls are not a city hole");
    }

    #[test]
    fn ignore_own_skips_the_standing_position() {
        let mut m = HoleESP::new();
        let mut env = TestEnv::in_world();
        obsidian_hole(&mut env);
        // Move the hole under the player's feet (0, 64, 0).
        for (dx, dy, dz) in [(0, -1, 0), (-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1)] {
            env.push_block(BlockPos::new(3 + dx, 63 + dy, 3 + dz), "minecraft:air");
        }
        for (dx, dy, dz) in [(0, -1, 0), (-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1)] {
            env.push_block(BlockPos::new(dx, 64 + dy, dz), "minecraft:obsidian");
        }
        // ignore-own defaults to false, so the hole is found first.
        m.on_tick(&mut env);
        assert!(m.holes().iter().any(|h| h.pos == BlockPos::new(0, 64, 0)));
        m.ignore_own.set(true);
        m.on_tick(&mut env);
        assert!(!m.holes().iter().any(|h| h.pos == BlockPos::new(0, 64, 0)));
    }
}

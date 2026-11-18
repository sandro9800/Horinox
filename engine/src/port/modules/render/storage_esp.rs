//! `systems.modules.render.StorageESP` -?renders storage blocks through walls.
//!
//! Ported from `render/StorageESP.java`. Java walks the loaded block entities of
//! the world; the port scans a bounded cube around the player with `env.block`
//! every 20 ticks and keeps the hits in a small cache, then draws them with the
//! Java colour/opacity/fade settings.

use std::collections::HashSet;

use crate::meteor_core::category::Category;
use crate::meteor_core::color::{Color, SettingColor};
use crate::meteor_core::env::{BlockInfo, Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, ListSet};

use super::box_draw::{draw_box, Exclude};
use super::esp::{draw_lines, draw_sides, SHAPE_MODES};

/// The block-entity types `StorageBlockListSetting.STORAGE_BLOCKS` lists.
pub const STORAGE_BLOCKS: &[&str] = &[
    "minecraft:barrel",
    "minecraft:blast_furnace",
    "minecraft:brewing_stand",
    "minecraft:campfire",
    "minecraft:chest",
    "minecraft:chiseled_bookshelf",
    "minecraft:crafter",
    "minecraft:dispenser",
    "minecraft:decorated_pot",
    "minecraft:dropper",
    "minecraft:ender_chest",
    "minecraft:furnace",
    "minecraft:hopper",
    "minecraft:shulker_box",
    "minecraft:smoker",
    "minecraft:trapped_chest",
];

/// The colour bucket `getBlockEntityColor` picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageKind {
    Chest,
    TrappedChest,
    Barrel,
    Shulker,
    EnderChest,
    Other,
}

/// `(block id, block-entity type id, kind)` for every storage block the port
/// matches; the shulker boxes all map to the one `minecraft:shulker_box` type.
fn storage_of(block: &BlockInfo) -> Option<(&'static str, StorageKind)> {
    let short = block.id.strip_prefix("minecraft:").unwrap_or(&block.id);
    let hit = match short {
        "chest" => ("minecraft:chest", StorageKind::Chest),
        "trapped_chest" => ("minecraft:trapped_chest", StorageKind::TrappedChest),
        "barrel" => ("minecraft:barrel", StorageKind::Barrel),
        "ender_chest" => ("minecraft:ender_chest", StorageKind::EnderChest),
        "furnace" => ("minecraft:furnace", StorageKind::Other),
        "blast_furnace" => ("minecraft:blast_furnace", StorageKind::Other),
        "smoker" => ("minecraft:smoker", StorageKind::Other),
        "dispenser" => ("minecraft:dispenser", StorageKind::Other),
        "dropper" => ("minecraft:dropper", StorageKind::Other),
        "hopper" => ("minecraft:hopper", StorageKind::Other),
        "brewing_stand" => ("minecraft:brewing_stand", StorageKind::Other),
        "chiseled_bookshelf" => ("minecraft:chiseled_bookshelf", StorageKind::Other),
        "crafter" => ("minecraft:crafter", StorageKind::Other),
        "decorated_pot" => ("minecraft:decorated_pot", StorageKind::Other),
        "campfire" | "soul_campfire" => ("minecraft:campfire", StorageKind::Other),
        other if other.ends_with("shulker_box") => ("minecraft:shulker_box", StorageKind::Shulker),
        _ => return None,
    };
    Some(hit)
}

/// How far around the player the port scans (Java reads loaded block entities).
pub const SCAN_RADIUS: i32 = 24;
/// Vertical reach of the port's scan.
pub const SCAN_HEIGHT: i32 = 12;

pub struct StorageESP {
    base: ModuleBase,
    // General
    mode: EnumSet,
    storage_blocks: ListSet,
    tracers: BoolSet,
    shape_mode: EnumSet,
    fill_opacity: IntSet,
    width: IntSet,
    glow_multiplier: DoubleSet,
    chest: ColorSet,
    trapped_chest: ColorSet,
    barrel: ColorSet,
    shulker: ColorSet,
    ender_chest: ColorSet,
    other: ColorSet,
    fade_distance: DoubleSet,
    // Opened Rendering
    hide_opened: BoolSet,
    opened_color: ColorSet,
    // Runtime
    cache: Vec<(BlockPos, &'static str, StorageKind)>,
    /// `interactedBlocks` in Java, filled by `mark_opened` because the port has no
    /// block-interaction hook.
    opened: HashSet<BlockPos>,
    count: usize,
}

impl StorageESP {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "storage-esp",
            "Renders all specified storage blocks.",
        );
        let s = base.settings_mut();
        let mode = s
            .default_group()
            .enum_setting("mode", &["box", "shader"], 1);
        s.describe_last("Rendering mode.");
        let storage_blocks = s
            .default_group()
            .list_setting("storage-blocks", STORAGE_BLOCKS);
        s.describe_last("Select the storage blocks to display.");
        let tracers = s.default_group().bool_setting("tracers", false);
        s.describe_last("Draws tracers to storage blocks.");
        let shape_mode = s.default_group().enum_setting("shape-mode", SHAPE_MODES, 2);
        s.describe_last("How the shapes are rendered.");
        let fill_opacity = s.default_group().int_setting("fill-opacity", 50, 0, 255);
        s.describe_last("The opacity of the shape fill.");
        let sm = shape_mode.clone();
        s.set_visible_last(std::sync::Arc::new(move || !sm.is("lines")));
        let width = s.default_group().int_setting("width", 1, 1, 10);
        s.describe_last("The width of the shader outline.");
        let mo = mode.clone();
        s.set_visible_last(std::sync::Arc::new(move || mo.is("shader")));
        let glow_multiplier =
            s.default_group()
                .double_setting("glow-multiplier", 3.5, 0.0, 10.0, 3);
        s.describe_last("Multiplier for glow effect");
        let mo = mode.clone();
        s.set_visible_last(std::sync::Arc::new(move || mo.is("shader")));

        let chest = s
            .default_group()
            .color_setting("chest", SettingColor::new(255, 160, 0, 255));
        s.describe_last("The color of chests.");
        let trapped_chest = s
            .default_group()
            .color_setting("trapped-chest", SettingColor::new(255, 0, 0, 255));
        s.describe_last("The color of trapped chests.");
        let barrel = s
            .default_group()
            .color_setting("barrel", SettingColor::new(255, 160, 0, 255));
        s.describe_last("The color of barrels.");
        let shulker = s
            .default_group()
            .color_setting("shulker", SettingColor::new(255, 160, 0, 255));
        s.describe_last("The color of Shulker Boxes.");
        let ender_chest = s
            .default_group()
            .color_setting("ender-chest", SettingColor::new(120, 0, 255, 255));
        s.describe_last("The color of Ender Chests.");
        let other = s
            .default_group()
            .color_setting("other", SettingColor::new(140, 140, 140, 255));
        s.describe_last("The color of furnaces, dispensers, droppers and hoppers.");
        let fade_distance = s
            .default_group()
            .double_setting("fade-distance", 6.0, 0.0, 12.0, 1);
        s.describe_last("The distance at which the color will fade.");

        s.create_group("Opened Rendering");
        let hide_opened = s
            .groups
            .last_mut()
            .unwrap()
            .bool_setting("hide-opened", false);
        s.describe_last("Hides opened containers.");
        let opened_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("opened-color", SettingColor::new(203, 90, 203, 0));
        s.describe_last(
            "Optional setting to change colors of opened chests, as opposed to not rendering. Disabled at zero opacity.",
        );

        Self {
            base,
            mode,
            storage_blocks,
            tracers,
            shape_mode,
            fill_opacity,
            width,
            glow_multiplier,
            chest,
            trapped_chest,
            barrel,
            shulker,
            ender_chest,
            other,
            fade_distance,
            hide_opened,
            opened_color,
            cache: Vec::new(),
            opened: HashSet::new(),
            count: 0,
        }
    }

    /// `StorageESP.getWidget()`'s "clear rendering cache" button.
    pub fn clear_opened(&mut self) {
        self.opened.clear();
    }

    /// Marks a container as opened (`InteractBlockEvent` in Java).
    pub fn mark_opened(&mut self, pos: BlockPos) {
        self.opened.insert(pos);
    }

    /// The blocks the last scan found.
    pub fn cached(&self) -> &[(BlockPos, &'static str, StorageKind)] {
        &self.cache
    }

    /// `Utils.blockEntities()` stand-in: rescan the player's cube.
    pub fn scan(&mut self, env: &mut dyn Env) {
        self.cache.clear();
        // PORT GAP: no block-entity or block-interaction hook -?the scan is a
        // bounded cube and `opened` is only filled through `mark_opened`.
        let Some(player) = env.player() else { return };
        let origin = player.pos.to_block_pos();
        for x in (origin.x - SCAN_RADIUS)..=(origin.x + SCAN_RADIUS) {
            for z in (origin.z - SCAN_RADIUS)..=(origin.z + SCAN_RADIUS) {
                for y in (origin.y - SCAN_HEIGHT)..=(origin.y + SCAN_HEIGHT) {
                    let pos = BlockPos::new(x, y, z);
                    let Some(block) = env.block(pos) else {
                        continue;
                    };
                    if block.is_air() {
                        continue;
                    }
                    let Some((type_id, kind)) = storage_of(&block) else {
                        continue;
                    };
                    if !self.storage_blocks.contains(type_id) {
                        continue;
                    }
                    self.cache.push((pos, type_id, kind));
                }
            }
        }
    }

    fn color_of(&self, kind: StorageKind) -> Color {
        match kind {
            StorageKind::Chest => self.chest.color(),
            StorageKind::TrappedChest => self.trapped_chest.color(),
            StorageKind::Barrel => self.barrel.color(),
            StorageKind::Shulker => self.shulker.color(),
            StorageKind::EnderChest => self.ender_chest.color(),
            StorageKind::Other => self.other.color(),
        }
    }

    /// The face a double chest shares with its other half.
    fn connected_face(block: &BlockInfo) -> Option<Dir> {
        let chest_type = block.prop("type")?;
        if chest_type == "single" {
            return None;
        }
        let facing = match block.prop("facing")?.to_ascii_lowercase().as_str() {
            "north" => Dir::North,
            "south" => Dir::South,
            "west" => Dir::West,
            "east" => Dir::East,
            _ => return None,
        };
        // `ChestBlock.getConnectedDirection`.
        Some(if chest_type == "left" {
            counter_clockwise(facing)
        } else {
            clockwise(facing)
        })
    }

    fn render_block(&self, r: &mut dyn Renderer, env: &dyn Env, pos: BlockPos, kind: StorageKind) {
        let Some(block) = env.block(pos) else { return };
        let interacted = self.opened.contains(&pos);
        if interacted && self.hide_opened.get() {
            return;
        }
        let mut line = self.color_of(kind);
        let mut side = line.with_alpha(self.fill_opacity.get().clamp(0, 255) as u8);
        if interacted && self.opened_color.color().a > 0 {
            line = self.opened_color.color();
            side = line.with_alpha(self.fill_opacity.get().clamp(0, 255) as u8);
        }

        let centre = pos.center();
        let camera = env.player().map(|p| p.eye_pos()).unwrap_or(Vec3d::ZERO);
        let dist = camera.distance_sq(centre);
        let fade = self.fade_distance.get();
        let mut a = 1.0;
        if fade > 0.0 && dist <= fade * fade {
            a = dist / (fade * fade);
        }
        if a < 0.075 {
            return;
        }
        line = line.with_alpha((line.a as f64 * a) as u8);
        side = side.with_alpha((side.a as f64 * a) as u8);

        if self.tracers.get() {
            r.line(camera, centre, line.argb(), 1.5, true);
        }

        // The Java shader mode draws the block model; the port draws the box in
        // both modes.
        // PORT GAP: no post-process outline shader for `mode shader`.
        let _ = (&self.width, &self.glow_multiplier);

        let exclude = if matches!(kind, StorageKind::Chest | StorageKind::TrappedChest) {
            Self::connected_face(&block)
                .map(Exclude::of)
                .unwrap_or(Exclude::NONE)
        } else {
            Exclude::NONE
        };

        let mut min = Vec3d::new(pos.x as f64, pos.y as f64, pos.z as f64);
        let mut max = Vec3d::new(pos.x as f64 + 1.0, pos.y as f64 + 1.0, pos.z as f64 + 1.0);
        if matches!(kind, StorageKind::Chest | StorageKind::EnderChest) {
            // `renderBox` insets chests by 1/16 so they sit inside their block.
            let inset = 1.0 / 16.0;
            if !exclude.west {
                min.x += inset;
            }
            if !exclude.north {
                min.z += inset;
            }
            if !exclude.east {
                max.x -= inset;
            }
            max.y -= inset * 2.0;
            if !exclude.south {
                max.z -= inset;
            }
        }

        draw_box(
            r,
            min,
            max,
            side.argb(),
            line.argb(),
            draw_sides(&self.shape_mode),
            draw_lines(&self.shape_mode),
            exclude,
        );
    }
}

impl Default for StorageESP {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for StorageESP {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.cache.clear();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // Java reads the loaded block entities every frame; the port refreshes its
        // cache once a second to keep the block reads bounded.
        if self.cache.is_empty() || env.ticks() % 20 == 0 {
            self.scan(env);
        }
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        self.count = 0;
        let cache = self.cache.clone();
        for (pos, _type_id, kind) in &cache {
            self.render_block(r, env, *pos, *kind);
            self.count += 1;
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.count.to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(StorageESP::new()));
}

fn clockwise(d: Dir) -> Dir {
    match d {
        Dir::North => Dir::East,
        Dir::East => Dir::South,
        Dir::South => Dir::West,
        Dir::West => Dir::North,
        other => other,
    }
}

fn counter_clockwise(d: Dir) -> Dir {
    match d {
        Dir::North => Dir::West,
        Dir::West => Dir::South,
        Dir::South => Dir::East,
        Dir::East => Dir::North,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn settings_match_the_java_table() {
        let m = StorageESP::new();
        for name in [
            "mode",
            "storage-blocks",
            "tracers",
            "shape-mode",
            "fill-opacity",
            "width",
            "glow-multiplier",
            "chest",
            "trapped-chest",
            "barrel",
            "shulker",
            "ender-chest",
            "other",
            "fade-distance",
            "hide-opened",
            "opened-color",
        ] {
            assert!(
                m.base.settings.get(name).is_some(),
                "setting {name} missing"
            );
        }
        assert_eq!(m.base.settings.groups.len(), 2);
        assert_eq!(m.base.settings.groups[1].name, "Opened Rendering");
        assert!(m.mode.is("shader"), "Java default is Mode.Shader");
        assert_eq!(m.fill_opacity.get(), 50);
        assert_eq!(m.storage_blocks.get().len(), STORAGE_BLOCKS.len());
    }

    #[test]
    fn a_nearby_chest_is_scanned_and_drawn() {
        let mut m = StorageESP::new();
        m.mode.set_index(0); // Box
        let mut env = TestEnv::in_world();
        env.push_block(
            BlockPos::new(3, 65, 3),
            "minecraft:chest[facing=north,type=single]",
        );
        m.on_tick(&mut env);
        assert_eq!(
            m.cached().len(),
            1,
            "the chest should be in the cache: {:?}",
            m.cached()
        );

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(m.count, 1);
        assert_eq!(m.info_string().as_deref(), Some("1"));
        assert!(r.boxes > 0 && r.lines > 0);
    }

    #[test]
    fn the_block_list_gates_the_scan() {
        let mut m = StorageESP::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(2, 65, 2), "minecraft:barrel");
        m.storage_blocks.set(vec!["minecraft:chest".into()]);
        m.on_tick(&mut env);
        assert!(m.cached().is_empty(), "barrels are not in the list");

        m.storage_blocks.set(vec!["minecraft:barrel".into()]);
        m.on_tick(&mut env);
        assert_eq!(m.cached().len(), 1);
    }

    #[test]
    fn opened_containers_follow_the_settings() {
        let mut m = StorageESP::new();
        m.mode.set_index(0);
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(3, 65, 3);
        env.push_block(pos, "minecraft:chest[facing=north,type=single]");
        m.on_tick(&mut env);
        m.mark_opened(pos);

        m.hide_opened.set(true);
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.boxes, 0, "hide-opened drops it entirely");

        m.hide_opened.set(false);
        m.opened_color.set(Color::MAGENTA);
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0, "opened-color keeps it drawn");
    }

    #[test]
    fn double_chests_share_one_face() {
        let block = BlockInfo::new("minecraft:chest[facing=north,type=left]");
        assert_eq!(StorageESP::connected_face(&block), Some(Dir::West));
        let single = BlockInfo::new("minecraft:chest[facing=north,type=single]");
        assert_eq!(StorageESP::connected_face(&single), None);
    }
}

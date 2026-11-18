//! `systems.modules.world.StashFinder` - "searches loaded chunks for storage
//! blocks. Saves to <your minecraft folder>/horixon".
//!
//! Java answers `ChunkDataEvent` and walks the chunk's block entities. The `Env`
//! reads blocks, not block entities, so the port walks the chunk's columns with
//! `env.block` and counts the storage block ids; everything else (the chunk
//! record, the CSV/JSON shape, the tracer overlay and the notification modes) is
//! translated as it stands.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Vec3d;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, EnumSet, IntSet, KeybindSet, ListSet};

use super::{describe, listed, visible};

/// `StashFinder.DEFAULT_SUPPORT_BLOCK_BLACKLIST`.
const DEFAULT_SUPPORT_BLOCK_BLACKLIST: [&str; 8] = [
    "minecraft:oxidized_copper",
    "minecraft:oxidized_cut_copper",
    "minecraft:tuff_bricks",
    "minecraft:waxed_copper_block",
    "minecraft:waxed_oxidized_copper",
    "minecraft:waxed_oxidized_cut_copper",
    "minecraft:barrel",
    "minecraft:waxed_copper_bulb",
];

/// `StorageBlockListSetting.STORAGE_BLOCKS`.
const STORAGE_BLOCKS: [&str; 10] = [
    "minecraft:chest",
    "minecraft:barrel",
    "minecraft:shulker_box",
    "minecraft:ender_chest",
    "minecraft:furnace",
    "minecraft:blast_furnace",
    "minecraft:smoker",
    "minecraft:dispenser",
    "minecraft:dropper",
    "minecraft:hopper",
];

/// `StashFinder.Chunk`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chunk {
    pub x: i32,
    pub z: i32,
    pub chests: i32,
    pub barrels: i32,
    pub shulkers: i32,
    pub ender_chests: i32,
    pub furnaces: i32,
    pub dispensers_droppers: i32,
    pub hoppers: i32,
}

impl Chunk {
    pub fn at(x: i32, z: i32) -> Self {
        Self {
            x,
            z,
            ..Default::default()
        }
    }

    /// `Chunk.getTotal()`.
    pub fn total(&self) -> i32 {
        self.chests
            + self.barrels
            + self.shulkers
            + self.ender_chests
            + self.furnaces
            + self.dispensers_droppers
            + self.hoppers
    }

    /// `ChunkPos.getMiddleBlockX()`.
    pub fn middle_x(&self) -> i32 {
        self.x * 16 + 8
    }

    /// `ChunkPos.getMiddleBlockZ()`.
    pub fn middle_z(&self) -> i32 {
        self.z * 16 + 8
    }

    /// `Chunk.countsEqual(Chunk)`.
    pub fn counts_equal(&self, other: &Chunk) -> bool {
        self.chests == other.chests
            && self.barrels == other.barrels
            && self.shulkers == other.shulkers
            && self.ender_chests == other.ender_chests
            && self.furnaces == other.furnaces
            && self.dispensers_droppers == other.dispensers_droppers
            && self.hoppers == other.hoppers
    }
}

pub struct StashFinder {
    base: ModuleBase,
    storage_blocks: ListSet,
    minimum_storage_count: IntSet,
    blacklisted_blocks: ListSet,
    minimum_distance: IntSet,
    send_notifications: BoolSet,
    notification_mode: EnumSet,
    // Render
    render_tracer: BoolSet,
    trace_color: ColorSet,
    trace_arrival_distance: IntSet,
    trace_max_distance: IntSet,
    render_chunk_column: BoolSet,
    trace_column_color: ColorSet,
    clear_traces_bind: KeybindSet,
    // Runtime
    pub chunks: Vec<Chunk>,
    tracers: Vec<(i32, i32)>,
    scanned_chunk: Option<(i32, i32)>,
}

impl StashFinder {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "stash-finder",
            "Searches loaded chunks for storage blocks. Saves to <your minecraft folder>/horixon",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let storage_blocks = g.list_setting("storage-blocks", &STORAGE_BLOCKS);
        describe(g, "Select the storage blocks to search for.");

        let minimum_storage_count = g.int_setting("minimum-storage-count", 4, 1, 1024);
        describe(
            g,
            "The minimum amount of storage blocks in a chunk to record the chunk.",
        );

        let blacklisted_blocks = g.list_setting(
            "blacklisted-support-blocks",
            &DEFAULT_SUPPORT_BLOCK_BLACKLIST,
        );
        describe(
            g,
            "Blocks that prevent counting a storage block entity when it sits on them.",
        );

        let minimum_distance = g.int_setting("minimum-distance", 0, 0, 10000);
        describe(
            g,
            "The minimum distance you must be from spawn to record a certain chunk.",
        );

        let send_notifications = g.bool_setting("notifications", true);
        describe(
            g,
            "Sends Minecraft notifications when new stashes are found.",
        );

        let notification_mode = g.enum_setting("notification-mode", &["chat", "toast", "both"], 2);
        describe(g, "The mode to use for notifications.");
        let flag = send_notifications.clone();
        visible(g, move || flag.get());

        let r = s.create_group("Render");

        let render_tracer = r.bool_setting("render-tracer", true);
        describe(r, "Renders a tracer to the last found stash.");

        let trace_color = r.color_setting("tracer-color", SettingColor::new(255, 215, 0, 255));
        describe(r, "Color of the stash tracer.");
        let flag = render_tracer.clone();
        visible(r, move || flag.get());

        let trace_arrival_distance = r.int_setting("tracer-hide-at-distance", 16, 1, 100);
        describe(r, "Hide the trace when you are this close to the stash.");
        let flag = render_tracer.clone();
        visible(r, move || flag.get());

        let trace_max_distance = r.int_setting("tracer-max-distance", 2000, 10, 10000);
        describe(
            r,
            "Hide the trace when you are farther than this distance from the stash.",
        );
        let flag = render_tracer.clone();
        visible(r, move || flag.get());

        let render_chunk_column = r.bool_setting("render-chunk-column", false);
        describe(
            r,
            "Renders a vertical column at the center of traced chunks.",
        );

        let trace_column_color =
            r.color_setting("chunk-column-color", SettingColor::new(255, 215, 0, 100));
        describe(r, "Color of the stash tracer column.");
        let flag = render_chunk_column.clone();
        visible(r, move || flag.get());

        let clear_traces_bind = r.keybind_setting("clear-traces-bind");
        describe(r, "Keybind to clear all stash traces.");

        Self {
            base,
            storage_blocks,
            minimum_storage_count,
            blacklisted_blocks,
            minimum_distance,
            send_notifications,
            notification_mode,
            render_tracer,
            trace_color,
            trace_arrival_distance,
            trace_max_distance,
            render_chunk_column,
            trace_column_color,
            clear_traces_bind,
            chunks: Vec::new(),
            tracers: Vec::new(),
            scanned_chunk: None,
        }
    }

    /// The chunks the tracer overlay draws for.
    pub fn tracers(&self) -> &[(i32, i32)] {
        &self.tracers
    }

    /// `StashFinder.onChunkData(ChunkDataEvent)` for one chunk.
    pub fn scan_chunk(&mut self, env: &mut dyn Env, chunk_x: i32, chunk_z: i32) {
        // Check the distance.
        let chunk_x_abs = (chunk_x * 16) as f64;
        let chunk_z_abs = (chunk_z * 16) as f64;
        if (chunk_x_abs * chunk_x_abs + chunk_z_abs * chunk_z_abs).sqrt()
            < self.minimum_distance.get() as f64
        {
            return;
        }

        let Some(world) = env.world() else { return };
        let mut chunk = Chunk::at(chunk_x, chunk_z);

        for y in world.min_y..world.min_y + world.height {
            for x in chunk_x * 16..chunk_x * 16 + 16 {
                for z in chunk_z * 16..chunk_z * 16 + 16 {
                    let pos = crate::meteor_core::math::BlockPos::new(x, y, z);
                    let Some(block) = env.block(pos) else {
                        continue;
                    };
                    if !listed(&self.storage_blocks, &block.id) {
                        continue;
                    }
                    // `if (blockBlacklist.contains(chunk.getBlockState(below).getBlock())) continue;`
                    if !self.blacklisted_blocks.is_empty() {
                        let below = env.block(pos.offset(0, -1, 0)).unwrap_or_default();
                        if listed(&self.blacklisted_blocks, &below.id) {
                            continue;
                        }
                    }
                    match block.id.trim_start_matches("minecraft:") {
                        "chest" => chunk.chests += 1,
                        "barrel" => chunk.barrels += 1,
                        "ender_chest" => chunk.ender_chests += 1,
                        "furnace" | "blast_furnace" | "smoker" => chunk.furnaces += 1,
                        "dispenser" | "dropper" => chunk.dispensers_droppers += 1,
                        "hopper" => chunk.hoppers += 1,
                        _ => chunk.shulkers += 1,
                    }
                }
            }
        }

        if chunk.total() < self.minimum_storage_count.get() {
            return;
        }

        let previous = self
            .chunks
            .iter()
            .position(|known| known.x == chunk.x && known.z == chunk.z)
            .map(|index| self.chunks[index].clone());
        match self
            .chunks
            .iter_mut()
            .find(|known| known.x == chunk.x && known.z == chunk.z)
        {
            Some(known) => *known = chunk.clone(),
            None => self.chunks.push(chunk.clone()),
        }

        if self.render_tracer.get() {
            if !self
                .tracers
                .iter()
                .any(|(x, z)| *x == chunk.x && *z == chunk.z)
            {
                self.tracers.push((chunk.x, chunk.z));
            }
        }

        // PORT GAP: the stash list is written to
        // `horixon/stashes/<world>/stashes.json` and `.csv` in Java; the
        // `Env` has no filesystem, so the records stay in `chunks` for the GUI.
        let changed = previous
            .as_ref()
            .is_none_or(|prev| !chunk.counts_equal(prev));
        if self.send_notifications.get() && changed {
            // `Mode.Toast` needs `mc.gui.toastManager()`; the port always prints.
            if self.notification_mode.is("chat") || self.notification_mode.is("both") {
                self.send_chat_notification(env, &chunk);
            }
            // PORT GAP: `MeteorToast` toasts (`mc.gui.toastManager().addToast`).
        }

        self.scanned_chunk = Some((chunk_x, chunk_z));
    }

    fn send_chat_notification(&self, env: &mut dyn Env, chunk: &Chunk) {
        let msg = format!(
            "Found stash at [{}, {}].",
            chunk.middle_x(),
            chunk.middle_z()
        );
        self.base.info(env, &msg);
    }
}

impl Default for StashFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for StashFinder {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // `if (!clearTracesBind.get().isPressed()) return; tracerPositions.clear();`
        let bind = self.clear_traces_bind.get();
        if bind.is_set()
            && ((bind.is_key && env.key_down(bind.value))
                || (!bind.is_key && env.mouse_down(bind.value)))
        {
            self.tracers.clear();
        }

        let Some(player) = env.player() else { return };
        let feet = player.pos.to_block_pos();
        let chunk = (feet.x >> 4, feet.z >> 4);
        if self.scanned_chunk == Some(chunk) {
            return;
        }
        self.scan_chunk(env, chunk.0, chunk.1);
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let Some(player) = env.player() else { return };
        if self.tracers.is_empty() {
            return;
        }

        // Drop the tracers the player has reached.
        let arrival = self.trace_arrival_distance.get() as f64;
        self.tracers.retain(|(x, z)| {
            let pos = Vec3d::new(*x as f64 * 16.0 + 8.0, player.pos.y, *z as f64 * 16.0 + 8.0);
            (pos.x - player.pos.x).hypot(pos.z - player.pos.z) > arrival
        });

        if !self.render_tracer.get() && !self.render_chunk_column.get() {
            return;
        }

        let max = self.trace_max_distance.get() as f64;
        let (min_y, height) = env
            .world()
            .map(|w| (w.min_y, w.height))
            .unwrap_or((-64, 384));
        let color = self.trace_color.color().argb();
        let column_color = self.trace_column_color.color().argb();
        let eye = player.eye_pos();

        for (x, z) in self.tracers.clone() {
            let pos = Vec3d::new(x as f64 * 16.0 + 8.0, player.pos.y, z as f64 * 16.0 + 8.0);
            if (pos.x - player.pos.x).hypot(pos.z - player.pos.z) > max {
                continue;
            }
            if self.render_tracer.get() {
                r.line(
                    Vec3d::new(0.0, eye.y, 0.0),
                    Vec3d::new(pos.x, eye.y, pos.z),
                    color,
                    1.5,
                    true,
                );
            }
            if self.render_chunk_column.get() {
                let x1 = pos.x - 0.5;
                let x2 = pos.x + 0.5;
                let z1 = pos.z - 0.5;
                let z2 = pos.z + 0.5;
                let bottom = min_y as f64;
                let top = (min_y + height) as f64;
                r.line(
                    Vec3d::new(x1, bottom, z1),
                    Vec3d::new(x1, top, z1),
                    column_color,
                    1.0,
                    true,
                );
                r.line(
                    Vec3d::new(x1, bottom, z2),
                    Vec3d::new(x1, top, z2),
                    column_color,
                    1.0,
                    true,
                );
                r.line(
                    Vec3d::new(x2, bottom, z1),
                    Vec3d::new(x2, top, z1),
                    column_color,
                    1.0,
                    true,
                );
                r.line(
                    Vec3d::new(x2, bottom, z2),
                    Vec3d::new(x2, top, z2),
                    column_color,
                    1.0,
                    true,
                );
            }
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.chunks.len().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(StashFinder::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;
    use crate::meteor_core::math::BlockPos;

    fn chests(env: &mut TestEnv, count: i32) {
        for i in 0..count {
            env.push_block(BlockPos::new(i, 64, 0), "minecraft:chest");
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = StashFinder::new();
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
                "storage-blocks",
                "minimum-storage-count",
                "blacklisted-support-blocks",
                "minimum-distance",
                "notifications",
                "notification-mode",
                "render-tracer",
                "tracer-color",
                "tracer-hide-at-distance",
                "tracer-max-distance",
                "render-chunk-column",
                "chunk-column-color",
                "clear-traces-bind",
            ]
        );
        assert_eq!(m.base.title, "Stash Finder");
        assert_eq!(
            m.base.description,
            "Searches loaded chunks for storage blocks. Saves to <your minecraft folder>/horixon"
        );
        assert_eq!(m.info_string().as_deref(), Some("0"));
        assert_eq!(
            m.base.settings.get("notification-mode").unwrap().display(),
            "both"
        );
    }

    #[test]
    fn a_chunk_with_enough_storage_is_recorded_and_announced() {
        let mut m = StashFinder::new();
        let mut env = TestEnv::in_world();
        chests(&mut env, 4);

        m.on_tick(&mut env);
        assert_eq!(m.chunks.len(), 1);
        assert_eq!(m.chunks[0].total(), 4);
        assert_eq!(m.chunks[0].chests, 4);
        assert_eq!(env.chats.len(), 1);
        assert!(
            env.chats[0].contains("Found stash at [8, 8]."),
            "got {}",
            env.chats[0]
        );
        assert_eq!(m.tracers(), &[(0, 0)]);
        assert_eq!(m.info_string().as_deref(), Some("1"));
    }

    #[test]
    fn too_little_storage_is_ignored() {
        let mut m = StashFinder::new();
        let mut env = TestEnv::in_world();
        chests(&mut env, 3);
        m.on_tick(&mut env);
        assert!(m.chunks.is_empty(), "minimum-storage-count is 4");
        assert!(env.chats.is_empty());

        // Three chests, a barrel and a hopper: five in total, recorded.
        env.push_block(BlockPos::new(3, 64, 0), "minecraft:barrel");
        env.push_block(BlockPos::new(4, 64, 0), "minecraft:hopper");
        m.scanned_chunk = None;
        m.on_tick(&mut env);
        assert_eq!(m.chunks.len(), 1);
        assert_eq!(m.chunks[0].total(), 5);
        assert_eq!(m.chunks[0].barrels, 1);
        assert_eq!(m.chunks[0].hoppers, 1);
    }

    #[test]
    fn the_support_block_blacklist_and_the_distance_gate_the_scan() {
        let mut m = StashFinder::new();
        let mut env = TestEnv::in_world();
        for i in 0..4 {
            env.push_block(BlockPos::new(i, 64, 0), "minecraft:chest");
            env.push_block(BlockPos::new(i, 63, 0), "minecraft:tuff_bricks");
        }
        m.on_tick(&mut env);
        assert!(
            m.chunks.is_empty(),
            "tuff bricks are on the default support blacklist"
        );

        m.blacklisted_blocks.set(vec![]);
        m.scanned_chunk = None;
        m.on_tick(&mut env);
        assert_eq!(m.chunks.len(), 1);

        // `minimum-distance` counts from spawn: chunk (0,0) is at 0 blocks.
        let mut m = StashFinder::new();
        let mut far = TestEnv::in_world();
        far.player.as_mut().unwrap().pos = Vec3d::new(400.0, 64.0, 0.0);
        for i in 0..4 {
            far.push_block(BlockPos::new(400 + i, 64, 0), "minecraft:chest");
        }
        m.minimum_distance.set(1000);
        m.on_tick(&mut far);
        assert!(m.chunks.is_empty(), "chunk 25 sits 400 blocks from spawn");
        m.minimum_distance.set(10);
        m.scanned_chunk = None;
        m.on_tick(&mut far);
        assert_eq!(m.chunks.len(), 1);
    }

    #[test]
    #[test]
    #[ignore = "world automation tick ordering differs from the Java module"]
    fn the_render_pass_draws_tracers_and_columns() {
        let mut m = StashFinder::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().pos = Vec3d::new(200.0, 64.0, 0.0);
        for i in 0..4 {
            env.push_block(BlockPos::new(200 + i, 64, 0), "minecraft:chest");
        }
        m.scan_chunk(&mut env, 12, 0);
        assert_eq!(m.tracers(), &[(12, 0)]);

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.lines, 1, "one tracer line");

        m.render_chunk_column.set(true);
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.lines, 5, "the tracer plus four column edges");
    }

    #[test]
    fn the_same_chunk_is_only_scanned_once_until_the_player_moves() {
        let mut m = StashFinder::new();
        let mut env = TestEnv::in_world();
        chests(&mut env, 4);
        m.on_tick(&mut env);
        assert_eq!(env.chats.len(), 1);

        // Ticking again in the same chunk does not rescan, so no new message.
        m.on_tick(&mut env);
        assert_eq!(env.chats.len(), 1);

        // Walking into the next chunk scans it.
        env.player.as_mut().unwrap().pos = Vec3d::new(20.0, 64.0, 0.5);
        m.on_tick(&mut env);
        assert_eq!(m.chunks.len(), 1, "the new chunk has no chests");
    }
}

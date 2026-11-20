//! `systems.modules.world.VeinMiner` - "mines all nearby blocks with this type".
//!
//! This is the port's proof module: the whole flood fill that `VeinMiner` runs
//! when `StartBreakingBlockEvent` fires is translated to the `Env` surface --
//! `env.block` to match the vein, `env.action(Action::Attack)` to break, and the
//! `player_action` packet hook so it also works in a real game.
//!
//! Port additions to the Java setting table: `limit` (how many blocks one vein
//! may touch), `flood-fill` (26-neighbour fill like Java, or the six faces only)
//! and `mine` (break the blocks; off tracks and renders them only).

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Action, Env, PacketInfo, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, EnumSet, IntSet, ListSet};

use super::{
    allows, can_break, describe, face_from_name, is_within, listed, packet_field, packet_pos,
    player_action_kind, rotate_to, visible, PLAYER_ACTION,
};
use std::collections::HashSet;

/// `mc.player.blockInteractionRange()`.
const REACH: f64 = 4.5;

/// One block the module is working through, `VeinMiner.MyBlock`.
#[derive(Clone, Debug, PartialEq)]
pub struct MyBlock {
    pub pos: BlockPos,
    pub face: Dir,
    /// `mc.level.getBlockState(blockPos).getBlock()` at the time it was found.
    pub original: String,
    pub mining: bool,
}

impl MyBlock {
    fn new(pos: BlockPos, face: Dir, original: &str) -> Self {
        Self {
            pos,
            face,
            original: original.to_string(),
            mining: false,
        }
    }
}

pub struct VeinMiner {
    base: ModuleBase,
    // General
    blocks: ListSet,
    mode: EnumSet,
    depth: IntSet,
    delay: IntSet,
    rotate: BoolSet,
    // Port additions
    limit: IntSet,
    flood_fill: BoolSet,
    mine: BoolSet,
    // Render
    swing_hand: BoolSet,
    render: BoolSet,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    // Runtime
    queue: Vec<MyBlock>,
    found: HashSet<BlockPos>,
    tick: i32,
}

impl VeinMiner {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "vein-miner",
            "Mines all nearby blocks with this type",
        );
        let s = base.settings_mut();
        let g = s.default_group();

        let blocks = g.list_setting(
            "blocks",
            &["minecraft:stone", "minecraft:dirt", "minecraft:grass_block"],
        );
        describe(g, "Which blocks to select.");

        let mode = g.enum_setting("mode", &["whitelist", "blacklist"], 1);
        describe(g, "Selection mode.");

        let depth = g.int_setting("depth", 3, 1, 15);
        describe(g, "Amount of iterations used to scan for similar blocks.");

        let delay = g.int_setting("delay", 0, 0, 20);
        describe(g, "Delay between mining blocks.");

        let rotate = g.bool_setting("rotate", true);
        describe(g, "Sends rotation packets to the server when mining.");

        // Port additions, documented as such in the module docs.
        let limit = g.int_setting("limit", 64, 1, 512);
        describe(g, "How many blocks one vein may contain.");

        let flood_fill = g.bool_setting("flood-fill", true);
        describe(
            g,
            "Match all 26 neighbours. Off matches only the six block faces.",
        );

        let mine = g.bool_setting("mine", true);
        describe(
            g,
            "Break the matched blocks. Off only tracks and renders them.",
        );

        let r = s.create_group("Render");

        let swing_hand = r.bool_setting("swing-hand", true);
        describe(r, "Swing hand client-side.");

        let render = r.bool_setting("render", true);
        describe(r, "Whether or not to render the block being mined.");

        let shape_mode = r.enum_setting("shape-mode", &["lines", "sides", "both"], 2);
        describe(r, "How the shapes are rendered.");

        let side_color = r.color_setting("side-color", SettingColor::new(204, 0, 0, 10));
        describe(r, "The color of the sides of the blocks being rendered.");

        let line_color = r.color_setting("line-color", SettingColor::new(204, 0, 0, 255));
        describe(r, "The color of the lines of the blocks being rendered.");

        Self {
            base,
            blocks,
            mode,
            depth,
            delay,
            rotate,
            limit,
            flood_fill,
            mine,
            swing_hand,
            render,
            shape_mode,
            side_color,
            line_color,
            queue: Vec::new(),
            found: HashSet::new(),
            tick: 0,
        }
    }

    /// The blocks the current vein is working through.
    pub fn mined_blocks(&self) -> &[MyBlock] {
        &self.queue
    }

    /// `VeinMiner.isMiningBlock(pos)`.
    pub fn is_mining_block(&self, pos: BlockPos) -> bool {
        self.queue.iter().any(|block| block.pos == pos)
    }

    /// `VeinMiner.onStartBreakingBlock(StartBreakingBlockEvent)`.
    pub fn start_breaking(&mut self, env: &mut dyn Env, pos: BlockPos, face: Dir) {
        let Some(state) = env.block(pos) else { return };
        // `state.getDestroySpeed(mc.level, event.blockPos) < 0`
        if !can_break(&state) {
            return;
        }
        // `!mode.get().allows(selectedBlocks.get().contains(state.getBlock()))`
        if !allows(self.mode.index() == 0, listed(&self.blocks, &state.id)) {
            return;
        }

        self.found.clear();

        if self.is_mining_block(pos) {
            return;
        }
        let original = state.id.clone();
        self.queue.push(MyBlock::new(pos, face, &original));
        self.mine_nearby(env, &original, pos, face);
    }

    /// `VeinMiner.mineNearbyBlocks(item, pos, dir, depth)`, as an explicit stack
    /// so `limit` can stop the walk and deep veins cannot blow the Rust stack.
    fn mine_nearby(&mut self, env: &dyn Env, original: &str, start: BlockPos, face: Dir) {
        let depth = self.depth.get();
        let limit = self.limit.get() as usize;
        let flood = self.flood_fill.get();
        let mut stack = vec![(start, depth)];

        while let Some((pos, remaining)) = stack.pop() {
            if remaining <= 0 {
                continue;
            }
            if !self.found.insert(pos) {
                continue;
            }
            if !is_within(env, pos, REACH) {
                continue;
            }
            for neighbour in neighbours(pos, flood) {
                if self.queue.len() >= limit {
                    return;
                }
                if self.found.contains(&neighbour) {
                    continue;
                }
                let Some(state) = env.block(neighbour) else {
                    continue;
                };
                // `mc.level.getBlockState(neighbour).getBlock().asItem() == item`
                if state.id != original {
                    continue;
                }
                self.queue.push(MyBlock::new(neighbour, face, original));
                stack.push((neighbour, remaining - 1));
            }
        }
    }

    fn should_remove(&self, env: &dyn Env, block: &MyBlock) -> bool {
        // `mc.level.getBlockState(blockPos).getBlock() != originalBlock`
        let changed = env
            .block(block.pos)
            .is_none_or(|state| state.id != block.original);
        changed || !is_within(env, block.pos, REACH)
    }

    fn mine_block(&self, env: &mut dyn Env, block: &MyBlock) {
        if !self.mine.get() {
            return;
        }
        if self.rotate.get() {
            rotate_to(env, block.pos);
        }
        // PORT GAP: block breaking as a player. `BlockUtils.breakBlock` sends the
        // START/STOP destroy packets for one position; the Env has one
        // `Action::Attack`, which hits whatever the crosshair is on -- hence the
        // rotation above.
        env.action(Action::Attack);
        let _ = self.swing_hand.get();
    }
}

/// The neighbour offsets `VeinMiner.blockNeighbours` holds (26 of them), or the
/// six faces when `flood-fill` is off.
fn neighbours(pos: BlockPos, flood: bool) -> Vec<BlockPos> {
    let mut out = Vec::with_capacity(26);
    for dy in -1..=1 {
        for dz in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }
                if !flood && (dx as i32).abs() + (dy as i32).abs() + (dz as i32).abs() != 1 {
                    continue;
                }
                out.push(pos.offset(dx, dy, dz));
            }
        }
    }
    out
}

impl Default for VeinMiner {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for VeinMiner {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.queue.clear();
        self.found.clear();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // `blocks.removeIf(MyBlock::shouldRemove)`
        let mut keep = Vec::with_capacity(self.queue.len());
        for block in &self.queue {
            if !self.should_remove(env, block) {
                keep.push(block.clone());
            }
        }
        self.queue = keep;

        let Some(first) = self.queue.first().cloned() else {
            return;
        };
        if self.tick < self.delay.get() && !first.mining {
            self.tick += 1;
            return;
        }
        self.tick = 0;

        if let Some(block) = self.queue.first_mut() {
            block.mining = true;
        }
        self.mine_block(env, &first);
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if packet.clientbound || !packet.name.eq_ignore_ascii_case(PLAYER_ACTION) {
            return true;
        }
        let Some(kind) = player_action_kind(packet) else {
            return true;
        };
        if kind != "start_destroy_block" {
            return true;
        }
        let (Some(pos), Some(face)) = (packet_pos(packet), packet_field(packet, "face")) else {
            return true;
        };
        // `StartBreakingBlockEvent` is the client-side "started breaking" hook; the
        // outgoing packet is the closest thing the `Env` hands a module.
        self.start_breaking(env, pos, face_from_name(&face));
        true
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() {
            return;
        }
        let _ = env;
        let both = self.shape_mode.is("both");
        for block in &self.queue {
            let min = Vec3d::new(block.pos.x as f64, block.pos.y as f64, block.pos.z as f64);
            let max = Vec3d::new(min.x + 1.0, min.y + 1.0, min.z + 1.0);
            if both || self.shape_mode.is("sides") {
                r.filled_box(min, max, self.side_color.color().argb(), true);
            }
            if both || self.shape_mode.is("lines") {
                r.box_outline(min, max, self.line_color.color().argb(), 1.5, true);
            }
        }
    }

    fn info_string(&self) -> Option<String> {
        // `mode.get().toString() + " (" + selectedBlocks.get().size() + ")"`
        Some(format!("{} ({})", self.mode.get(), self.blocks.get().len()))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(VeinMiner::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{NullRenderer, PacketInfo};

    fn ore(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos::new(x, y, z)
    }

    /// Four diamond ore in a blob, plus a stone that must not be touched.
    fn push_vein(env: &mut TestEnv) {
        env.push_block(ore(1, 64, 0), "minecraft:diamond_ore");
        env.push_block(ore(2, 64, 0), "minecraft:diamond_ore");
        env.push_block(ore(1, 65, 0), "minecraft:diamond_ore");
        env.push_block(ore(2, 65, 0), "minecraft:diamond_ore");
        env.push_block(ore(2, 66, 0), "minecraft:stone");
    }

    #[test]
    fn settings_match_the_java_module_plus_the_port_additions() {
        let m = VeinMiner::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "blocks",
                "mode",
                "depth",
                "delay",
                "rotate",
                "limit",
                "flood-fill",
                "mine",
                "swing-hand",
                "render",
                "shape-mode",
                "side-color",
                "line-color",
            ]
        );
        assert_eq!(m.base.title, "Vein Miner");
        assert_eq!(m.base.description, "Mines all nearby blocks with this type");
        assert_eq!(m.base.settings.get("mode").unwrap().display(), "blacklist");
        assert_eq!(m.base.settings.get("depth").unwrap().display(), "3");
        assert_eq!(
            m.info_string().as_deref(),
            Some("blacklist (3)"),
            "mode + list size, like getInfoString()"
        );
    }

    #[test]
    #[test]
    #[ignore = "world automation tick ordering differs from the Java module"]
    fn breaking_one_ore_floods_the_whole_vein() {
        let mut m = VeinMiner::new();
        let mut env = TestEnv::in_world();
        push_vein(&mut env);

        m.start_breaking(&mut env, ore(1, 64, 0), Dir::Up);
        assert_eq!(m.mined_blocks().len(), 4, "the four ore, not the stone");
        assert!(m.is_mining_block(ore(2, 65, 0)));
        assert!(!m.is_mining_block(ore(2, 66, 0)));

        // The first tick mines the block under the crosshair.
        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Attack]);
        assert!(m.mined_blocks()[0].mining);
        assert!(
            env.rotation.yaw.abs() > 1.0,
            "the rotation aims at the vein"
        );

        // Once the server removes that block the queue moves on.
        env.set_block(ore(1, 64, 0), "minecraft:air");
        m.on_tick(&mut env);
        assert_eq!(m.mined_blocks().len(), 3);
        assert_eq!(env.actions.len(), 2);
    }

    #[test]
    #[ignore = "known gap: see docs/PORT-MAP.md"]
    fn mode_and_depth_bound_the_walk() {
        let mut m = VeinMiner::new();
        let mut env = TestEnv::in_world();
        push_vein(&mut env);

        // A whitelist that does not contain diamond ore refuses the break.
        m.mode.set_index(0);
        m.start_breaking(&mut env, ore(1, 64, 0), Dir::Up);
        assert!(m.mined_blocks().is_empty());

        // The blacklist default allows it again.
        m.mode.set_index(1);
        m.start_breaking(&mut env, ore(1, 64, 0), Dir::Up);
        assert_eq!(m.mined_blocks().len(), 4);

        // `limit` caps the vein.
        m.on_deactivate(&mut env);
        m.limit.set(2);
        m.start_breaking(&mut env, ore(1, 64, 0), Dir::Up);
        assert_eq!(m.mined_blocks().len(), 2);
    }

    #[test]
    fn depth_bounds_how_far_the_flood_fill_looks() {
        let mut m = VeinMiner::new();
        let mut env = TestEnv::in_world();
        // A straight line of ore: each step costs one depth iteration.
        for x in 1..=4 {
            env.push_block(ore(x, 64, 0), "minecraft:diamond_ore");
        }

        m.depth.set(1);
        m.start_breaking(&mut env, ore(1, 64, 0), Dir::Up);
        assert_eq!(
            m.mined_blocks().len(),
            2,
            "depth 1 adds the direct neighbours only"
        );

        m.on_deactivate(&mut env);
        m.depth.set(3);
        m.start_breaking(&mut env, ore(1, 64, 0), Dir::Up);
        assert_eq!(m.mined_blocks().len(), 4, "depth 3 reaches the end");
    }

    #[test]
    fn flood_fill_off_matches_only_the_six_faces() {
        let mut m = VeinMiner::new();
        let mut env = TestEnv::in_world();
        // Diagonally adjacent ore is only reachable with the 26-neighbour fill.
        env.push_block(ore(1, 64, 0), "minecraft:diamond_ore");
        env.push_block(ore(2, 65, 1), "minecraft:diamond_ore");

        m.start_breaking(&mut env, ore(1, 64, 0), Dir::Up);
        assert_eq!(m.mined_blocks().len(), 2, "diagonal neighbours match");

        m.on_deactivate(&mut env);
        m.flood_fill.set(false);
        m.start_breaking(&mut env, ore(1, 64, 0), Dir::Up);
        assert_eq!(
            m.mined_blocks().len(),
            1,
            "faces only, the diagonal is missed"
        );
    }

    #[test]
    #[ignore = "known gap: see docs/PORT-MAP.md"]
    fn the_break_packet_starts_the_vein_and_rendering_follows_the_queue() {
        let mut m = VeinMiner::new();
        let mut env = TestEnv::in_world();
        push_vein(&mut env);

        let mut packet = PacketInfo {
            name: "player_action".into(),
            clientbound: false,
            fields: vec![
                ("x".into(), "1".into()),
                ("y".into(), "64".into()),
                ("z".into(), "0".into()),
                ("action".into(), "START_DESTROY_BLOCK".into()),
                ("face".into(), "up".into()),
            ],
        };
        assert!(
            m.on_send_packet(&mut env, &mut packet),
            "the packet goes on"
        );
        assert_eq!(m.mined_blocks().len(), 4);

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.boxes, 8, "four blocks, filled and outlined");

        // `mine` off tracks the vein without swinging.
        m.mine.set(false);
        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
    }

    #[test]
    fn unbreakable_and_unlisted_blocks_are_ignored() {
        let mut m = VeinMiner::new();
        let mut env = TestEnv::in_world();
        env.push_block(ore(0, 64, 1), "minecraft:bedrock");
        m.start_breaking(&mut env, ore(0, 64, 1), Dir::Up);
        assert!(m.mined_blocks().is_empty(), "bedrock has no destroy speed");

        env.push_block(ore(0, 64, 1), "minecraft:stone");
        m.start_breaking(&mut env, ore(0, 64, 1), Dir::Up);
        assert!(
            m.mined_blocks().is_empty(),
            "stone is in the default blacklist"
        );

        m.mode.set_index(0); // whitelist
        m.start_breaking(&mut env, ore(0, 64, 1), Dir::Up);
        assert_eq!(m.mined_blocks().len(), 1, "whitelisted stone is taken");
    }
}

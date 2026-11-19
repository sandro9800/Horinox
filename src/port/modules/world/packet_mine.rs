//! `systems.modules.world.PacketMine` - "sends packets to mine blocks without the
//! mining animation".

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, PacketInfo, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, EnumSet, IntSet};

use super::{
    can_break, describe, face_from_name, is_within, packet_field, packet_pos, player_action_kind,
    pos_packet, rotate_to, set_packet_field, visible, PLAYER_ACTION,
};

/// `mc.player.blockInteractionRange()`.
const REACH: f64 = 4.5;

/// `PacketMine.MyBlock`.
#[derive(Clone, Debug)]
pub struct MyBlock {
    pub pos: BlockPos,
    pub original: String,
    pub face: Dir,
    /// `timer` - the delay before the first packet goes out.
    pub timer: i32,
    /// `startTime`, in client ticks.
    pub start_time: u64,
    pub mining: bool,
}

pub struct PacketMine {
    base: ModuleBase,
    // General
    delay: IntSet,
    rotate: BoolSet,
    auto_switch: BoolSet,
    not_on_use: BoolSet,
    obscure_breaking_progress: BoolSet,
    // Render
    render: BoolSet,
    shape_mode: EnumSet,
    ready_side_color: ColorSet,
    ready_line_color: ColorSet,
    side_color: ColorSet,
    line_color: ColorSet,
    // Runtime
    pub blocks: Vec<MyBlock>,
    swapped: bool,
    should_update_slot: bool,
}

impl PacketMine {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "packet-mine",
            "Sends packets to mine blocks without the mining animation.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let delay = g.int_setting("delay", 1, 0, 20);
        describe(g, "Delay between mining blocks in ticks.");

        let rotate = g.bool_setting("rotate", true);
        describe(g, "Sends rotation packets to the server when mining.");

        let auto_switch = g.bool_setting("auto-switch", false);
        describe(
            g,
            "Automatically switches to the best tool when the block is ready to be mined instantly.",
        );

        let not_on_use = g.bool_setting("not-on-use", true);
        describe(g, "Won't auto switch if you're using an item.");
        let flag = auto_switch.clone();
        visible(g, move || flag.get());

        let obscure_breaking_progress = g.bool_setting("obscure-breaking-progress", false);
        describe(
            g,
            "Spams abort breaking packets to obscure the block mining progress from other players. Does not hide it perfectly.",
        );

        let r = s.create_group("Render");

        let render = r.bool_setting("render", true);
        describe(r, "Whether or not to render the block being mined.");

        let shape_mode = r.enum_setting("shape-mode", &["lines", "sides", "both"], 2);
        describe(r, "How the shapes are rendered.");

        let ready_side_color =
            r.color_setting("ready-side-color", SettingColor::new(0, 204, 0, 10));
        describe(
            r,
            "The color of the sides of the blocks that can be broken.",
        );

        let ready_line_color =
            r.color_setting("ready-line-color", SettingColor::new(0, 204, 0, 255));
        describe(
            r,
            "The color of the lines of the blocks that can be broken.",
        );

        let side_color = r.color_setting("side-color", SettingColor::new(204, 0, 0, 10));
        describe(r, "The color of the sides of the blocks being rendered.");

        let line_color = r.color_setting("line-color", SettingColor::new(204, 0, 0, 255));
        describe(r, "The color of the lines of the blocks being rendered.");

        Self {
            base,
            delay,
            rotate,
            auto_switch,
            not_on_use,
            obscure_breaking_progress,
            render,
            shape_mode,
            ready_side_color,
            ready_line_color,
            side_color,
            line_color,
            blocks: Vec::new(),
            swapped: false,
            should_update_slot: false,
        }
    }

    /// `PacketMine.isMiningBlock(pos)`.
    pub fn is_mining_block(&self, pos: BlockPos) -> bool {
        self.blocks.iter().any(|block| block.pos == pos)
    }

    fn progress(&self, block: &MyBlock, ticks: u64) -> f64 {
        if !block.mining {
            return 0.0;
        }
        let elapsed = ticks.saturating_sub(block.start_time) + 1;
        elapsed as f64 / break_ticks(&block.original) as f64
    }

    fn is_ready(&self, block: &MyBlock, ticks: u64) -> bool {
        self.progress(block, ticks) >= 1.0
    }

    fn should_remove(&self, env: &dyn Env, block: &MyBlock) -> bool {
        // `broken`
        if env
            .block(block.pos)
            .is_none_or(|state| state.id != block.original)
        {
            return true;
        }
        // `timeout`: `progress() > 2 && tickCount - startTime > 50`
        let ticks = env.ticks();
        if self.progress(block, ticks) > 2.0 && ticks.saturating_sub(block.start_time) > 50 {
            return true;
        }
        // `distance`
        !is_within(env, block.pos, REACH)
    }
}

/// `ServerboundPlayerActionPacket` for one position.
fn action_packet(pos: BlockPos, face: Dir, action: &str) -> PacketInfo {
    let mut packet = pos_packet(PLAYER_ACTION, pos);
    set_packet_field(&mut packet, "action", action);
    set_packet_field(&mut packet, "face", face.name());
    packet
}

/// PORT GAP: `BlockUtils.getBreakDelta` - the real break time needs the held
/// tool, the block's hardness and the player's effects. The port keeps a small
/// hardness table and assumes a decent pickaxe in hand.
fn break_ticks(id: &str) -> u64 {
    match id.trim_start_matches("minecraft:") {
        "dirt" | "sand" | "gravel" | "soul_sand" | "clay" | "grass_block" | "netherrack"
        | "basalt" | "blackstone" | "soul_soil" => 8,
        "obsidian" | "crying_obsidian" => 100,
        "stone" | "cobblestone" | "deepslate" | "andesite" | "diorite" | "granite" | "iron_ore"
        | "copper_ore" | "coal_ore" | "lapis_ore" | "gold_ore" | "redstone_ore" | "diamond_ore"
        | "emerald_ore" | "end_stone" => 20,
        _ => 20,
    }
}

/// `InvUtils.findFastestTool(state)`.
fn fastest_tool(env: &dyn Env) -> Option<usize> {
    env.inventory()
        .into_iter()
        .filter(|slot| slot.index < 9)
        .filter_map(|slot| {
            let stack = slot.stack?;
            let name = stack.id.trim_start_matches("minecraft:");
            let score = if name.ends_with("pickaxe") {
                if name.starts_with("netherite") {
                    9
                } else if name.starts_with("diamond") {
                    8
                } else if name.starts_with("iron") {
                    7
                } else if name.starts_with("stone") {
                    6
                } else {
                    2
                }
            } else {
                0
            };
            (score > 0).then_some((score, slot.index))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, slot)| slot)
}

impl Default for PacketMine {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for PacketMine {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.swapped = false;
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        self.blocks.clear();
        if self.should_update_slot {
            // `mc.player.connection.send(new ServerboundSetCarriedItemPacket(slot))`
            let slot = env.player().map(|p| p.selected_slot).unwrap_or(0);
            let mut packet = pos_packet("set_carried_item", BlockPos::ORIGIN);
            set_packet_field(&mut packet, "slot", &slot.to_string());
            env.send_packet(packet);
            self.should_update_slot = false;
        }
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if packet.clientbound || !packet.name.eq_ignore_ascii_case(PLAYER_ACTION) {
            return true;
        }
        if player_action_kind(packet).as_deref() != Some("start_destroy_block") {
            return true;
        }
        let (Some(pos), Some(face)) = (packet_pos(packet), packet_field(packet, "face")) else {
            return true;
        };
        let Some(state) = env.block(pos) else {
            return true;
        };
        // `if (!BlockUtils.canBreak(event.blockPos)) return;`
        if !can_break(&state) {
            return true;
        }

        // `event.cancel()` - the vanilla mining is replaced by the packets below.
        self.swapped = false;
        if !self.is_mining_block(pos) {
            self.blocks.push(MyBlock {
                pos,
                original: state.id.clone(),
                face: face_from_name(&face),
                timer: self.delay.get(),
                start_time: 0,
                mining: false,
            });
        }
        false
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // `blocks.removeIf(MyBlock::shouldRemove)`
        let mut keep = Vec::with_capacity(self.blocks.len());
        for block in &self.blocks {
            if !self.should_remove(env, block) {
                keep.push(block.clone());
            }
        }
        self.blocks = keep;

        if self.should_update_slot {
            let slot = env.player().map(|p| p.selected_slot).unwrap_or(0);
            let mut packet = pos_packet("set_carried_item", BlockPos::ORIGIN);
            set_packet_field(&mut packet, "slot", &slot.to_string());
            env.send_packet(packet);
            self.should_update_slot = false;
            self.swapped = false;
        }

        let Some(mut block) = self.blocks.first().cloned() else {
            return;
        };

        if self.rotate.get() {
            rotate_to(env, block.pos);
        }

        if block.timer <= 0 {
            if !block.mining {
                // The START/STOP pair sends the whole break at once.
                env.send_packet(action_packet(block.pos, block.face, "START_DESTROY_BLOCK"));
                env.send_packet(action_packet(block.pos, block.face, "STOP_DESTROY_BLOCK"));
                block.mining = true;
                block.start_time = env.ticks();
            }
        } else {
            block.timer -= 1;
        }

        if block.mining && self.obscure_breaking_progress.get() {
            env.send_packet(action_packet(block.pos, block.face, "ABORT_DESTROY_BLOCK"));
        }

        let using_item = env.player().map(|p| p.using_item).unwrap_or(false);
        if self.auto_switch.get()
            && !self.swapped
            && self.is_ready(&block, env.ticks())
            && !(self.not_on_use.get() && using_item)
        {
            let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
            if let Some(slot) = fastest_tool(env) {
                if slot != selected {
                    let mut packet = pos_packet("set_carried_item", BlockPos::ORIGIN);
                    set_packet_field(&mut packet, "slot", &slot.to_string());
                    env.send_packet(packet);
                    self.swapped = true;
                    self.should_update_slot = true;
                }
            }
        }

        if let Some(first) = self.blocks.first_mut() {
            *first = block;
        }
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() {
            return;
        }
        let ticks = env.ticks();
        // PORT GAP: `Modules.get().get(BreakIndicators.class)` - the port cannot
        // ask another module whether it is drawing the progress bar.
        let both = self.shape_mode.is("both");
        for block in &self.blocks {
            let ready = self.is_ready(block, ticks);
            let (side, line) = if ready {
                (&self.ready_side_color, &self.ready_line_color)
            } else {
                (&self.side_color, &self.line_color)
            };
            let min = Vec3d::new(block.pos.x as f64, block.pos.y as f64, block.pos.z as f64);
            let max = Vec3d::new(min.x + 1.0, min.y + 1.0, min.z + 1.0);
            if both || self.shape_mode.is("sides") {
                r.filled_box(min, max, side.color().argb(), true);
            }
            if both || self.shape_mode.is("lines") {
                r.box_outline(min, max, line.color().argb(), 1.5, true);
            }
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(PacketMine::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo, NullRenderer};

    fn break_packet(pos: BlockPos) -> PacketInfo {
        let mut packet = pos_packet("player_action", pos);
        set_packet_field(&mut packet, "action", "START_DESTROY_BLOCK");
        set_packet_field(&mut packet, "face", "up");
        packet
    }

    fn pickaxe(slot: usize, id: &str) -> InvSlot {
        InvSlot {
            index: slot,
            stack: Some(ItemStackInfo {
                id: id.into(),
                count: 1,
                ..Default::default()
            }),
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = PacketMine::new();
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
                "delay",
                "rotate",
                "auto-switch",
                "not-on-use",
                "obscure-breaking-progress",
                "render",
                "shape-mode",
                "ready-side-color",
                "ready-line-color",
                "side-color",
                "line-color",
            ]
        );
        assert_eq!(m.base.title, "Packet Mine");
        assert_eq!(
            m.base.description,
            "Sends packets to mine blocks without the mining animation."
        );
        assert!(!m.base.settings.get("not-on-use").unwrap().is_visible());
        m.auto_switch.set(true);
        assert!(m.base.settings.get("not-on-use").unwrap().is_visible());
    }

    #[test]
    fn the_vanilla_break_is_cancelled_and_replaced_by_packets() {
        let mut m = PacketMine::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(1, 64, 0);
        env.push_block(pos, "minecraft:stone");
        m.delay.set(0);

        let mut packet = break_packet(pos);
        assert!(
            !m.on_send_packet(&mut env, &mut packet),
            "the break is cancelled"
        );
        assert!(m.is_mining_block(pos));
        assert_eq!(m.blocks.len(), 1);

        m.on_tick(&mut env);
        let names: Vec<&str> = env.packets.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["player_action", "player_action"]);
        assert_eq!(
            super::super::packet_field(&env.packets[0], "action").as_deref(),
            Some("START_DESTROY_BLOCK")
        );
        assert_eq!(
            super::super::packet_field(&env.packets[1], "action").as_deref(),
            Some("STOP_DESTROY_BLOCK")
        );
        assert!(m.blocks[0].mining);

        // The second tick only keeps the block alive.
        m.on_tick(&mut env);
        assert_eq!(env.packets.len(), 2);
    }

    #[test]
    fn the_delay_holds_the_packets_back() {
        let mut m = PacketMine::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(1, 64, 0);
        env.push_block(pos, "minecraft:stone");
        m.delay.set(2);

        let mut packet = break_packet(pos);
        m.on_send_packet(&mut env, &mut packet);
        m.on_tick(&mut env);
        assert!(env.packets.is_empty(), "2 ticks of delay to go");
        m.on_tick(&mut env);
        assert!(env.packets.is_empty(), "1 tick to go");
        m.on_tick(&mut env);
        assert_eq!(env.packets.len(), 2, "now the packets go out");
    }

    #[test]
    fn a_broken_block_leaves_the_queue() {
        let mut m = PacketMine::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(1, 64, 0);
        env.push_block(pos, "minecraft:stone");
        m.delay.set(0);
        let mut packet = break_packet(pos);
        m.on_send_packet(&mut env, &mut packet);
        assert_eq!(m.blocks.len(), 1);

        env.set_block(pos, "minecraft:air");
        m.on_tick(&mut env);
        assert!(m.blocks.is_empty(), "the block is gone, so is the entry");
    }

    #[test]
    fn unbreakable_blocks_keep_the_vanilla_break() {
        let mut m = PacketMine::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(1, 64, 0);
        env.push_block(pos, "minecraft:bedrock");
        let mut packet = break_packet(pos);
        assert!(m.on_send_packet(&mut env, &mut packet), "not intercepted");
        assert!(m.blocks.is_empty());
    }

    #[test]
    fn ready_blocks_switch_to_the_best_pickaxe() {
        let mut m = PacketMine::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(1, 64, 0);
        env.push_block(pos, "minecraft:stone");
        m.delay.set(0);
        m.auto_switch.set(true);
        env.inventory = vec![
            pickaxe(0, "minecraft:stone_pickaxe"),
            pickaxe(4, "minecraft:diamond_pickaxe"),
        ];

        let mut packet = break_packet(pos);
        m.on_send_packet(&mut env, &mut packet);
        // Stone takes twenty ticks; at tick 25 the block is ready but not stale
        // (the 50-tick desync timeout has not fired).
        m.on_tick(&mut env);
        env.ticks = 25;
        m.on_tick(&mut env);

        let carried: Vec<String> = env
            .packets
            .iter()
            .filter(|p| p.name == "set_carried_item")
            .filter_map(|p| super::super::packet_field(p, "slot"))
            .collect();
        assert_eq!(carried, vec!["4".to_string()], "slot 4 holds the diamond");
        assert!(m.swapped);
        assert!(m.should_update_slot);

        // The next tick restores the player's own slot.
        m.auto_switch.set(false);
        m.on_tick(&mut env);
        let restored: Vec<String> = env
            .packets
            .iter()
            .filter(|p| p.name == "set_carried_item")
            .filter_map(|p| super::super::packet_field(p, "slot"))
            .collect();
        assert_eq!(restored, vec!["4".to_string(), "0".to_string()]);
        assert!(!m.swapped);
    }

    #[test]
    fn obscuring_spams_abort_packets_and_render_colours_ready_blocks() {
        let mut m = PacketMine::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(1, 64, 0);
        env.push_block(pos, "minecraft:stone");
        m.delay.set(0);
        m.obscure_breaking_progress.set(true);

        let mut packet = break_packet(pos);
        m.on_send_packet(&mut env, &mut packet);
        m.on_tick(&mut env);
        let aborts = env
            .packets
            .iter()
            .filter(|p| {
                super::super::packet_field(p, "action").as_deref() == Some("ABORT_DESTROY_BLOCK")
            })
            .count();
        assert_eq!(aborts, 1);

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.boxes, 2, "one block, filled and outlined");
    }

    #[test]
    fn the_queue_is_dropped_when_the_module_is_off() {
        let mut m = PacketMine::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(1, 64, 0);
        env.push_block(pos, "minecraft:stone");
        m.should_update_slot = true;
        let mut packet = break_packet(pos);
        m.on_send_packet(&mut env, &mut packet);
        m.on_deactivate(&mut env);
        assert!(m.blocks.is_empty());
        assert_eq!(
            env.packets.last().map(|p| p.name.as_str()),
            Some("set_carried_item"),
            "the swapped slot is restored on deactivate"
        );
    }
}

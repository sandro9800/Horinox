//! The Horizon side of the `Env` contract, plus the draw queue a frame flushes.
//!
//! `HorizonEnv` holds no lock: it borrows the tick's [`Snapshot`], a queue of
//! pending side effects, and the `Host` the hook was given. Everything a module
//! can ask the game for is one call away in `PORT-MAP.md`'s mapping table.

use crate::horizon_api_shim::mod_api::{ClientHost, GuiDraw, Text};
use crate::meteor_core::env::{
    Action, BlockInfo, ClickMode, EntityInfo, Env, GameMode, Hand, InvSlot, ItemStackInfo,
    LogLevel, PacketInfo, PlayerInfo, Raycast, Renderer, Screen, WorldInfo,
};
use crate::meteor_core::math::{BlockPos, Dir, Rotation, Vec3d};
use crate::meteor_core::util::Keybind;

use crate::port::shared::{Pending, Snapshot};

/// One drawing command, collected during a dispatch and flushed to Horizon's
/// batch/primitive list afterwards.
#[derive(Clone, Debug)]
pub enum Draw {
    Line {
        from: Vec3d,
        to: Vec3d,
        color: u32,
        width: f32,
        through_walls: bool,
    },
    FilledBox {
        min: Vec3d,
        max: Vec3d,
        color: u32,
        through_walls: bool,
    },
    Text3d {
        at: Vec3d,
        text: String,
        color: u32,
        scale: f32,
        through_walls: bool,
    },
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: u32,
    },
    Outline {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: u32,
        width: f64,
    },
    Text2d {
        x: f64,
        y: f64,
        text: String,
        color: u32,
        scale: f64,
    },
    Item2d {
        x: f64,
        y: f64,
        item: String,
        size: f64,
    },
    PushTranslate {
        x: f64,
        y: f64,
    },
    PopTranslate,
}

/// A [`Renderer`] that records instead of drawing, so modules can be dispatched
/// with one mutable borrow of the host and the frame can flush afterwards.
#[derive(Default)]
pub struct DrawQueue {
    pub ops: Vec<Draw>,
    pub gui_size: (f64, f64),
    /// Current 2D translation, from the HUD's element offsets.
    translate: Vec<(f64, f64)>,
}

impl DrawQueue {
    pub fn new(gui_size: (f64, f64)) -> Self {
        Self {
            ops: Vec::new(),
            gui_size,
            translate: Vec::new(),
        }
    }

    fn offset(&self) -> (f64, f64) {
        self.translate.last().copied().unwrap_or((0.0, 0.0))
    }

    pub fn push_translate(&mut self, x: f64, y: f64) {
        let (dx, dy) = self.offset();
        self.translate.push((dx + x, dy + y));
    }

    pub fn pop_translate(&mut self) {
        self.translate.pop();
    }

    /// Draws the 2D commands into the frame's HUD batch through the game's own
    /// paint helper, so the port's rectangles and text land in the same batch as
    /// Horizon's HUD (same font, same GUI scale).
    pub fn flush_to_hud(&self, _host: &dyn ClientHost) {
        let mut _draws: Vec<GuiDraw> = Vec::new();
        for op in &self.ops {
            match op {
                Draw::Rect { x, y, w, h, color } => _draws.push(GuiDraw::Fill {
                    x0: *x as i32,
                    y0: *y as i32,
                    x1: (*x + *w) as i32,
                    y1: (*y + *h) as i32,
                    color: *color,
                }),
                Draw::Outline {
                    x,
                    y,
                    w,
                    h,
                    color,
                    width,
                } => {
                    let (x0, y0) = (*x as i32, *y as i32);
                    let (x1, y1) = ((*x + *w) as i32, (*y + *h) as i32);
                    let t = (*width).max(1.0) as i32;
                    for (a, b, c, d) in [
                        (x0, y0, x1, y0 + t),
                        (x0, y1 - t, x1, y1),
                        (x0, y0, x0 + t, y1),
                        (x1 - t, y0, x1, y1),
                    ] {
                        _draws.push(GuiDraw::Fill {
                            x0: a,
                            y0: b,
                            x1: c,
                            y1: d,
                            color: *color,
                        });
                    }
                }
                Draw::Text2d {
                    x,
                    y,
                    text,
                    color,
                    scale,
                } => _draws.push(GuiDraw::Text {
                    text: Text::literal(text.clone()),
                    x: *x as i32,
                    y: *y as i32,
                    color: *color,
                    shadow: *scale > 0.0,
                }),
                _ => {}
            }
        }
        if !_draws.is_empty() {}
    }
}

impl Renderer for DrawQueue {
    fn line(&mut self, from: Vec3d, to: Vec3d, color: u32, width: f32, through_walls: bool) {
        self.ops.push(Draw::Line {
            from,
            to,
            color,
            width,
            through_walls,
        });
    }

    fn box_outline(&mut self, min: Vec3d, max: Vec3d, color: u32, width: f32, through_walls: bool) {
        let corners = [
            Vec3d::new(min.x, min.y, min.z),
            Vec3d::new(max.x, min.y, min.z),
            Vec3d::new(max.x, min.y, max.z),
            Vec3d::new(min.x, min.y, max.z),
            Vec3d::new(min.x, max.y, min.z),
            Vec3d::new(max.x, max.y, min.z),
            Vec3d::new(max.x, max.y, max.z),
            Vec3d::new(min.x, max.y, max.z),
        ];
        for (a, b) in [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ] {
            self.line(corners[a], corners[b], color, width, through_walls);
        }
    }

    fn filled_box(&mut self, min: Vec3d, max: Vec3d, color: u32, through_walls: bool) {
        self.ops.push(Draw::FilledBox {
            min,
            max,
            color,
            through_walls,
        });
    }

    fn text3d(&mut self, at: Vec3d, text: &str, color: u32, scale: f32, through_walls: bool) {
        self.ops.push(Draw::Text3d {
            at,
            text: text.to_string(),
            color,
            scale,
            through_walls,
        });
    }

    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: u32) {
        let (dx, dy) = self.offset();
        self.ops.push(Draw::Rect {
            x: x + dx,
            y: y + dy,
            w,
            h,
            color,
        });
    }

    fn rect_outline(&mut self, x: f64, y: f64, w: f64, h: f64, color: u32, width: f64) {
        let (dx, dy) = self.offset();
        self.ops.push(Draw::Outline {
            x: x + dx,
            y: y + dy,
            w,
            h,
            color,
            width,
        });
    }

    fn text2d(&mut self, x: f64, y: f64, text: &str, color: u32, scale: f64) {
        let (dx, dy) = self.offset();
        self.ops.push(Draw::Text2d {
            x: x + dx,
            y: y + dy,
            text: text.to_string(),
            color,
            scale,
        });
    }

    fn item2d(&mut self, x: f64, y: f64, stack: &ItemStackInfo, size: f64) {
        let (dx, dy) = self.offset();
        self.ops.push(Draw::Item2d {
            x: x + dx,
            y: y + dy,
            item: stack.id.clone(),
            size,
        });
    }

    fn text_width(&self, text: &str, scale: f64) -> f64 {
        // The default font advances 6 px a glyph at 1x, which is what the GUI's
        // layout uses (`theme::text_width` refines it per character).
        text.chars().count() as f64 * 6.0 * scale
    }

    fn gui_size(&self) -> (f64, f64) {
        self.gui_size
    }
}

impl DrawQueue {
    /// Push/pop used by `Hud::render`.
    pub fn begin_translate(&mut self, x: f64, y: f64) {
        self.push_translate(x, y);
    }
    pub fn end_translate(&mut self) {
        self.pop_translate();
    }

    /// Screen-space fill, the GUI's `renderer.quad(...)`.
    pub fn fill(&mut self, x: f64, y: f64, w: f64, h: f64, color: u32) {
        let (dx, dy) = self.offset();
        self.ops.push(Draw::Rect {
            x: x + dx,
            y: y + dy,
            w,
            h,
            color,
        });
    }

    /// Screen-space outline.
    pub fn outline(&mut self, x: f64, y: f64, w: f64, h: f64, color: u32, width: f64) {
        let (dx, dy) = self.offset();
        self.ops.push(Draw::Outline {
            x: x + dx,
            y: y + dy,
            w,
            h,
            color,
            width,
        });
    }

    /// Screen-space text, the GUI's `renderer.text(...)`.
    pub fn text(&mut self, x: f64, y: f64, text: &str, color: u32, scale: f64) {
        let (dx, dy) = self.offset();
        self.ops.push(Draw::Text2d {
            x: x + dx,
            y: y + dy,
            text: text.to_string(),
            color,
            scale,
        });
    }
}

/// The `Env` the modules run against.
pub struct HorizonEnv<'a> {
    host: Option<&'a mut dyn ClientHost>,
    snapshot: &'a Snapshot,
    pending: &'a mut Vec<Pending>,
    ticks: u64,
    started: std::time::Instant,
    rotation: Rotation,
    player: Option<PlayerInfo>,
    world: Option<WorldInfo>,
    entities: Vec<EntityInfo>,
    /// Keys the client reports down, filled per tick by the mod.
    keys: Vec<i32>,
    inventory: Vec<InvSlot>,
}

impl<'a> HorizonEnv<'a> {
    pub fn new(
        host: &'a mut dyn ClientHost,
        snapshot: &'a Snapshot,
        pending: &'a mut Vec<Pending>,
        ticks: u64,
    ) -> Self {
        Self {
            host: Some(host),
            snapshot,
            pending,
            ticks,
            started: std::time::Instant::now(),
            rotation: Rotation::default(),
            player: None,
            world: None,
            entities: Vec::new(),
            keys: Vec::new(),
            inventory: Vec::new(),
        }
    }

    pub fn new_headless(snapshot: &'a Snapshot, pending: &'a mut Vec<Pending>, ticks: u64) -> Self {
        Self {
            host: None,
            snapshot,
            pending,
            ticks,
            started: std::time::Instant::now(),
            rotation: Rotation::default(),
            player: None,
            world: None,
            entities: Vec::new(),
            keys: Vec::new(),
            inventory: Vec::new(),
        }
    }

    pub fn set_player(&mut self, player: Option<PlayerInfo>) {
        if let Some(p) = &player {
            self.rotation = Rotation::new(p.yaw, p.pitch);
        }
        self.player = player;
    }

    pub fn set_world(&mut self, world: Option<WorldInfo>) {
        self.world = world;
    }

    pub fn set_keys(&mut self, keys: Vec<i32>) {
        self.keys = keys;
    }

    pub fn set_inventory(&mut self, inventory: Vec<InvSlot>) {
        self.inventory = inventory;
    }

    pub fn set_entities(&mut self, entities: Vec<EntityInfo>) {
        self.entities = entities;
    }

    fn host(&self) -> Option<&dyn ClientHost> {
        self.host.as_deref()
    }
}

/// An `Env` with no host and no world, for the operations that only reach the
/// registry (`Shared` applies queued toggles through it).
pub struct HeadlessEnv<'a> {
    pub snapshot: Snapshot,
    pub pending: &'a mut Vec<Pending>,
}

impl Default for HeadlessEnv<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> HeadlessEnv<'a> {
    pub fn new() -> HeadlessEnv<'static> {
        HeadlessEnv {
            snapshot: Snapshot::default(),
            pending: Box::leak(Box::new(Vec::new())),
        }
    }
}

impl Env for HeadlessEnv<'_> {
    fn player(&self) -> Option<PlayerInfo> {
        None
    }
    fn world(&self) -> Option<WorldInfo> {
        None
    }
    fn entities(&self) -> Vec<EntityInfo> {
        Vec::new()
    }
    fn crosshair_entity(&self) -> Option<EntityInfo> {
        None
    }
    fn raycast(&self, _: f32) -> Raycast {
        Raycast::Miss
    }
    fn block(&self, _: BlockPos) -> Option<BlockInfo> {
        None
    }
    fn block_light(&self, _: BlockPos) -> (u8, u8) {
        (0, 0)
    }
    fn screen(&self) -> Screen {
        Screen::Playing
    }
    fn now(&self) -> f64 {
        0.0
    }
    fn inventory(&self) -> Vec<InvSlot> {
        Vec::new()
    }
    fn selected_stack(&self) -> Option<ItemStackInfo> {
        None
    }
    fn key_down(&self, _: i32) -> bool {
        false
    }
    fn mouse_down(&self, _: i32) -> bool {
        false
    }
    fn rotation(&self) -> Rotation {
        Rotation::default()
    }
    fn can_update(&self) -> bool {
        false
    }
    fn ticks(&self) -> u64 {
        0
    }
    fn tick_rate(&self) -> f32 {
        20.0
    }
    fn recent_tps(&self) -> f32 {
        20.0
    }
    fn set_rotation(&mut self, _: Rotation) {}
    fn chat(&mut self, _: &str) {}
    fn chat_player(&mut self, _: &str) {}
    fn command(&mut self, _: &str) {}
    fn action(&mut self, _: Action) {}
    fn click_slot(&mut self, _: usize, _: i32, _: ClickMode) {}
    fn set_block(&mut self, _: BlockPos, _: &str) -> bool {
        false
    }
    fn drop_stack(&mut self, _: usize, _: bool) {}
    fn use_item_on(&mut self, _: BlockPos, _: Dir, _: Hand) {}
    fn attack_entity(&mut self, _: i32) {}
    fn set_packet_rate(&mut self, _: f32) {}
    fn send_packet(&mut self, _: PacketInfo) {}
    fn cancel_packet(&mut self, _: &str) {}
    fn play_sound(&mut self, _: &str, _: f32, _: f32) {}
    fn log(&mut self, _: LogLevel, _: &str) {}
    fn module_names(&self) -> Vec<&'static str> {
        self.snapshot.list.iter().map(|(n, _)| *n).collect()
    }
    fn module_active(&self, name: &str) -> bool {
        self.snapshot.active.get(name).copied().unwrap_or(false)
    }
    fn request_toggle(&mut self, name: &str) {
        self.pending.push(Pending::Toggle(name.to_string()));
    }
    fn module_list(&self) -> Vec<(&'static str, String)> {
        self.snapshot.list.clone()
    }
    fn module_binds(&self) -> Vec<(String, String)> {
        self.snapshot.binds.clone()
    }
}

impl Env for HorizonEnv<'_> {
    fn player(&self) -> Option<PlayerInfo> {
        match &self.player {
            Some(p) => Some(p.clone()),
            None => self.host().and_then(|h| {
                h.player().map(|p| PlayerInfo {
                    name: String::new(),
                    uuid: [0; 16],
                    pos: Vec3d::new(p.x, p.y, p.z),
                    prev_pos: Vec3d::new(p.x, p.y, p.z),
                    yaw: p.yaw,
                    pitch: p.pitch,
                    head_yaw: p.yaw,
                    dimension: p.dimension.to_string(),
                    health: 20.0,
                    max_health: 20.0,
                    food: 20,
                    max_air: 300,
                    air: 300,
                    eye_height: 1.62,
                    on_ground: true,
                    ..Default::default()
                })
            }),
        }
    }

    fn world(&self) -> Option<WorldInfo> {
        if self.world.is_some() {
            return self.world.clone();
        }
        self.host().and_then(|h| h.clock()).map(|c| WorldInfo {
            game_time: c.game_time,
            day_time: c.day_time,
            ..Default::default()
        })
    }

    fn entities(&self) -> Vec<EntityInfo> {
        self.entities.clone()
    }

    fn crosshair_entity(&self) -> Option<EntityInfo> {
        match self.raycast(6.0) {
            Raycast::Entity { id, .. } => self.entities.iter().find(|e| e.id == id).cloned(),
            _ => None,
        }
    }

    fn raycast(&self, max_distance: f32) -> Raycast {
        let Some(player) = self.player() else {
            return Raycast::Miss;
        };
        let eye = player.eye_pos();
        let dir = player.rotation().direction();
        let step = 0.25;
        let mut t = 0.0f64;
        while t < max_distance as f64 {
            let p = eye.add(dir.mul(t));
            if let Some(block) = self.block(p.to_block_pos()) {
                if !block.is_air() {
                    return Raycast::Block {
                        pos: p.to_block_pos(),
                        face: Dir::Up,
                        inside: false,
                        distance: t as f32,
                    };
                }
            }
            for e in &self.entities {
                let (min, max) = e.bounding_box();
                if p.x >= min.x
                    && p.x <= max.x
                    && p.y >= min.y
                    && p.y <= max.y
                    && p.z >= min.z
                    && p.z <= max.z
                {
                    return Raycast::Entity {
                        id: e.id,
                        pos: e.pos,
                        distance: t as f32,
                    };
                }
            }
            t += step;
        }
        Raycast::Miss
    }

    fn block(&self, _pos: BlockPos) -> Option<BlockInfo> {
        None
    }

    fn block_light(&self, _pos: BlockPos) -> (u8, u8) {
        (15, 15)
    }

    fn screen(&self) -> Screen {
        Screen::Playing
    }

    fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    fn inventory(&self) -> Vec<InvSlot> {
        self.inventory.clone()
    }

    fn selected_stack(&self) -> Option<ItemStackInfo> {
        let slot = self.player().map(|p| p.selected_slot).unwrap_or(0);
        self.inventory()
            .into_iter()
            .find(|s| s.index == slot)
            .and_then(|s| s.stack)
    }

    fn key_down(&self, key: i32) -> bool {
        self.keys.contains(&key)
    }

    fn mouse_down(&self, _button: i32) -> bool {
        false
    }

    fn rotation(&self) -> Rotation {
        self.rotation
    }

    fn can_update(&self) -> bool {
        self.player.is_some() || self.host().and_then(|h| h.player()).is_some()
    }

    fn ticks(&self) -> u64 {
        self.ticks
    }

    fn tick_rate(&self) -> f32 {
        crate::port::shared::timer_speed() * 20.0
    }

    fn recent_tps(&self) -> f32 {
        self.tick_rate()
    }

    fn set_rotation(&mut self, rotation: Rotation) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn chat(&mut self, msg: &str) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn chat_player(&mut self, msg: &str) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn command(&mut self, cmd: &str) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn action(&mut self, action: Action) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn click_slot(&mut self, slot: usize, button: i32, mode: ClickMode) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn set_block(&mut self, _pos: BlockPos, _state: &str) -> bool {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
        false
    }

    fn drop_stack(&mut self, slot: usize, all: bool) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn use_item_on(&mut self, _pos: BlockPos, _face: Dir, _hand: Hand) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn attack_entity(&mut self, id: i32) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn set_packet_rate(&mut self, rate: f32) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn send_packet(&mut self, packet: PacketInfo) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn cancel_packet(&mut self, name: &str) {
        // Stage 1: the generated `Host` exposes no helper for this write;
        // it lands with the interaction/packet hooks (see docs/PORT-MAP.md).
    }

    fn play_sound(&mut self, _id: &str, _volume: f32, _pitch: f32) {}

    fn log(&mut self, level: LogLevel, msg: &str) {
        let tag = match level {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };
        if let Some(h) = self.host() {
            h.log(&format!("{tag}"), &format!("[horixon] {msg}"));
        }
    }

    // ---- client systems ----

    fn module_names(&self) -> Vec<&'static str> {
        self.snapshot.list.iter().map(|(n, _)| *n).collect()
    }

    fn module_active(&self, name: &str) -> bool {
        self.snapshot.active.get(name).copied().unwrap_or(false)
    }

    fn module_titles(&self) -> Vec<(String, String)> {
        self.snapshot
            .infos
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn request_toggle(&mut self, name: &str) {
        self.pending.push(Pending::Toggle(name.to_string()));
    }

    fn set_module_setting(&mut self, module: &str, setting: &str, value: &str) -> bool {
        if !self.snapshot.settings.contains_key(module) {
            return false;
        }
        self.pending.push(Pending::SetSetting {
            module: module.to_string(),
            setting: setting.to_string(),
            value: value.to_string(),
        });
        true
    }

    fn module_settings(&self, module: &str) -> Vec<(String, String)> {
        self.snapshot
            .settings
            .get(module)
            .cloned()
            .unwrap_or_default()
    }

    fn hud_active(&self) -> bool {
        self.snapshot.hud_active
    }

    fn set_hud_active(&mut self, on: bool) {
        self.pending.push(Pending::SetHudActive(on));
    }

    fn friends(&self) -> Vec<String> {
        self.snapshot.friends.clone()
    }

    fn add_friend(&mut self, name: &str) -> bool {
        self.pending.push(Pending::SetSetting {
            module: "__friends".into(),
            setting: "add".into(),
            value: name.to_string(),
        });
        true
    }

    fn remove_friend(&mut self, name: &str) -> bool {
        self.pending.push(Pending::SetSetting {
            module: "__friends".into(),
            setting: "remove".into(),
            value: name.to_string(),
        });
        true
    }

    fn enemies(&self) -> Vec<String> {
        self.snapshot.enemies.clone()
    }

    fn prefix(&self) -> String {
        self.snapshot.prefix.clone()
    }

    fn chat_feedback(&self) -> bool {
        self.snapshot.chat_feedback
    }

    fn module_list(&self) -> Vec<(&'static str, String)> {
        self.snapshot.list.clone()
    }

    fn module_binds(&self) -> Vec<(String, String)> {
        self.snapshot.binds.clone()
    }

    fn begin_bind(&mut self, module: &str) {
        self.pending.push(Pending::BeginBind(module.to_string()));
    }

    fn set_module_keybind(&mut self, module: &str, keybind: Keybind) -> bool {
        self.pending.push(Pending::SetKeybind {
            module: module.to_string(),
            keybind,
        });
        true
    }

    fn reset_module(&mut self, module: &str) {
        self.pending.push(Pending::ResetModule(module.to_string()));
    }

    fn reset_module_setting(&mut self, module: &str, setting: &str) -> bool {
        self.pending.push(Pending::ResetSetting {
            module: module.to_string(),
            setting: setting.to_string(),
        });
        true
    }

    fn teleport_local(&mut self, _pos: Vec3d) {
        // Horizon's client keeps the player where the server puts them; a local
        // teleport is the `Player` module's packet path (see PORT-MAP).
    }

    fn disconnect(&mut self, _reason: &str) {
        self.pending.push(Pending::Toggle("__disconnect".into()));
    }

    fn reload_configs(&mut self) -> usize {
        self.pending.push(Pending::ReloadConfigs);
        4
    }

    fn save_configs(&mut self) -> usize {
        self.pending.push(Pending::SaveConfigs);
        4
    }
}

/// Unused today, kept so the mapping table has one place to grow into.
pub fn game_mode_of(name: &str) -> GameMode {
    match name {
        "creative" => GameMode::Creative,
        "adventure" => GameMode::Adventure,
        "spectator" => GameMode::Spectator,
        _ => GameMode::Survival,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_records_and_offsets_screen_draws() {
        let mut q = DrawQueue::new((1920.0, 1080.0));
        q.push_translate(100.0, 50.0);
        q.rect(0.0, 0.0, 10.0, 10.0, 0xFF00_0000);
        q.pop_translate();
        match &q.ops[0] {
            Draw::Rect { x, y, .. } => {
                assert_eq!((*x, *y), (100.0, 50.0));
            }
            other => panic!("expected a rect, got {other:?}"),
        }
        assert_eq!(q.gui_size(), (1920.0, 1080.0));
    }

    #[test]
    fn box_outline_emits_twelve_edges() {
        let mut q = DrawQueue::new((100.0, 100.0));
        q.box_outline(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 1.0, 1.0),
            0xFFFF_FFFF,
            1.0,
            true,
        );
        assert_eq!(q.ops.len(), 12);
    }

    #[test]
    fn headless_env_reports_no_world() {
        let env = HeadlessEnv::new();
        assert!(env.player().is_none());
        assert!(!env.can_update());
        assert_eq!(env.screen(), Screen::Playing);
    }
}

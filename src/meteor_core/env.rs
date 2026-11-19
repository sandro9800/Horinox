//! The game-facing contract.
//!
//! Meteor's modules call straight into Minecraft (`mc.player`, `mc.level`, the
//! packet classes). Horizon's native mods instead reach the game through the
//! `Host` helpers and a fixed set of hook points, so the port puts one trait -
//! [`Env`] - between them: modules are written against `Env`, and
//! `meteor-mod`'s `HorizonEnv` implements it by calling `horizon_client`'s
//! helpers. That keeps every module engine-agnostic and unit-testable, and
//! confines the Horizon mapping to one file.

pub use crate::meteor_core::math::Dir;
use crate::meteor_core::math::{BlockPos, Rotation, Vec3d};

/// `net.minecraft.world.level.GameType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl GameMode {
    pub const fn name(self) -> &'static str {
        match self {
            GameMode::Survival => "survival",
            GameMode::Creative => "creative",
            GameMode::Adventure => "adventure",
            GameMode::Spectator => "spectator",
        }
    }
}

/// What a module reads of the local player. Fields mirror the vanilla getters
/// Meteor calls, so a module ports over one-for-one.
#[derive(Clone, Debug, Default)]
pub struct PlayerInfo {
    pub name: String,
    pub uuid: [u8; 16],
    /// Feet position this tick.
    pub pos: Vec3d,
    /// Feet position last tick (interpolation, breadcrumbs).
    pub prev_pos: Vec3d,
    pub velocity: Vec3d,
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub on_ground: bool,
    pub in_water: bool,
    pub in_lava: bool,
    pub sneaking: bool,
    pub sprinting: bool,
    pub using_item: bool,
    pub blocking: bool,
    pub glowing: bool,
    pub health: f32,
    pub max_health: f32,
    pub absorption: f32,
    pub armor: i32,
    pub food: i32,
    pub saturation: f32,
    pub air: i32,
    pub max_air: i32,
    pub xp_level: i32,
    pub xp_progress: f32,
    pub selected_slot: usize,
    pub game_mode: GameMode,
    pub dimension: String,
    pub eye_height: f32,
}

impl PlayerInfo {
    /// `Player.getEyePos()`.
    pub fn eye_pos(&self) -> Vec3d {
        Vec3d::new(self.pos.x, self.pos.y + self.eye_height as f64, self.pos.z)
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    pub fn rotation(&self) -> Rotation {
        Rotation::new(self.yaw, self.pitch)
    }
}

/// A dropped item or an inventory stack, as the ESP/HUD modules read them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemStackInfo {
    /// `minecraft:diamond_sword`.
    pub id: String,
    pub count: i32,
    /// `minecraft:custom_name`'s text, if any.
    pub name: Option<String>,
    /// Damage taken, for tools and armour.
    pub damage: Option<i32>,
    /// `(enchantment id, level)`.
    pub enchantments: Vec<(String, u32)>,
}

impl ItemStackInfo {
    pub fn is_empty(&self) -> bool {
        self.id.is_empty() || self.id == "minecraft:air" || self.count <= 0
    }

    /// The display name Meteor's HUD shows: custom name, else a titleised id.
    pub fn display_name(&self) -> String {
        match &self.name {
            Some(n) => n.clone(),
            None => crate::meteor_core::util::name_to_title(
                self.id
                    .strip_prefix("minecraft:")
                    .unwrap_or(&self.id)
                    .replace('_', "-")
                    .as_str(),
            ),
        }
    }

    pub fn enchant_level(&self, id: &str) -> u32 {
        self.enchantments
            .iter()
            .find(|(e, _)| e == id || e.strip_prefix("minecraft:") == Some(id))
            .map(|(_, l)| *l)
            .unwrap_or(0)
    }
}

/// Every entity the client knows, mobs and other players alike - the shape of
/// Horizon's `native_client_entities()` plus the fields the combat modules need.
#[derive(Clone, Debug, Default)]
pub struct EntityInfo {
    pub id: i32,
    pub kind: String,
    pub uuid: [u8; 16],
    pub name: Option<String>,
    pub pos: Vec3d,
    pub prev_pos: Vec3d,
    pub velocity: Vec3d,
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub health: f32,
    pub max_health: f32,
    pub distance: f64,
    pub alive: bool,
    pub invisible: bool,
    pub baby: bool,
    pub is_player: bool,
    pub is_mob: bool,
    pub is_item: bool,
    /// A player's ping, for the nametag/tab modules.
    pub ping: i32,
    pub item: Option<ItemStackInfo>,
    /// Armour and hands, in `feet, legs, chest, head, main, off` order.
    pub equipment: [Option<ItemStackInfo>; 6],
    pub on_fire: bool,
    pub sneaking: bool,
    pub using_item: bool,
    pub blocking: bool,
}

impl EntityInfo {
    pub fn eye_pos(&self) -> Vec3d {
        Vec3d::new(self.pos.x, self.pos.y + 1.62, self.pos.z)
    }

    pub fn bounding_box(&self) -> (Vec3d, Vec3d) {
        let (w, h) = if self.is_player {
            (0.6, 1.8)
        } else {
            (0.6, 1.8)
        };
        (
            Vec3d::new(self.pos.x - w / 2.0, self.pos.y, self.pos.z - w / 2.0),
            Vec3d::new(self.pos.x + w / 2.0, self.pos.y + h, self.pos.z + w / 2.0),
        )
    }

    pub fn short_kind(&self) -> &str {
        self.kind.strip_prefix("minecraft:").unwrap_or(&self.kind)
    }
}

/// `mc.level` data the world and render modules read.
#[derive(Clone, Debug, Default)]
pub struct WorldInfo {
    pub dimension: String,
    /// `Level.getGameTime()`.
    pub game_time: i64,
    /// `Level.getDayTime()`.
    pub day_time: i64,
    pub raining: bool,
    pub thundering: bool,
    pub rain_level: f32,
    pub thunder_level: f32,
    pub min_y: i32,
    pub height: i32,
    pub difficulty: String,
}

/// A block state, as the world reads and writes it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockInfo {
    /// The full state string, `minecraft:oak_stairs[facing=east,half=bottom]`.
    pub state: String,
    /// The block id alone.
    pub id: String,
    /// Parsed properties.
    pub properties: Vec<(String, String)>,
}

impl BlockInfo {
    pub fn new(state: &str) -> Self {
        let (id, properties) = match state.split_once('[') {
            Some((id, rest)) => {
                let rest = rest.trim_end_matches(']');
                let props = rest
                    .split(',')
                    .filter_map(|kv| kv.split_once('='))
                    .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                    .collect();
                (id.trim().to_string(), props)
            }
            None => (state.trim().to_string(), Vec::new()),
        };
        Self {
            state: state.to_string(),
            id,
            properties,
        }
    }

    pub fn prop(&self, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn is(&self, id: &str) -> bool {
        self.id == id || self.id.strip_prefix("minecraft:") == Some(id)
    }

    pub fn is_air(&self) -> bool {
        self.is("minecraft:air") || self.is("air") || self.is("cave_air") || self.is("void_air")
    }
}

/// `mc.hitResult` - what the crosshair is on.
#[derive(Clone, Debug)]
pub enum Raycast {
    Block {
        pos: BlockPos,
        face: Dir,
        inside: bool,
        distance: f32,
    },
    Entity {
        id: i32,
        pos: Vec3d,
        distance: f32,
    },
    Miss,
}

impl Raycast {
    pub fn block_pos(&self) -> Option<BlockPos> {
        match self {
            Raycast::Block { pos, .. } => Some(*pos),
            _ => None,
        }
    }
}

/// `mc.screen`'s classification, for the modules that gate on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Screen {
    /// No screen: the world is being played.
    #[default]
    Playing,
    Chat,
    Death,
    /// Any other screen - inventory, container, options, our own GUI.
    Other,
}

/// A packet the module set may veto or rewrite. Horizon exposes decoded play
/// packets (`native_client_packet` / `native_server_packet`), so the port keeps
/// the same two directions and a name plus a small typed payload.
#[derive(Clone, Debug)]
pub struct PacketInfo {
    pub name: String,
    pub clientbound: bool,
    /// The packet's own fields, as `(key, value)` text, for the logger and the
    /// packet-matching settings.
    pub fields: Vec<(String, String)>,
}

/// Generic player actions Meteor's anti-cheat and automation modules fire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Attack,
    Use,
    Jump,
    Sneak(bool),
    Sprint(bool),
    DropStack,
    SwapHands,
    PickBlock,
    StopUsingItem,
}

/// Where a container slot click goes (`AbstractContainerMenu.clicked`'s mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickMode {
    Pickup,
    QuickMove,
    Swap,
    Clone,
    Throw,
    QuickCraft,
    PickupAll,
}

/// The hand a use/attack happens with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Main,
    Off,
}

/// One inventory slot as the HUD and `inventory` modules read it.
#[derive(Clone, Debug, Default)]
pub struct InvSlot {
    pub index: usize,
    pub stack: Option<ItemStackInfo>,
}

/// The render surface modules draw on.
///
/// `meteor-mod` implements this over Horizon's `native_world_primitives`
/// (3D) and the GUI batch (2D). Colours are ARGB, distances in blocks.
pub trait Renderer {
    // ---- 3D (world space) ----
    /// A line `width` pixels wide, seen through walls when `through_walls`.
    fn line(&mut self, from: Vec3d, to: Vec3d, color: u32, width: f32, through_walls: bool);
    fn box_outline(&mut self, min: Vec3d, max: Vec3d, color: u32, width: f32, through_walls: bool);
    fn filled_box(&mut self, min: Vec3d, max: Vec3d, color: u32, through_walls: bool);
    /// Camera-facing text at a world position; `scale` 1 is a name tag's size.
    fn text3d(&mut self, at: Vec3d, text: &str, color: u32, scale: f32, through_walls: bool);

    // ---- 2D (screen space, GUI pixels) ----
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: u32);
    fn rect_outline(&mut self, x: f64, y: f64, w: f64, h: f64, color: u32, width: f64);
    fn text2d(&mut self, x: f64, y: f64, text: &str, color: u32, scale: f64);
    fn item2d(&mut self, x: f64, y: f64, stack: &ItemStackInfo, size: f64);
    /// Text width in GUI pixels for the current font and scale.
    fn text_width(&self, text: &str, scale: f64) -> f64;
    /// The GUI size in pixels.
    fn gui_size(&self) -> (f64, f64);
}

/// A drawing pass that ignores its input - for headless tests and for the
/// editor previews that only need geometry validated.
#[derive(Default)]
pub struct NullRenderer {
    pub lines: usize,
    pub boxes: usize,
    pub texts: usize,
}

impl Renderer for NullRenderer {
    fn line(&mut self, _: Vec3d, _: Vec3d, _: u32, _: f32, _: bool) {
        self.lines += 1;
    }
    fn box_outline(&mut self, _: Vec3d, _: Vec3d, _: u32, _: f32, _: bool) {
        self.boxes += 1;
    }
    fn filled_box(&mut self, _: Vec3d, _: Vec3d, _: u32, _: bool) {
        self.boxes += 1;
    }
    fn text3d(&mut self, _: Vec3d, _: &str, _: u32, _: f32, _: bool) {
        self.texts += 1;
    }
    fn rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: u32) {}
    fn rect_outline(&mut self, _: f64, _: f64, _: f64, _: f64, _: u32, _: f64) {}
    fn text2d(&mut self, _: f64, _: f64, _: &str, _: u32, _: f64) {
        self.texts += 1;
    }
    fn item2d(&mut self, _: f64, _: f64, _: &ItemStackInfo, _: f64) {}
    fn text_width(&self, text: &str, scale: f64) -> f64 {
        text.chars().count() as f64 * 6.0 * scale
    }
    fn gui_size(&self) -> (f64, f64) {
        (1920.0, 1080.0)
    }
}

/// Everything a module may do to the game, and everything it may read.
///
/// One implementor exists in the mod (`HorizonEnv`) and one in tests
/// (`TestEnv`); modules take `&mut dyn Env` and never see a Horizon type.
pub trait Env {
    // ---- reads ----
    fn player(&self) -> Option<PlayerInfo>;
    fn world(&self) -> Option<WorldInfo>;
    fn entities(&self) -> Vec<EntityInfo>;
    /// The entity the crosshair is on, if any.
    fn crosshair_entity(&self) -> Option<EntityInfo>;
    fn raycast(&self, max_distance: f32) -> Raycast;
    fn block(&self, pos: BlockPos) -> Option<BlockInfo>;
    fn block_light(&self, pos: BlockPos) -> (u8, u8);
    fn screen(&self) -> Screen;
    /// UTC seconds, for the timers that must not drift with the game clock.
    fn now(&self) -> f64;
    /// The player's inventory, `0..9` the hotbar, `9..36` main, then armour and
    /// the off hand.
    fn inventory(&self) -> Vec<InvSlot>;
    fn selected_stack(&self) -> Option<ItemStackInfo>;
    fn key_down(&self, key: i32) -> bool;
    fn mouse_down(&self, button: i32) -> bool;
    /// The client's own rotation state (Meteor's `Rotation` module + `Rotations`).
    fn rotation(&self) -> Rotation;
    /// `Utils.canUpdate()`: in a world, not in a screen that pauses the client.
    fn can_update(&self) -> bool;
    /// Ticks the client has run, for the timers that count in ticks.
    fn ticks(&self) -> u64;
    /// The ms-per-tick the client is currently running at (Timer module).
    fn tick_rate(&self) -> f32;
    fn recent_tps(&self) -> f32;

    // ---- writes ----
    fn set_rotation(&mut self, rotation: Rotation);
    fn chat(&mut self, msg: &str);
    fn chat_player(&mut self, msg: &str);
    fn command(&mut self, cmd: &str);
    fn action(&mut self, action: Action);
    fn click_slot(&mut self, slot: usize, button: i32, mode: ClickMode);
    fn set_block(&mut self, pos: BlockPos, state: &str) -> bool;
    fn drop_stack(&mut self, slot: usize, all: bool);
    fn use_item_on(&mut self, pos: BlockPos, face: Dir, hand: Hand);
    fn attack_entity(&mut self, id: i32);
    fn set_packet_rate(&mut self, rate: f32);
    /// Ask the client to send one raw play packet (Meteor's packet modules).
    fn send_packet(&mut self, packet: PacketInfo);
    fn cancel_packet(&mut self, name: &str);
    fn play_sound(&mut self, id: &str, volume: f32, pitch: f32);
    fn log(&mut self, level: LogLevel, msg: &str);

    // ---- client systems ---------------------------------------------------
    //
    // A module that reads or toggles another module does it through these, which
    // the mod answers from its own registry. Java calls
    // `Modules.get().get(KillAura.class)` directly; going through the `Env`
    // keeps the dispatch free of aliasing borrows and gives every module a test
    // double for free.

    /// Every module's `name`, for command completion.
    fn module_names(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// `Modules.get().isActive(...)`.
    fn module_active(&self, _name: &str) -> bool {
        false
    }

    /// `(name, infoString)` for every active module, for `ActiveModulesHud`.
    fn module_titles(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// `Modules.toggle(...)`, applied after the current dispatch.
    fn request_toggle(&mut self, _name: &str) {}

    /// `Setting.parse` on another module's setting, for `KillAura`-style
    /// two-way settings.
    fn set_module_setting(&mut self, _module: &str, _setting: &str, _value: &str) -> bool {
        false
    }

    /// A module's settings as `(name, display)` pairs, for `ModulesHud`.
    fn module_settings(&self, _module: &str) -> Vec<(String, String)> {
        Vec::new()
    }

    /// `Hud.get().active`.
    fn hud_active(&self) -> bool {
        true
    }

    fn set_hud_active(&mut self, _on: bool) {}

    /// Friend names or UUIDs, for `Friends` checks in the combat modules.
    fn friends(&self) -> Vec<String> {
        Vec::new()
    }

    fn add_friend(&mut self, _name: &str) -> bool {
        false
    }

    fn remove_friend(&mut self, _name: &str) -> bool {
        false
    }

    fn is_friend(&self, name: &str) -> bool {
        self.friends().iter().any(|f| f.eq_ignore_ascii_case(name))
    }

    fn enemies(&self) -> Vec<String> {
        Vec::new()
    }

    fn is_enemy(&self, name: &str) -> bool {
        self.enemies().iter().any(|e| e.eq_ignore_ascii_case(name))
    }

    /// `Config.prefix` - the client's command prefix (`.` by default).
    fn prefix(&self) -> String {
        ".".to_string()
    }

    /// `Config.chatFeedback`.
    fn chat_feedback(&self) -> bool {
        true
    }

    /// `(module name, category)` for every module, for the `modules` command.
    fn module_list(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    /// `(keybind text, module title)` pairs, for `binds` and `BindsHud`.
    fn module_binds(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// Enters "press a key" mode for a module (`Modules.setModuleToBind`).
    fn begin_bind(&mut self, _module: &str) {}

    /// Binds a module to a key without the interactive flow.
    fn set_module_keybind(
        &mut self,
        _module: &str,
        _keybind: crate::meteor_core::util::Keybind,
    ) -> bool {
        false
    }

    /// `Settings.reset()` on one module.
    fn reset_module(&mut self, _module: &str) {}

    /// `Setting.reset()` for one setting of one module.
    fn reset_module_setting(&mut self, _module: &str, _setting: &str) -> bool {
        false
    }

    // ---- client actions the commands reach ----
    /// A local position change (`hclip`/`vclip` - Meteor sets the position
    /// packet's coordinates).
    fn teleport_local(&mut self, _pos: Vec3d) {}

    /// Leaves the current server (`disconnect`).
    fn disconnect(&mut self, _reason: &str) {}

    /// `Systems.load()`; returns how many systems reloaded.
    fn reload_configs(&mut self) -> usize {
        0
    }

    /// `Systems.save()`; returns how many systems saved.
    fn save_configs(&mut self) -> usize {
        0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

pub mod test_env {
    //! A deterministic `Env` for module tests: flat world, no entities, every
    //! write recorded so a test can assert on the intent.

    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    pub struct TestEnv {
        pub player: Option<PlayerInfo>,
        pub world: Option<WorldInfo>,
        pub entities: Vec<EntityInfo>,
        pub blocks: HashMap<BlockPos, BlockInfo>,
        pub rotation: Rotation,
        // ---- recorded writes ----
        pub chats: Vec<String>,
        pub commands: Vec<String>,
        pub actions: Vec<Action>,
        pub packets: Vec<PacketInfo>,
        pub clicks: Vec<(usize, i32, ClickMode)>,
        pub placed: Vec<(BlockPos, String)>,
        pub packet_rates: Vec<f32>,
        pub rotations: Vec<Rotation>,
        pub attacks: Vec<i32>,
        pub uses: Vec<(BlockPos, Dir, Hand)>,
        pub drops: Vec<(usize, bool)>,
        pub sounds: Vec<String>,
        pub cancelled: Vec<String>,
        pub toggles: Vec<String>,
        pub setting_writes: Vec<(String, String, String)>,
        pub resets: Vec<(String, Option<String>)>,
        pub keybinds: Vec<(String, crate::meteor_core::util::Keybind)>,
        pub teleports: Vec<Vec3d>,
        pub disconnects: Vec<String>,
        pub saves: usize,
        pub reloads: usize,
        pub logs: Vec<String>,
        // ---- state a test sets ----
        pub now: f64,
        pub ticks: u64,
        pub keys: Vec<i32>,
        pub inventory: Vec<InvSlot>,
        pub screen: Screen,
        pub hud_active: bool,
        pub active_modules: Vec<&'static str>,
        pub module_names: Vec<&'static str>,
        pub module_settings: HashMap<String, Vec<(String, String)>>,
        pub friends: Vec<String>,
        pub enemies: Vec<String>,
    }

    impl TestEnv {
        pub fn in_world() -> Self {
            let mut e = TestEnv {
                hud_active: true,
                ..Default::default()
            };
            e.player = Some(PlayerInfo {
                name: "Dev".into(),
                pos: Vec3d::new(0.5, 64.0, 0.5),
                prev_pos: Vec3d::new(0.5, 64.0, 0.5),
                yaw: 0.0,
                pitch: 0.0,
                health: 20.0,
                max_health: 20.0,
                food: 20,
                max_air: 300,
                air: 300,
                dimension: "minecraft:overworld".into(),
                on_ground: true,
                eye_height: 1.62,
                ..Default::default()
            });
            e.world = Some(WorldInfo {
                dimension: "minecraft:overworld".into(),
                min_y: -64,
                height: 384,
                ..Default::default()
            });
            e.screen = Screen::Playing;
            e
        }

        pub fn push_block(&mut self, pos: BlockPos, id: &str) {
            self.blocks.insert(pos, BlockInfo::new(id));
        }

        pub fn push_entity(&mut self, entity: EntityInfo) {
            self.entities.push(entity);
        }

        /// The last rotation the modules asked for, if any.
        pub fn last_rotation(&self) -> Option<Rotation> {
            self.rotations.last().copied()
        }

        /// Whether a packet with this name was cancelled.
        pub fn cancelled_packet(&self, name: &str) -> bool {
            self.cancelled.iter().any(|n| n == name)
        }
    }

    impl Env for TestEnv {
        fn player(&self) -> Option<PlayerInfo> {
            self.player.clone()
        }
        fn world(&self) -> Option<WorldInfo> {
            self.world.clone()
        }
        fn entities(&self) -> Vec<EntityInfo> {
            self.entities.clone()
        }
        fn crosshair_entity(&self) -> Option<EntityInfo> {
            self.entities.first().cloned()
        }
        fn raycast(&self, max_distance: f32) -> Raycast {
            let p = match self.player() {
                Some(p) => p,
                None => return Raycast::Miss,
            };
            let dir = self.rotation.direction();
            let mut t = 0.0f64;
            while t < max_distance as f64 {
                let pos = p.eye_pos().add(dir.mul(t));
                let bp = pos.to_block_pos();
                if let Some(b) = self.blocks.get(&bp) {
                    if !b.is_air() {
                        return Raycast::Block {
                            pos: bp,
                            face: Dir::Up,
                            inside: false,
                            distance: t as f32,
                        };
                    }
                }
                t += 0.25;
            }
            Raycast::Miss
        }
        fn block(&self, pos: BlockPos) -> Option<BlockInfo> {
            Some(
                self.blocks
                    .get(&pos)
                    .cloned()
                    .unwrap_or_else(|| BlockInfo::new("minecraft:air")),
            )
        }
        fn block_light(&self, _: BlockPos) -> (u8, u8) {
            (15, 15)
        }
        fn screen(&self) -> Screen {
            self.screen
        }
        fn now(&self) -> f64 {
            self.now
        }
        fn inventory(&self) -> Vec<InvSlot> {
            self.inventory.clone()
        }
        fn selected_stack(&self) -> Option<ItemStackInfo> {
            self.inventory
                .iter()
                .find(|s| s.index == self.player.as_ref().map(|p| p.selected_slot).unwrap_or(0))
                .and_then(|s| s.stack.clone())
        }
        fn key_down(&self, key: i32) -> bool {
            self.keys.contains(&key)
        }
        fn mouse_down(&self, button: i32) -> bool {
            self.keys.contains(&(1000 + button))
        }
        fn rotation(&self) -> Rotation {
            self.rotation
        }
        fn can_update(&self) -> bool {
            self.player.is_some()
        }
        fn ticks(&self) -> u64 {
            self.ticks
        }
        fn tick_rate(&self) -> f32 {
            20.0
        }
        fn recent_tps(&self) -> f32 {
            20.0
        }
        fn set_rotation(&mut self, rotation: Rotation) {
            self.rotation = rotation;
            self.rotations.push(rotation);
        }
        fn chat(&mut self, msg: &str) {
            self.chats.push(msg.to_string());
        }
        fn chat_player(&mut self, msg: &str) {
            self.chats.push(msg.to_string());
        }
        fn command(&mut self, cmd: &str) {
            self.commands.push(cmd.to_string());
        }
        fn action(&mut self, action: Action) {
            self.actions.push(action);
        }
        fn click_slot(&mut self, slot: usize, button: i32, mode: ClickMode) {
            self.clicks.push((slot, button, mode));
        }
        fn set_block(&mut self, pos: BlockPos, state: &str) -> bool {
            self.placed.push((pos, state.to_string()));
            self.blocks.insert(pos, BlockInfo::new(state));
            true
        }
        fn drop_stack(&mut self, slot: usize, all: bool) {
            self.drops.push((slot, all));
        }
        fn use_item_on(&mut self, pos: BlockPos, face: Dir, hand: Hand) {
            self.uses.push((pos, face, hand));
        }
        fn attack_entity(&mut self, id: i32) {
            self.attacks.push(id);
        }
        fn set_packet_rate(&mut self, rate: f32) {
            self.packet_rates.push(rate);
        }
        fn send_packet(&mut self, packet: PacketInfo) {
            self.packets.push(packet);
        }
        fn cancel_packet(&mut self, name: &str) {
            self.cancelled.push(name.to_string());
        }
        fn play_sound(&mut self, id: &str, _volume: f32, _pitch: f32) {
            self.sounds.push(id.to_string());
        }
        fn log(&mut self, _level: LogLevel, msg: &str) {
            self.logs.push(msg.to_string());
        }

        // ---- client systems ----
        fn module_names(&self) -> Vec<&'static str> {
            if self.module_names.is_empty() {
                vec!["test-module"]
            } else {
                self.module_names.clone()
            }
        }
        fn module_active(&self, name: &str) -> bool {
            self.active_modules.iter().any(|m| *m == name)
        }
        fn request_toggle(&mut self, name: &str) {
            self.toggles.push(name.to_string());
        }
        fn module_titles(&self) -> Vec<(String, String)> {
            self.active_modules
                .iter()
                .map(|m| (m.to_string(), String::new()))
                .collect()
        }
        fn set_module_setting(&mut self, module: &str, setting: &str, value: &str) -> bool {
            self.setting_writes
                .push((module.to_string(), setting.to_string(), value.to_string()));
            true
        }
        fn module_settings(&self, module: &str) -> Vec<(String, String)> {
            self.module_settings
                .get(module)
                .cloned()
                .unwrap_or_default()
        }
        fn hud_active(&self) -> bool {
            self.hud_active
        }
        fn set_hud_active(&mut self, on: bool) {
            self.hud_active = on;
        }
        fn friends(&self) -> Vec<String> {
            self.friends.clone()
        }
        fn add_friend(&mut self, name: &str) -> bool {
            if self.friends.iter().any(|f| f == name) {
                return false;
            }
            self.friends.push(name.to_string());
            true
        }
        fn remove_friend(&mut self, name: &str) -> bool {
            let before = self.friends.len();
            self.friends.retain(|f| f != name);
            self.friends.len() != before
        }
        fn enemies(&self) -> Vec<String> {
            self.enemies.clone()
        }
        fn module_list(&self) -> Vec<(&'static str, String)> {
            self.module_names()
                .into_iter()
                .map(|n| (n, "Misc".to_string()))
                .collect()
        }
        fn module_binds(&self) -> Vec<(String, String)> {
            Vec::new()
        }
        fn begin_bind(&mut self, module: &str) {
            self.keybinds.push((
                module.to_string(),
                crate::meteor_core::util::Keybind::none(),
            ));
        }
        fn set_module_keybind(
            &mut self,
            module: &str,
            keybind: crate::meteor_core::util::Keybind,
        ) -> bool {
            self.keybinds.push((module.to_string(), keybind));
            true
        }
        fn reset_module(&mut self, module: &str) {
            self.resets.push((module.to_string(), None));
        }
        fn reset_module_setting(&mut self, module: &str, setting: &str) -> bool {
            self.resets
                .push((module.to_string(), Some(setting.to_string())));
            true
        }
        fn teleport_local(&mut self, pos: Vec3d) {
            self.teleports.push(pos);
        }
        fn disconnect(&mut self, reason: &str) {
            self.disconnects.push(reason.to_string());
        }
        fn reload_configs(&mut self) -> usize {
            self.reloads += 1;
            4
        }
        fn save_configs(&mut self) -> usize {
            self.saves += 1;
            4
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_state_parses_properties() {
        let b = BlockInfo::new("minecraft:oak_stairs[facing=east,half=bottom]");
        assert_eq!(b.id, "minecraft:oak_stairs");
        assert_eq!(b.prop("facing"), Some("east"));
        assert!(b.is("oak_stairs"));
        assert!(!b.is_air());
        assert!(BlockInfo::new("minecraft:air").is_air());
    }

    #[test]
    fn item_display_name() {
        let i = ItemStackInfo {
            id: "minecraft:golden_apple".into(),
            count: 1,
            ..Default::default()
        };
        assert_eq!(i.display_name(), "Golden Apple");
        assert!(!i.is_empty());
        assert!(ItemStackInfo::default().is_empty());
    }

    #[test]
    fn test_env_raycasts_the_first_solid_block() {
        let mut e = test_env::TestEnv::in_world();
        // The eye sits at y 65.62, so a level ray meets the block at y 65.
        e.push_block(BlockPos::new(0, 65, 2), "minecraft:stone");
        e.player.as_mut().unwrap().pitch = 0.0;
        match e.raycast(5.0) {
            Raycast::Block { pos, .. } => assert_eq!(pos, BlockPos::new(0, 65, 2)),
            other => panic!("expected a block hit, got {other:?}"),
        }
    }
}

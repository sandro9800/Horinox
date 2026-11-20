//! The port's process-wide state, and the dispatch that keeps borrows honest.
//!
//! Java's `MeteorClient` holds static systems and its modules reach them through
//! `Systems.get(...)`. A Horizon mod is one trait object shared by the render and
//! server threads, so the same systems live here behind one `Mutex`, and the
//! dispatch is arranged so a module never sees the registry it is being run from:
//!
//! 1. [`Snapshot`] copies the cheap facts modules read about each other
//!    (names, active flags, info strings, settings text) into one immutable map.
//! 2. [`HorizonEnv`] borrows that snapshot plus a queue of side effects; it holds
//!    no lock at all.
//! 3. After the dispatch, [`Shared::apply`] carries out the queued toggles and
//!    setting writes.
//!
//! That is what makes `.toggle`, `Modules.get(...)`-style reads and the keybind
//! flow safe from every thread without interior mutability inside a module.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

/// The Timer module's multiplier: the mod reads it when it schedules the next
/// client tick (`TIMER_SPEED * 20` ticks a second, 1.0 at vanilla speed).
pub static TIMER_SPEED: AtomicU32 = AtomicU32::new(0x3F80_0000); // 1.0f32 bits

pub fn timer_speed() -> f32 {
    f32::from_bits(TIMER_SPEED.load(Ordering::Relaxed))
}

pub fn set_timer_speed(v: f32) {
    TIMER_SPEED.store(v.to_bits(), Ordering::Relaxed);
}

use crate::meteor_core::category::Category;
use crate::meteor_core::command::{builtin_commands, CommandManager};
use crate::meteor_core::config::ConfigFile;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::friends::Friends;
use crate::meteor_core::hud::Hud;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::Module;
use crate::meteor_core::setting::Settings;
use crate::meteor_core::theme::MeteorTheme;
use crate::meteor_core::util::Keybind;

/// Everything a module may read about its neighbours, refreshed once a tick.
#[derive(Clone, Default)]
pub struct Snapshot {
    /// `(name, category name)` for the `modules` command.
    pub list: Vec<(&'static str, String)>,
    /// `name -> active`.
    pub active: BTreeMap<&'static str, bool>,
    /// `name -> infoString`, for `ActiveModulesHud`.
    pub infos: BTreeMap<&'static str, String>,
    /// `name -> (setting name, display)` for `.setting <module> <name>`.
    pub settings: BTreeMap<&'static str, Vec<(String, String)>>,
    /// `(keybind, module title)` for `binds`.
    pub binds: Vec<(String, String)>,
    /// `name -> title`.
    pub titles: BTreeMap<&'static str, String>,
    pub hud_active: bool,
    pub friends: Vec<String>,
    pub enemies: Vec<String>,
    pub prefix: String,
    pub chat_feedback: bool,
}

impl Snapshot {
    pub fn build(
        modules: &Modules,
        hud: &Hud,
        friends: &Friends,
        prefix: &str,
        feedback: bool,
    ) -> Self {
        let mut s = Snapshot {
            hud_active: hud.active,
            prefix: prefix.to_string(),
            chat_feedback: feedback,
            friends: friends.names(),
            enemies: friends.enemy_names(),
            ..Default::default()
        };
        for m in modules.all() {
            let base = m.base();
            s.list.push((base.name, base.category.name().to_string()));
            s.active.insert(base.name, modules.is_active(base.name));
            s.titles.insert(base.name, base.title.clone());
            if let Some(info) = m.info_string() {
                s.infos.insert(base.name, info);
            }
            let rows: Vec<(String, String)> = base
                .settings
                .iter()
                .filter(|x| x.is_visible())
                .map(|x| (x.name.clone(), x.display()))
                .collect();
            s.settings.insert(base.name, rows);
            if base.keybind.is_set() {
                s.binds.push((base.keybind.to_string(), base.title.clone()));
            }
        }
        s.binds.sort();
        s
    }

    fn setting_names_for_module(&self, module: &str) -> Vec<(String, String)> {
        self.settings
            .get(module)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect()
    }
}

/// A side effect a module asked for during its tick, carried out afterwards.
#[derive(Clone, Debug)]
pub enum Pending {
    Toggle(String),
    SetSetting {
        module: String,
        setting: String,
        value: String,
    },
    ResetSetting {
        module: String,
        setting: String,
    },
    ResetModule(String),
    BeginBind(String),
    SetKeybind {
        module: String,
        keybind: Keybind,
    },
    SetHudActive(bool),
    SaveConfigs,
    ReloadConfigs,
}

/// `MeteorClient`'s systems.
pub struct Shared {
    pub modules: Modules,
    pub hud: Hud,
    pub commands: CommandManager,
    pub friends: Friends,
    pub theme: MeteorTheme,
    pub config: Config,
    pub snapshot: Snapshot,
    pub pending: Vec<Pending>,
    /// The module the bind flow is waiting on.
    pub binding: Option<String>,
    pub awaiting_key_release: bool,
    /// Set while a world is joined, so `Utils.canUpdate()`-style gates agree.
    pub in_world: bool,
    pub ticks: u64,
    // Runtime knobs the modules set and the mod applies to the game:
    pub timer_speed: f32,
    /// Rotations the mod should apply this tick, `(yaw, pitch, priority)`.
    pub rotation_request: Option<(f32, f32)>,
    pub packets_blocked: Vec<String>,
    pub world_prev_pos: Option<crate::meteor_core::math::Vec3d>,
}

/// The client's own settings (`systems.config.Config`).
pub struct Config {
    pub prefix: String,
    pub chat_feedback: bool,
    pub folder: PathBuf,
    pub settings: Settings,
}

impl Default for Config {
    fn default() -> Self {
        let mut settings = Settings::new();
        let g = settings.default_group();
        g.string_setting("prefix", ".");
        g.bool_setting("chat-feedback", true);
        g.bool_setting("custom-font", true);
        Self {
            prefix: ".".to_string(),
            chat_feedback: true,
            folder: default_folder(),
            settings,
        }
    }
}

fn default_folder() -> PathBuf {
    // `native_data_dir(mod_id)` is Horizon's per-mod folder; the port falls back
    // to the process's current folder for a headless run.
    std::env::var_os("HORIZON_MOD_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("horixon"))
}

impl Shared {
    pub fn new() -> Self {
        let mut shared = Self {
            modules: Modules::new(),
            hud: Hud::new(),
            commands: builtin_commands(),
            friends: Friends::new(),
            theme: MeteorTheme::new(),
            config: Config::default(),
            snapshot: Snapshot::default(),
            pending: Vec::new(),
            binding: None,
            awaiting_key_release: false,
            in_world: false,
            ticks: 0,
            timer_speed: 1.0,
            rotation_request: None,
            packets_blocked: Vec::new(),
            world_prev_pos: None,
        };
        crate::port::modules::register(&mut shared);
        crate::port::hud::register(&mut shared.hud);
        shared.refresh_snapshot();
        shared
    }

    pub fn refresh_snapshot(&mut self) {
        self.snapshot = Snapshot::build(
            &self.modules,
            &self.hud,
            &self.friends,
            &self.config.prefix.clone(),
            self.config.chat_feedback,
        );
    }

    /// Runs every active module's tick.
    pub fn tick(
        &mut self,
        host: &mut dyn crate::horizon_api_shim::mod_api::ClientHost,
        player: Option<crate::meteor_core::env::PlayerInfo>,
        world: Option<crate::meteor_core::env::WorldInfo>,
    ) {
        self.ticks += 1;
        self.refresh_snapshot();
        let Shared {
            modules,
            snapshot,
            pending,
            ticks,
            ..
        } = self;
        let mut env = crate::port::env_impl::HorizonEnv::new(host, snapshot, pending, *ticks);
        env.set_player(player);
        env.set_world(world);
        modules.tick(&mut env);
        let _ = env;
        self.apply_pending(host);
    }

    /// World-space drawing for one frame.
    pub fn render_world(&mut self, queue: &mut crate::port::env_impl::DrawQueue) {
        self.refresh_snapshot();
        let Shared {
            modules,
            snapshot,
            pending,
            ticks,
            ..
        } = self;
        let mut env = crate::port::env_impl::HorizonEnv::new_headless(snapshot, pending, *ticks);
        modules.render_world(&mut env, queue);
    }

    /// Screen-space drawing for one frame, after the HUD.
    pub fn render_2d(&mut self, queue: &mut crate::port::env_impl::DrawQueue, env: &mut dyn Env) {
        self.modules.render_2d(env, queue);
    }

    pub fn render_hud(&mut self, queue: &mut crate::port::env_impl::DrawQueue, env: &mut dyn Env) {
        self.modules.render_hud(env, queue);
    }

    /// Carries out what modules queued during the dispatch.
    pub fn apply_pending(&mut self, host: &mut dyn crate::horizon_api_shim::mod_api::ClientHost) {
        let pending = std::mem::take(&mut self.pending);
        if pending.is_empty() {
            return;
        }
        let mut env = crate::port::env_impl::HeadlessEnv::new();
        for p in pending {
            match p {
                Pending::Toggle(name) => {
                    self.modules.toggle_with_message_by_name(
                        &name,
                        &mut env,
                        self.config.chat_feedback,
                    );
                }
                Pending::SetSetting {
                    module,
                    setting,
                    value,
                } => {
                    if let Some(base) = self.modules.base_mut(&module) {
                        if let Some(s) = base
                            .settings
                            .groups
                            .iter_mut()
                            .flat_map(|g| g.settings.iter_mut())
                            .find(|s| s.name.eq_ignore_ascii_case(&setting))
                        {
                            s.parse(&value);
                        }
                    }
                }
                Pending::ResetSetting { module, setting } => {
                    if let Some(base) = self.modules.base(&module) {
                        if let Some(s) = base.settings.get(&setting) {
                            s.reset();
                        }
                    }
                }
                Pending::ResetModule(name) => {
                    if let Some(base) = self.modules.base(&name) {
                        base.settings.reset();
                    }
                }
                Pending::BeginBind(name) => {
                    self.binding = Some(name);
                    self.awaiting_key_release = true;
                }
                Pending::SetKeybind { module, keybind } => {
                    if let Some(base) = self.modules.base_mut(&module) {
                        base.keybind = keybind;
                    }
                }
                Pending::SetHudActive(on) => self.hud.active = on,
                Pending::SaveConfigs => {
                    let _ = self.save();
                }
                Pending::ReloadConfigs => self.load(),
            }
        }
        let _ = host;
        self.refresh_snapshot();
    }

    /// A chat line from the player: client commands first, then the server.
    pub fn on_chat(
        &mut self,
        host: &mut dyn crate::horizon_api_shim::mod_api::ClientHost,
        line: &str,
    ) -> bool {
        self.refresh_snapshot();
        let Shared {
            commands,
            snapshot,
            pending,
            ticks,
            ..
        } = self;
        let mut env = crate::port::env_impl::HorizonEnv::new(host, snapshot, pending, *ticks);
        let result = commands.dispatch(&mut env, line);
        drop(env);
        self.apply_pending(host);
        matches!(result, crate::meteor_core::command::Dispatch::Handled)
    }

    /// A key or mouse event; returns true when the client consumed it.
    pub fn on_input(
        &mut self,
        host: &mut dyn crate::horizon_api_shim::mod_api::ClientHost,
        is_key: bool,
        value: i32,
        pressed: bool,
        modifiers: u8,
    ) -> bool {
        self.refresh_snapshot();
        let feedback = self.config.chat_feedback;
        let Shared {
            modules,
            snapshot,
            pending,
            ticks,
            ..
        } = self;
        let mut env = crate::port::env_impl::HorizonEnv::new(host, snapshot, pending, *ticks);
        let consumed = modules.on_input(&mut env, is_key, value, pressed, modifiers, feedback);
        drop(env);
        self.apply_pending(host);
        consumed
    }

    /// Packet hook: returns false when a module cancelled it.
    pub fn on_packet(
        &mut self,
        env: &mut dyn Env,
        packet: &mut PacketInfo,
        clientbound: bool,
    ) -> bool {
        if clientbound {
            self.modules.receive_packet(env, packet)
        } else {
            self.modules.send_packet(env, packet)
        }
    }

    // ---- persistence ------------------------------------------------------

    /// `Systems.save()`.
    pub fn save(&mut self) -> usize {
        let folder = self.config.folder.clone();
        let mut n = 0;
        if ConfigFile::new("modules", self.modules.to_tag())
            .save(&folder)
            .is_ok()
        {
            n += 1;
        }
        if ConfigFile::new("hud", self.hud.to_tag())
            .save(&folder)
            .is_ok()
        {
            n += 1;
        }
        if ConfigFile::new("friends", self.friends.to_tag())
            .save(&folder)
            .is_ok()
        {
            n += 1;
        }
        let mut cfg = crate::meteor_core::config::Tag::compound();
        cfg.put_str("prefix", &self.config.prefix);
        cfg.put_bool("chat-feedback", self.config.chat_feedback);
        cfg.put(
            "settings",
            crate::meteor_core::config::settings_to_tag(&self.config.settings),
        );
        let mut theme = crate::meteor_core::config::Tag::compound();
        theme.put(
            "settings",
            crate::meteor_core::config::settings_to_tag(&self.theme.settings),
        );
        cfg.put("theme", theme);
        if ConfigFile::new("config", cfg).save(&folder).is_ok() {
            n += 1;
        }
        n
    }

    /// `Systems.load()`.
    pub fn load(&mut self) {
        let folder = self.config.folder.clone();
        let mut env = crate::port::env_impl::HeadlessEnv::new();
        if let Some(tag) = ConfigFile::load(&folder, "modules") {
            self.modules.from_tag(&tag, &mut env);
        }
        if let Some(tag) = ConfigFile::load(&folder, "hud") {
            self.hud.from_tag(&tag);
        }
        if let Some(tag) = ConfigFile::load(&folder, "friends") {
            self.friends.from_tag(&tag);
        }
        if let Some(tag) = ConfigFile::load(&folder, "config") {
            if !tag.get_str("prefix").is_empty() {
                self.config.prefix = tag.get_str("prefix").to_string();
            }
            if let Some(f) = tag.get_bool("chat-feedback") {
                self.config.chat_feedback = f;
            }
            if let Some(s) = tag.get("settings") {
                crate::meteor_core::config::settings_from_tag(&mut self.config.settings, s);
            }
            if let Some(t) = tag.get("theme").and_then(|t| t.get("settings")) {
                crate::meteor_core::config::settings_from_tag(&mut self.theme.settings, t);
            }
        }
        self.refresh_snapshot();
    }

    /// Module titles by name, for the GUI.
    pub fn title_of(&self, name: &str) -> String {
        self.snapshot
            .titles
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// The modules of one category, in the registry's (title-sorted) order.
    pub fn group(&self, category: Category) -> Vec<&'static str> {
        self.modules
            .group(category)
            .into_iter()
            .map(|m| m.base().name)
            .collect()
    }

    pub fn setting_rows(&self, module: &str) -> Vec<(String, String)> {
        self.snapshot.setting_names_for_module(module)
    }
}

impl Default for Shared {
    fn default() -> Self {
        Self::new()
    }
}

/// Where `modules.nbt` and friends are written: Horizon's per-mod data folder.
pub fn data_dir() -> PathBuf {
    default_folder()
}

/// Every module, as `(name, category)`, for docs and for the tests that keep the
/// registry honest against Meteor's list.
pub fn module_inventory(modules: &Modules) -> Vec<(&'static str, Category)> {
    modules
        .all()
        .map(|m| (m.base().name, m.base().category))
        .collect()
}

/// Registers a module built from a closure, so a category file stays a list.
pub fn add(modules: &mut Modules, module: Box<dyn Module>) {
    modules.add(module);
}

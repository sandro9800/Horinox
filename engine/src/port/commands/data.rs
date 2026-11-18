//! The stores the game-facing commands need: waypoints, macros, fake players,
//! profiles and the ender-chest preview cache.
//!
//! Java keeps these in `systems.waypoints.Waypoints`, `systems.macros.Macros`,
//! `systems.profiles.Profiles` and
//! `utils.entity.fakeplayer.FakePlayerManager`. `meteor-core` does not own them
//! and the port keeps them local to the command package: in memory, plus one
//! text file each so `.save`/`.reload` and a restart keep them.
//!
//! Every store is behind a `Mutex` because [`crate::meteor_core::command::Command`] is
//! `Send + Sync` and the registry is shared by the render and the server thread.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use crate::meteor_core::env::Env;
use crate::meteor_core::math::{BlockPos, Vec3d};

/// Horizon hands a mod one data folder (`native_data_dir`); the port reads it
/// from `HORIZON_MOD_DATA` and falls back to the system temp folder so a
/// headless test run never writes into the workspace.
pub fn data_dir() -> PathBuf {
    std::env::var_os("HORIZON_MOD_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("horixon"))
}

/// `<data>/notebot`, where `NotebotSongArgumentType` resolves a song name.
pub fn notebot_dir() -> PathBuf {
    data_dir().join("notebot")
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---- waypoints ------------------------------------------------------------

/// `systems.waypoints.Waypoint`: a named point in one dimension.
#[derive(Clone, Debug, PartialEq)]
pub struct Waypoint {
    pub name: String,
    /// `Dimension.toString()`, `minecraft:overworld` style.
    pub dimension: String,
    pub pos: BlockPos,
    pub visible: bool,
}

impl Waypoint {
    /// `Waypoint.Builder().name(..).pos(..).dimension(..).build()`.
    pub fn new(name: &str, pos: BlockPos, dimension: &str) -> Self {
        Self {
            name: name.to_string(),
            dimension: dimension.to_string(),
            pos,
            visible: true,
        }
    }

    /// `waypoint list`'s short form.
    pub fn pos_text(&self) -> String {
        format!("X: {} Z: {}", self.pos.x, self.pos.z)
    }

    /// `waypoint get`'s full form.
    pub fn full_pos(&self) -> String {
        format!("X: {}, Y: {}, Z: {}", self.pos.x, self.pos.y, self.pos.z)
    }

    fn to_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            self.name, self.dimension, self.pos.x, self.pos.y, self.pos.z, self.visible
        )
    }

    fn from_line(line: &str) -> Option<Self> {
        let mut parts = line.split('\t');
        let name = parts.next()?.to_string();
        let dimension = parts.next()?.to_string();
        let x: i32 = parts.next()?.parse().ok()?;
        let y: i32 = parts.next()?.parse().ok()?;
        let z: i32 = parts.next()?.parse().ok()?;
        let visible = parts.next().map(|v| v != "false").unwrap_or(true);
        Some(Self {
            name,
            dimension,
            pos: BlockPos::new(x, y, z),
            visible,
        })
    }
}

static WAYPOINTS: Mutex<Vec<Waypoint>> = Mutex::new(Vec::new());

/// `Waypoints.get()`.
pub struct Waypoints;

impl Waypoints {
    pub fn list() -> Vec<Waypoint> {
        lock(&WAYPOINTS).clone()
    }

    pub fn is_empty() -> bool {
        lock(&WAYPOINTS).is_empty()
    }

    pub fn add(waypoint: Waypoint) {
        let mut all = lock(&WAYPOINTS);
        all.retain(|w| !w.name.eq_ignore_ascii_case(&waypoint.name));
        all.push(waypoint);
    }

    pub fn get(name: &str) -> Option<Waypoint> {
        lock(&WAYPOINTS)
            .iter()
            .find(|w| w.name.eq_ignore_ascii_case(name))
            .cloned()
    }

    pub fn remove(name: &str) -> bool {
        let mut all = lock(&WAYPOINTS);
        let before = all.len();
        all.retain(|w| !w.name.eq_ignore_ascii_case(name));
        all.len() != before
    }

    /// Flips `visible` and returns the new state.
    pub fn toggle(name: &str) -> Option<bool> {
        let mut all = lock(&WAYPOINTS);
        let w = all.iter_mut().find(|w| w.name.eq_ignore_ascii_case(name))?;
        w.visible = !w.visible;
        Some(w.visible)
    }

    pub fn clear() {
        lock(&WAYPOINTS).clear();
    }

    /// `Waypoints.save()`: one tab-separated line per waypoint.
    pub fn save() -> bool {
        let dir = data_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            return false;
        }
        let mut text = String::new();
        for waypoint in lock(&WAYPOINTS).iter() {
            text.push_str(&waypoint.to_line());
        }
        std::fs::write(dir.join("waypoints.txt"), text).is_ok()
    }

    /// `Waypoints.load()`; returns how many waypoints were read.
    pub fn load() -> usize {
        let path = data_dir().join("waypoints.txt");
        let Ok(text) = std::fs::read_to_string(path) else {
            return 0;
        };
        let parsed: Vec<Waypoint> = text.lines().filter_map(Waypoint::from_line).collect();
        let n = parsed.len();
        let mut all = lock(&WAYPOINTS);
        all.clear();
        all.extend(parsed);
        n
    }
}

// ---- macros ---------------------------------------------------------------

/// What a `systems.macros.Macro` does when it fires.
#[derive(Clone, Debug, PartialEq)]
pub enum MacroAction {
    /// Runs a client command, `.toggle kill-aura` style.
    Command(String),
    /// Sends a chat line.
    Chat(String),
    /// Toggles one module.
    Toggle(String),
}

impl MacroAction {
    pub fn kind(&self) -> &'static str {
        match self {
            MacroAction::Command(_) => "command",
            MacroAction::Chat(_) => "chat",
            MacroAction::Toggle(_) => "toggle",
        }
    }

    pub fn value(&self) -> &str {
        match self {
            MacroAction::Command(v) | MacroAction::Chat(v) | MacroAction::Toggle(v) => v,
        }
    }

    fn parse(kind: &str, value: &str) -> Option<Self> {
        match kind {
            "command" => Some(MacroAction::Command(value.to_string())),
            "chat" => Some(MacroAction::Chat(value.to_string())),
            "toggle" => Some(MacroAction::Toggle(value.to_string())),
            _ => None,
        }
    }
}

/// `systems.macros.Macro`.
#[derive(Clone, Debug, PartialEq)]
pub struct Macro {
    pub name: String,
    pub action: MacroAction,
}

impl Macro {
    pub fn new(name: &str, action: MacroAction) -> Self {
        Self {
            name: name.to_string(),
            action,
        }
    }

    /// `Macro.onAction()`.
    pub fn run(&self, env: &mut dyn Env) {
        match &self.action {
            MacroAction::Command(cmd) => env.command(cmd),
            MacroAction::Chat(msg) => env.chat_player(msg),
            MacroAction::Toggle(module) => env.request_toggle(module),
        }
    }

    fn to_line(&self) -> String {
        format!(
            "{}\t{}\t{}\n",
            self.name,
            self.action.kind(),
            self.action.value()
        )
    }

    fn from_line(line: &str) -> Option<Self> {
        let mut parts = line.split('\t');
        let name = parts.next()?.to_string();
        let kind = parts.next()?;
        let value = parts.next()?;
        Some(Self {
            name,
            action: MacroAction::parse(kind, value)?,
        })
    }
}

struct MacroStore {
    loaded: bool,
    macros: Vec<Macro>,
}

static MACROS: Mutex<MacroStore> = Mutex::new(MacroStore {
    loaded: false,
    macros: Vec::new(),
});

/// `Macros.get()`.
pub struct Macros;

impl Macros {
    /// Reads `macros.txt` once, the way `Macros.init()` loads the system.
    fn ensure_loaded(st: &mut MacroStore) {
        if st.loaded {
            return;
        }
        st.loaded = true;
        let Ok(text) = std::fs::read_to_string(data_dir().join("macros.txt")) else {
            return;
        };
        st.macros.extend(text.lines().filter_map(Macro::from_line));
    }

    pub fn all() -> Vec<Macro> {
        let mut st = lock(&MACROS);
        Self::ensure_loaded(&mut st);
        st.macros.clone()
    }

    pub fn get(name: &str) -> Option<Macro> {
        Self::all()
            .into_iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// Tries the whole quoted name first, then everything but the last token -?    /// `CmdCtx` splits on whitespace, so `"my macro" 20` arrives in pieces.
    pub fn resolve(args: &[String]) -> Option<(Macro, usize)> {
        for take in (1..=args.len()).rev() {
            let name = args[..take].join(" ");
            if let Some(mac) = Self::get(&name) {
                return Some((mac, take));
            }
        }
        None
    }

    /// `Macros.add(..)`.
    pub fn insert(mac: Macro) -> bool {
        let mut st = lock(&MACROS);
        Self::ensure_loaded(&mut st);
        let replaced = st
            .macros
            .iter()
            .any(|m| m.name.eq_ignore_ascii_case(&mac.name));
        st.macros
            .retain(|m| !m.name.eq_ignore_ascii_case(&mac.name));
        st.macros.push(mac);
        replaced
    }

    pub fn clear() {
        let mut st = lock(&MACROS);
        st.macros.clear();
        st.loaded = true;
    }

    /// `Macros.save()`.
    pub fn save() -> bool {
        let dir = data_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            return false;
        }
        let mut text = String::new();
        for mac in Self::all() {
            text.push_str(&mac.to_line());
        }
        std::fs::write(dir.join("macros.txt"), text).is_ok()
    }
}

// ---- fake players ---------------------------------------------------------

/// `utils.entity.fakeplayer.FakePlayerEntity`, as the command needs it: the
/// name, health and spawn point. Spawning the entity needs a level write API
/// Horizon does not expose, so [`FakePlayerManager`] only keeps the roster that
/// the `fake-player` module would spawn from.
#[derive(Clone, Debug, PartialEq)]
pub struct FakePlayer {
    pub name: String,
    pub health: f32,
    pub pos: Vec3d,
    pub copy_inv: bool,
}

static FAKE_PLAYERS: Mutex<Vec<FakePlayer>> = Mutex::new(Vec::new());

/// `FakePlayerManager`.
pub struct FakePlayerManager;

impl FakePlayerManager {
    pub fn add(name: &str, health: f32, copy_inv: bool, pos: Vec3d) {
        let mut all = lock(&FAKE_PLAYERS);
        all.retain(|fp| !fp.name.eq_ignore_ascii_case(name));
        all.push(FakePlayer {
            name: name.to_string(),
            health,
            pos,
            copy_inv,
        });
    }

    pub fn get(name: &str) -> Option<FakePlayer> {
        lock(&FAKE_PLAYERS)
            .iter()
            .find(|fp| fp.name == name)
            .cloned()
    }

    pub fn contains(name: &str) -> bool {
        lock(&FAKE_PLAYERS).iter().any(|fp| fp.name == name)
    }

    pub fn remove(name: &str) -> bool {
        let mut all = lock(&FAKE_PLAYERS);
        let before = all.len();
        all.retain(|fp| fp.name != name);
        all.len() != before
    }

    pub fn clear() {
        lock(&FAKE_PLAYERS).clear();
    }

    pub fn count() -> usize {
        lock(&FAKE_PLAYERS).len()
    }

    pub fn names() -> Vec<String> {
        lock(&FAKE_PLAYERS)
            .iter()
            .map(|fp| fp.name.clone())
            .collect()
    }
}

// ---- profiles -------------------------------------------------------------

/// `systems.profiles.Profile`. A profile is a set of settings the client
/// applies on join; the store keeps the roster and one file per profile.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub name: String,
}

static PROFILES: Mutex<Vec<Profile>> = Mutex::new(Vec::new());

/// `Profiles.get()`.
pub struct Profiles;

impl Profiles {
    pub fn names() -> Vec<String> {
        lock(&PROFILES).iter().map(|p| p.name.clone()).collect()
    }

    pub fn get(name: &str) -> Option<Profile> {
        lock(&PROFILES)
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
            .cloned()
    }

    pub fn insert(name: &str) {
        let mut all = lock(&PROFILES);
        if !all.iter().any(|p| p.name.eq_ignore_ascii_case(name)) {
            all.push(Profile {
                name: name.to_string(),
            });
        }
    }

    pub fn remove(name: &str) -> bool {
        let mut all = lock(&PROFILES);
        let before = all.len();
        all.retain(|p| !p.name.eq_ignore_ascii_case(name));
        all.len() != before
    }

    pub fn clear() {
        lock(&PROFILES).clear();
    }

    /// `Profile.load()`: the profile file's text, or `None` when it is missing.
    pub fn read(name: &str) -> Option<String> {
        std::fs::read_to_string(profile_path(name)?).ok()
    }

    /// `Profile.save()`.
    pub fn write(name: &str) -> bool {
        let path = profile_path(name);
        let Some(path) = path else { return false };
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return false;
            }
        }
        Self::insert(name);
        std::fs::write(path, format!("name\t{}\n", name)).is_ok()
    }
}

fn profile_path(name: &str) -> Option<PathBuf> {
    let clean = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' || c == '.');
    if !clean {
        return None;
    }
    Some(data_dir().join("profiles").join(format!("{}.txt", name)))
}

// ---- ender chest preview --------------------------------------------------

static ENDER_CHEST: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// `EnderChestCommand`'s memory of the last ender chest it saw. Meteor fills the
/// same 27 item stacks through `Utils.openContainer`; Horizon has no container
/// screen, so the mod's packet hook fills this cache instead.
pub struct EnderChest;

impl EnderChest {
    /// The cached contents, `minecraft:<id>` per non-empty slot.
    pub fn items() -> Vec<String> {
        lock(&ENDER_CHEST).clone()
    }

    pub fn set(items: Vec<String>) {
        *lock(&ENDER_CHEST) = items;
    }

    pub fn clear() {
        lock(&ENDER_CHEST).clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn waypoints_round_trip() {
        Waypoints::clear();
        Waypoints::add(Waypoint::new(
            "base",
            BlockPos::new(10, 64, -20),
            "minecraft:overworld",
        ));
        let wp = Waypoints::get("BASE").expect("waypoint lookup is case-insensitive");
        assert_eq!(wp.pos_text(), "X: 10 Z: -20");
        assert_eq!(wp.full_pos(), "X: 10, Y: 64, Z: -20");
        assert!(Waypoints::toggle("base").is_some());
        assert!(!Waypoints::get("base").unwrap().visible);
        assert!(Waypoints::save());
        Waypoints::clear();
        assert_eq!(Waypoints::load(), 1);
        assert_eq!(
            Waypoints::get("base").unwrap().pos,
            BlockPos::new(10, 64, -20)
        );
        assert!(Waypoints::remove("base"));
        assert!(Waypoints::is_empty());
    }

    #[test]
    fn macros_run_their_action() {
        Macros::clear();
        let mac = Macro::new("hello", MacroAction::Chat("hi there".into()));
        Macros::insert(mac.clone());
        assert_eq!(Macros::get("HELLO"), Some(mac.clone()));
        let mut env = TestEnv::in_world();
        mac.run(&mut env);
        assert_eq!(env.chats.last().unwrap(), "hi there");
        assert!(Macros::resolve(&["hello".into()]).is_some());
        assert!(Macros::save());
        assert_eq!(Macros::all().len(), 1);
    }

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn fake_players_and_profiles_are_rosters() {
        FakePlayerManager::clear();
        FakePlayerManager::add("Bob", 20.0, false, Vec3d::ZERO);
        assert!(FakePlayerManager::contains("Bob"));
        assert_eq!(FakePlayerManager::count(), 1);
        assert!(FakePlayerManager::remove("Bob"));
        assert_eq!(FakePlayerManager::count(), 0);

        Profiles::clear();
        Profiles::insert("anarchy");
        assert!(Profiles::get("ANARCHY").is_some());
        assert!(Profiles::write("anarchy"));
        assert!(Profiles::read("anarchy").is_some());
        assert!(Profiles::remove("anarchy"));
        assert!(Profiles::read("anarchy").is_none());
    }

    #[test]
    fn ender_chest_cache() {
        EnderChest::set(vec!["minecraft:diamond".into()]);
        assert_eq!(EnderChest::items().len(), 1);
        EnderChest::clear();
        assert!(EnderChest::items().is_empty());
    }
}

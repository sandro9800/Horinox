//! The command state that survives between invocations: `input`'s keypress
//! handlers, `macro`'s scheduler, `notebot`'s recorder, `server`'s plugin cache
//! and `locate`'s eye-of-ender trajectory.
//!
//! Java keeps this on the command objects (`MacroCommand.scheduledMacros`,
//! `NotebotCommand.song`, -? and ticks it from `MeteorClient.EVENT_BUS`. The
//! port's `Command` values are stateless (`Send + Sync`), so the state lives here
//! behind one `Mutex` per system, and `commands::on_tick` /
//! `commands::on_receive_packet` are the hooks the mod drives it from.

use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::meteor_core::env::GameMode;
use crate::meteor_core::math::Vec3d;

use crate::meteor_core::env::Env;
use crate::port::commands::data::Macros;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---- input ----------------------------------------------------------------

/// One `InputCommand.KeypressHandler`: hold `key` down for `ticks` ticks.
#[derive(Clone, Debug, PartialEq)]
pub struct KeypressHandler {
    pub key: String,
    pub ticks: i32,
    pub total_ticks: i32,
}

static INPUT_HANDLERS: Mutex<Vec<KeypressHandler>> = Mutex::new(Vec::new());

/// `InputCommand.activeHandlers`.
pub fn input_add(key: &str, ticks: i32) {
    lock(&INPUT_HANDLERS).push(KeypressHandler {
        key: key.to_string(),
        ticks,
        total_ticks: ticks,
    });
}

pub fn input_handlers() -> Vec<KeypressHandler> {
    lock(&INPUT_HANDLERS).clone()
}

pub fn input_clear() -> usize {
    let mut all = lock(&INPUT_HANDLERS);
    let n = all.len();
    all.clear();
    n
}

pub fn input_remove(index: usize) -> bool {
    let mut all = lock(&INPUT_HANDLERS);
    if index >= all.len() {
        return false;
    }
    all.remove(index);
    true
}

/// `InputCommand`'s `TickEvent.Post`: the handlers count down and drop off.
///
/// PORT GAP: Java holds the key down through `KeyMapping.setDown` and bumps the
/// click count; Horizon exposes no key-injection call, so the handlers are
/// bookkeeping (and `input list`/`remove` stay exact) while the actual press
/// has to wait for a `native_input` hook.
pub fn input_tick(_env: &mut dyn Env) {
    let mut all = lock(&INPUT_HANDLERS);
    all.retain_mut(|handler| {
        if handler.ticks > 0 {
            handler.ticks -= 1;
            true
        } else {
            false
        }
    });
}

// ---- macro scheduling -----------------------------------------------------

/// `MacroCommand.ScheduledMacro`.
#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledMacro {
    pub delay: i32,
    pub name: String,
}

struct MacroSchedule {
    queue: Vec<ScheduledMacro>,
    scheduled: Vec<ScheduledMacro>,
}

static MACRO_SCHEDULE: Mutex<MacroSchedule> = Mutex::new(MacroSchedule {
    queue: Vec::new(),
    scheduled: Vec::new(),
});

/// `MacroCommand.scheduleQueue.add(..)`.
pub fn macro_schedule(name: &str, delay: i32) {
    lock(&MACRO_SCHEDULE).queue.push(ScheduledMacro {
        delay,
        name: name.to_string(),
    });
}

pub fn macro_is_scheduled(name: &str) -> bool {
    let st = lock(&MACRO_SCHEDULE);
    st.queue
        .iter()
        .chain(st.scheduled.iter())
        .any(|job| job.name.eq_ignore_ascii_case(name))
}

pub fn macro_scheduled_count() -> usize {
    let st = lock(&MACRO_SCHEDULE);
    st.queue.len() + st.scheduled.len()
}

pub fn macro_clear(name: &str) -> bool {
    let mut st = lock(&MACRO_SCHEDULE);
    let before = st.queue.len() + st.scheduled.len();
    st.queue.retain(|job| !job.name.eq_ignore_ascii_case(name));
    st.scheduled
        .retain(|job| !job.name.eq_ignore_ascii_case(name));
    before != st.queue.len() + st.scheduled.len()
}

pub fn macro_clear_all() {
    let mut st = lock(&MACRO_SCHEDULE);
    st.queue.clear();
    st.scheduled.clear();
}

/// `MacroCommand`'s `TickEvent.Post`: move the queue in, run what is due, then
/// count the rest down (Meteor's exact order).
pub fn macro_tick(env: &mut dyn Env) {
    let due = {
        let mut st = lock(&MACRO_SCHEDULE);
        if !st.queue.is_empty() {
            let queue = std::mem::take(&mut st.queue);
            st.scheduled.extend(queue);
        }
        let mut due = Vec::new();
        let mut keep = Vec::new();
        for job in st.scheduled.drain(..) {
            if job.delay <= 0 {
                due.push(job.name);
            } else {
                keep.push(job);
            }
        }
        st.scheduled = keep;
        due
    };
    for name in due {
        if let Some(mac) = Macros::get(&name) {
            mac.run(env);
        }
    }
    let mut st = lock(&MACRO_SCHEDULE);
    for job in st.scheduled.iter_mut() {
        job.delay -= 1;
    }
}

// ---- notebot --------------------------------------------------------------

/// The `NoteBlockInstrument` ordinals `notebot` writes to its song files, in the
/// enum's order.
pub const INSTRUMENTS: [&str; 16] = [
    "harp",
    "basedrum",
    "snare",
    "hat",
    "bass",
    "flute",
    "bell",
    "guitar",
    "chime",
    "xylophone",
    "iron_xylophone",
    "cow_bell",
    "didgeridoo",
    "bit",
    "banjo",
    "pling",
];

/// `NotebotCommand`'s recording buffer and player state.
#[derive(Clone, Debug)]
pub struct NotebotState {
    pub recording: bool,
    /// `tick -> notes`; `-1` before the first sound arrives.
    pub ticks: i32,
    /// `(tick, note level, instrument ordinal)`.
    pub song: Vec<(i32, u8, u8)>,
    pub playing: bool,
    pub paused: bool,
    pub current_tick: i32,
    pub last_tick: i32,
    pub preview: bool,
    pub song_name: String,
}

impl NotebotState {
    const fn new() -> Self {
        Self {
            recording: false,
            ticks: -1,
            song: Vec::new(),
            playing: false,
            paused: false,
            current_tick: 0,
            last_tick: 0,
            preview: false,
            song_name: String::new(),
        }
    }

    /// `Notebot.getStatus()`.
    pub fn status(&self) -> String {
        if self.playing {
            format!(
                "Playing song. {}/{}",
                self.current_tick,
                self.last_tick.max(0)
            )
        } else if self.song.is_empty() {
            "No song loaded.".to_string()
        } else if self.paused {
            format!("Paused. {}", self.song_name)
        } else {
            "Ready to play.".to_string()
        }
    }
}

static NOTEBOT: Mutex<NotebotState> = Mutex::new(NotebotState::new());

pub fn notebot_with<R>(f: impl FnOnce(&mut NotebotState) -> R) -> R {
    let mut st = lock(&NOTEBOT);
    f(&mut st)
}

/// Stops playback and forgets the song (the notebot stop form).
pub fn notebot_reset() {
    notebot_with(|s| {
        s.playing = false;
        s.paused = false;
        s.current_tick = 0;
        s.song.clear();
        s.song_name.clear();
    });
}

pub fn notebot_status() -> String {
    lock(&NOTEBOT).status()
}

/// The recorder's `TickEvent.Post`.
pub fn notebot_tick() {
    let mut st = lock(&NOTEBOT);
    if st.recording && st.ticks >= 0 {
        st.ticks += 1;
    }
    if st.playing && !st.paused {
        st.current_tick += 1;
    }
}

/// `NotebotCommand.getNote`: brute-force the note level out of the pitch.
///
/// `Math.pow(2, (n - 12) / 12)` within 0.01 of the packet's pitch.
pub fn note_from_pitch(pitch: f32) -> Option<u8> {
    for n in 0..25i32 {
        let expected = 2.0f64.powf((n - 12) as f64 / 12.0) as f32;
        if expected - 0.01 < pitch && expected + 0.01 > pitch {
            return Some(n as u8);
        }
    }
    None
}

/// `NotebotCommand.getInstrumentFromSound`, in Meteor's own check order.
///
/// Meteor tests `xylophone` before `iron_xylophone` and `bell` before `cow_bell`,
/// so `block.note_block.iron_xylophone` lands on `xylophone` and `cow_bell` on
/// `bell`; the port keeps that order rather than "fixing" the mapping.
pub fn instrument_from_sound(path: &str) -> Option<u8> {
    for (ordinal, name) in INSTRUMENTS.iter().enumerate() {
        if path.contains(name) {
            return Some(ordinal as u8);
        }
    }
    None
}

// ---- server ---------------------------------------------------------------

#[derive(Default)]
struct ServerCache {
    plugins: Vec<String>,
    /// The `bukkit:version`-style alias the command tree offered.
    alias: Option<String>,
    scanning: bool,
}

static SERVER: Mutex<ServerCache> = Mutex::new(ServerCache {
    plugins: Vec::new(),
    alias: None,
    scanning: false,
});

pub fn server_plugins() -> Vec<String> {
    lock(&SERVER).plugins.clone()
}

pub fn server_add_plugins(names: impl IntoIterator<Item = String>) {
    let mut cache = lock(&SERVER);
    for name in names {
        if !cache.plugins.iter().any(|p| p.eq_ignore_ascii_case(&name)) {
            cache.plugins.push(name);
        }
    }
}

/// `ServerCommand.commandTreePlugins` -?what a `ClientboundCommandsPacket` gave.
pub fn server_set_tree_plugins(names: Vec<String>, alias: Option<String>) {
    let mut cache = lock(&SERVER);
    cache.plugins = names;
    cache.alias = alias;
}

pub fn server_alias() -> Option<String> {
    lock(&SERVER).alias.clone()
}

pub fn server_begin_scan() -> bool {
    let mut cache = lock(&SERVER);
    if cache.alias.is_none() {
        return false;
    }
    cache.scanning = true;
    true
}

pub fn server_scanning() -> bool {
    lock(&SERVER).scanning
}

pub fn server_end_scan() -> Vec<String> {
    let mut cache = lock(&SERVER);
    cache.scanning = false;
    cache.plugins.clone()
}

pub fn server_clear() {
    *lock(&SERVER) = ServerCache::default();
}

// ---- locate ---------------------------------------------------------------

/// `LocateCommand`'s two eye-of-ender throws.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LocateState {
    pub first_start: Option<Vec3d>,
    pub first_end: Option<Vec3d>,
    pub second_start: Option<Vec3d>,
    pub second_end: Option<Vec3d>,
}

static LOCATE: Mutex<LocateState> = Mutex::new(LocateState {
    first_start: None,
    first_end: None,
    second_start: None,
    second_end: None,
});

/// What a thrown eye told the command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocateOutcome {
    /// The first eye landed: a second throw is needed.
    AwaitSecond,
    /// Both throws are in; the intersection is the stronghold's rough position.
    Located(Vec3d),
    /// The two lines are parallel or the data is missing.
    Failed,
}

pub fn locate_reset() {
    *lock(&LOCATE) = LocateState::default();
}

pub fn locate_state() -> LocateState {
    *lock(&LOCATE)
}

/// `ClientboundAddEntityPacket` for an `EyeOfEnder`.
pub fn locate_eye_spawn(pos: Vec3d) {
    let mut st = lock(&LOCATE);
    if st.first_start.is_none() {
        st.first_start = Some(pos);
    } else {
        st.second_start = Some(pos);
    }
}

/// `EntityRemovedEvent` for an `EyeOfEnder`.
pub fn locate_eye_land(pos: Vec3d) -> LocateOutcome {
    let mut st = lock(&LOCATE);
    if st.first_end.is_none() {
        st.first_end = Some(pos);
        return LocateOutcome::AwaitSecond;
    }
    st.second_end = Some(pos);
    let (Some(first_start), Some(first_end), Some(second_start), Some(second_end)) =
        (st.first_start, st.first_end, st.second_start, st.second_end)
    else {
        return LocateOutcome::Failed;
    };
    // Meteor feeds the *second* throw in as `line` and the first as `line2`.
    let start = [second_start.x, second_start.z, second_end.x, second_end.z];
    let end = [first_start.x, first_start.z, first_end.x, first_end.z];
    let hit = intersect(start, end);
    if hit[0].is_nan() || hit[1].is_nan() || hit[0].is_infinite() || hit[1].is_infinite() {
        return LocateOutcome::Failed;
    }
    LocateOutcome::Located(Vec3d::new(hit[0], 0.0, hit[1]))
}

/// `LocateCommand.calcIntersection` for two `[x1, z1, x2, z2]` lines.
pub fn intersect(line: [f64; 4], line2: [f64; 4]) -> [f64; 2] {
    let a1 = line[3] - line[1];
    let b1 = line[0] - line[2];
    let c1 = a1 * line[0] + b1 * line[1];

    let a2 = line2[3] - line2[1];
    let b2 = line2[0] - line2[2];
    let c2 = a2 * line2[0] + b2 * line2[1];

    let delta = a1 * b2 - a2 * b1;
    [(b2 * c1 - b1 * c2) / delta, (a1 * c2 - a2 * c1) / delta]
}

// ---- the single-value knobs ----------------------------------------------

/// `mc.options.fov()` -?the value the renderer reads (`fov` command).
static FOV: AtomicU32 = AtomicU32::new(70.0f32.to_bits());

pub fn fov() -> f32 {
    f32::from_bits(FOV.load(Ordering::Relaxed))
}

/// Records the requested FOV. PORT GAP: Horizon exposes no options handle, so
/// the value is available to the mod (`commands::state::fov`) but not written
/// back into the game's own options object.
pub fn set_fov(value: f32) {
    FOV.store(value.to_bits(), Ordering::Relaxed);
}

/// `mc.gameMode.setLocalMode(...)` (`gamemode` command).
static LOCAL_GAMEMODE: Mutex<GameMode> = Mutex::new(GameMode::Survival);

pub fn set_local_gamemode(mode: GameMode) {
    *lock(&LOCAL_GAMEMODE) = mode;
}

pub fn local_gamemode() -> GameMode {
    *lock(&LOCAL_GAMEMODE)
}

/// `mc.setCameraEntity(...)` (`spectate` command).
static CAMERA_TARGET: Mutex<Option<String>> = Mutex::new(None);

pub fn set_camera_target(name: Option<&str>) {
    *lock(&CAMERA_TARGET) = name.map(|n| n.to_string());
}

pub fn camera_target() -> Option<String> {
    lock(&CAMERA_TARGET).clone()
}

/// `SwarmCommand.pendingConnection`.
static SWARM_PENDING: Mutex<Option<(String, i32)>> = Mutex::new(None);

pub fn swarm_set_pending(ip: &str, port: i32) {
    *lock(&SWARM_PENDING) = Some((ip.to_string(), port));
}

pub fn swarm_take_pending() -> Option<(String, i32)> {
    lock(&SWARM_PENDING).take()
}

pub fn swarm_pending() -> Option<(String, i32)> {
    lock(&SWARM_PENDING).clone()
}

/// `AutoWasp.target` (`wasp` command); the module reads the name.
static WASP_TARGET: Mutex<Option<String>> = Mutex::new(None);

pub fn set_wasp_target(name: Option<&str>) {
    *lock(&WASP_TARGET) = name.map(|n| n.to_string());
}

pub fn wasp_target() -> Option<String> {
    lock(&WASP_TARGET).clone()
}

/// The `input`/`macro`/`notebot` counters `ActiveModulesHud` style debug views
/// may want.
static TICKS_SERVED: AtomicI32 = AtomicI32::new(0);

pub fn note_tick() {
    TICKS_SERVED.fetch_add(1, Ordering::Relaxed);
}

pub fn ticks_served() -> i32 {
    TICKS_SERVED.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn keypress_handlers_count_down() {
        input_clear();
        input_add("sneak", 2);
        assert_eq!(input_handlers().len(), 1);
        let mut env = TestEnv::in_world();
        input_tick(&mut env);
        assert_eq!(input_handlers()[0].ticks, 1);
        input_tick(&mut env);
        input_tick(&mut env);
        assert!(input_handlers().is_empty());
        input_add("jump", 5);
        assert!(input_remove(0));
        assert!(!input_remove(0));
    }

    #[test]
    fn macro_schedule_waits_for_its_delay() {
        use crate::port::commands::data::{Macro, MacroAction};
        macro_clear_all();
        Macros::insert(Macro::new(
            "sched",
            MacroAction::Command(".say scheduled".into()),
        ));
        macro_schedule("sched", 1);
        assert!(macro_is_scheduled("sched"));
        let mut env = TestEnv::in_world();
        macro_tick(&mut env);
        assert!(env.commands.is_empty(), "delay 1 must not fire immediately");
        macro_tick(&mut env);
        assert_eq!(env.commands, vec![".say scheduled".to_string()]);
        assert_eq!(macro_scheduled_count(), 0);
    }

    #[test]
    fn note_pitch_and_instrument() {
        // Note level 12 is pitch 1.0; level 0 is 0.5.
        assert_eq!(note_from_pitch(1.0), Some(12));
        assert_eq!(note_from_pitch(0.5), Some(0));
        assert_eq!(note_from_pitch(0.123), None);
        assert_eq!(instrument_from_sound("block.note_block.harp"), Some(0));
        assert_eq!(instrument_from_sound("block.note_block.basedrum"), Some(1));
        assert_eq!(instrument_from_sound("block.note_block.pling"), Some(15));
        assert_eq!(
            instrument_from_sound("block.note_block.iron_xylophone"),
            Some(9)
        );
        assert_eq!(instrument_from_sound("step.stone"), None);
    }

    #[test]
    fn eye_of_ender_lines_cross() {
        locate_reset();
        // Throw 1: (0,0) heading east; throw 2: (10,10) heading north.
        locate_eye_spawn(Vec3d::new(0.0, 64.0, 0.0));
        assert_eq!(
            locate_eye_land(Vec3d::new(10.0, 64.0, 0.0)),
            LocateOutcome::AwaitSecond
        );
        locate_eye_spawn(Vec3d::new(10.0, 64.0, 10.0));
        match locate_eye_land(Vec3d::new(10.0, 64.0, 0.0)) {
            LocateOutcome::Located(pos) => {
                assert!((pos.x - 10.0).abs() < 1.0e-6, "x was {}", pos.x);
                assert!(pos.z.abs() < 1.0e-6 || (pos.z - 0.0).abs() < 1.0e-6);
            }
            other => panic!("expected an intersection, got {other:?}"),
        }
        // Parallel lines never meet.
        let hit = intersect([0.0, 0.0, 1.0, 0.0], [0.0, 1.0, 1.0, 1.0]);
        assert!(hit[0].is_nan() || hit[0].is_infinite());
        locate_reset();
    }

    #[test]
    fn knobs_round_trip() {
        set_fov(90.0);
        assert_eq!(fov(), 90.0);
        set_local_gamemode(GameMode::Adventure);
        assert_eq!(local_gamemode(), GameMode::Adventure);
        set_camera_target(Some("Bob"));
        assert_eq!(camera_target().as_deref(), Some("Bob"));
        set_camera_target(None);
        assert!(camera_target().is_none());
        swarm_set_pending("1.2.3.4", 5000);
        assert_eq!(swarm_pending().unwrap(), ("1.2.3.4".to_string(), 5000));
        assert_eq!(swarm_take_pending().unwrap().1, 5000);
        assert!(swarm_pending().is_none());
        set_wasp_target(Some("Bob"));
        assert_eq!(wasp_target().as_deref(), Some("Bob"));
        set_wasp_target(None);
    }

    #[test]
    fn server_cache_round_trip() {
        server_clear();
        server_set_tree_plugins(vec!["essentials".into()], Some("version".into()));
        assert_eq!(server_alias().as_deref(), Some("version"));
        assert!(server_begin_scan());
        server_add_plugins(vec!["Essentials".into(), "vulcan".into()]);
        assert_eq!(server_end_scan().len(), 2);
        server_clear();
        assert!(!server_begin_scan());
    }
}

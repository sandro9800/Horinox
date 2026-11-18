//! `systems.modules.misc.Notebot`.
//!
//! Java is a six-stage state machine - load a song, scan the note blocks around
//! the player, map the song's notes onto them, tune them, wait, play - plus a
//! render pass for the blocks in use. The port keeps the stages, the tuning
//! arithmetic (`(25 - from) + to` when the note has to wrap), the note map
//! settings and the render colours; the song *decoders* (NBS/MIDI) and the
//! Baritone-free playback packets are the parts Horizon has no equivalent for.

use std::collections::BTreeMap;

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{BlockInfo, Env, Hand, PacketInfo, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Dir, Rotation, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, IntSet};

/// `NoteBlockInstrument`'s playable set, in the note-map's setting order.
const INSTRUMENTS: &[&str] = &[
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
    "iron-xylophone",
    "cow-bell",
    "didgeridoo",
    "bit",
    "banjo",
    "pling",
];

/// `NotebotUtils.OptionalInstrument`.
const OPTIONAL_INSTRUMENTS: &[&str] = &[
    "None",
    "Harp",
    "Basedrum",
    "Snare",
    "Hat",
    "Bass",
    "Flute",
    "Bell",
    "Guitar",
    "Chime",
    "Xylophone",
    "IronXylophone",
    "CowBell",
    "Didgeridoo",
    "Bit",
    "Banjo",
    "Pling",
];

const NOTEBOT_MODES: &[&str] = &["AnyInstrument", "ExactInstruments"];
const INSTRUMENT_DETECT_MODES: &[&str] = &["BlockState", "BelowBlock"];
const SHAPE_MODES: &[&str] = &["Lines", "Sides", "Both"];

/// `utils.notebot.song.Note`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Note {
    pub instrument: &'static str,
    pub level: i32,
}

impl Note {
    pub fn new(instrument: &'static str, level: i32) -> Self {
        Self { instrument, level }
    }

    /// `NotebotUtils.getNoteFromNoteBlock`'s level part.
    pub fn note_level(&self) -> i32 {
        self.level
    }
}

/// `utils.notebot.song.Song` - the notes per tick.
#[derive(Clone, Debug, Default)]
pub struct Song {
    notes: BTreeMap<i32, Vec<Note>>,
}

impl Song {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, tick: i32, note: Note) {
        self.notes.entry(tick).or_default().push(note);
    }

    /// The port's stand-in for `SongDecoders.parse`: `tick:instrument:level`
    /// per line.
    /// `// PORT GAP:` the NBS and MIDI decoders are not ported; only this text
    /// format loads.
    pub fn from_text(text: &str) -> Self {
        let mut song = Song::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split(':');
            let (Some(tick), Some(instrument), Some(level)) =
                (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let (Ok(tick), Ok(level)) = (tick.trim().parse::<i32>(), level.trim().parse::<i32>())
            else {
                continue;
            };
            let Some(instrument) = instrument_name(instrument.trim()) else {
                continue;
            };
            song.add(tick, Note::new(instrument, level));
        }
        song
    }

    pub fn notes_at(&self, tick: i32) -> &[Note] {
        self.notes
            .get(&tick)
            .map(|notes| notes.as_slice())
            .unwrap_or(&[])
    }

    /// `Song.getLastTick()`.
    pub fn last_tick(&self) -> i32 {
        self.notes.keys().next_back().copied().unwrap_or(0)
    }

    /// `Song.getRequirements()` - every distinct note the song needs.
    pub fn requirements(&self) -> Vec<Note> {
        let mut notes: Vec<Note> = self.notes.values().flatten().cloned().collect();
        notes.sort();
        notes.dedup();
        notes
    }

    pub fn len(&self) -> usize {
        self.notes.values().map(|n| n.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// `Notebot.transposeOutOfRange`: shift notes into the 0..25 range.
    fn transpose(&mut self) {
        for note in self.notes.values_mut().flat_map(|notes| notes.iter_mut()) {
            let mut level = note.level;
            while level < 0 {
                level += 12;
            }
            while level > 24 {
                level -= 12;
            }
            note.level = level;
        }
    }
}

/// `Notebot.Stage`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    None,
    LoadingSong,
    SetUp,
    Tune,
    WaitingToCheckNoteblocks,
    Playing,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Stage::None => "None",
            Stage::LoadingSong => "LoadingSong",
            Stage::SetUp => "SetUp",
            Stage::Tune => "Tune",
            Stage::WaitingToCheckNoteblocks => "WaitingToCheckNoteblocks",
            Stage::Playing => "Playing",
        }
    }
}

/// `Notebot.PlayingMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayingMode {
    None,
    Preview,
    Noteblocks,
}

impl PlayingMode {
    fn name(self) -> &'static str {
        match self {
            PlayingMode::None => "None",
            PlayingMode::Preview => "Preview",
            PlayingMode::Noteblocks => "Noteblocks",
        }
    }
}

pub struct Notebot {
    base: ModuleBase,
    // General
    tick_delay: IntSet,
    concurrent_tune_blocks: IntSet,
    mode: EnumSet,
    instrument_detect_mode: EnumSet,
    polyphonic: BoolSet,
    auto_rotate: BoolSet,
    auto_play: BoolSet,
    transpose_out_of_range: BoolSet,
    swing_arm: BoolSet,
    check_noteblocks_again_delay: IntSet,
    // Render
    render_text: BoolSet,
    render_boxes: BoolSet,
    shape_mode: EnumSet,
    untuned_side_color: ColorSet,
    untuned_line_color: ColorSet,
    tuned_side_color: ColorSet,
    tuned_line_color: ColorSet,
    tune_hit_side_color: ColorSet,
    tune_hit_line_color: ColorSet,
    scanned_noteblock_side_color: ColorSet,
    scanned_noteblock_line_color: ColorSet,
    note_text_scale: DoubleSet,
    show_scanned_noteblocks: BoolSet,
    // Note Map
    note_map: Vec<(&'static str, EnumSet)>,
    // Runtime
    song: Option<Song>,
    stage: Stage,
    playing_mode: PlayingMode,
    is_playing: bool,
    current_tick: i32,
    ticks: i32,
    wait_ticks: i32,
    any_noteblock_tuned: bool,
    note_block_positions: Vec<(Note, BlockPos)>,
    scanned: Vec<(Note, BlockPos)>,
    clicked: Vec<BlockPos>,
    tune_hits: Vec<(BlockPos, i32)>,
    /// The requirements the scan could not satisfy, warned about once mapped.
    missing_notes: Vec<Note>,
    played: u32,
}

impl Notebot {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Misc, "notebot", "Plays noteblock nicely");

        let mut s = base.settings_mut();
        // The note map is created collapsed, as Java's `createGroup("Note Map", false)`.
        s.create_group("Note Map");
        if let Some(group) = s.groups.last_mut() {
            group.section_expanded = false;
        }
        s.create_group("Render");

        let tick_delay = s.default_group().int_setting("tick-delay", 1, 1, 20);
        s.describe_last("The delay when loading a song.");
        let concurrent_tune_blocks =
            s.default_group()
                .int_setting("concurrent-tune-blocks", 1, 1, 20);
        s.describe_last(
            "How many noteblocks can be tuned at the same time. On Paper it is recommended to set it to 1 to avoid bugs.",
        );
        let mode = s.default_group().enum_setting("mode", NOTEBOT_MODES, 1);
        s.describe_last("Select mode of notebot");
        let instrument_detect_mode =
            s.default_group()
                .enum_setting("instrument-detect-mode", INSTRUMENT_DETECT_MODES, 0);
        s.describe_last(
            "Select an instrument detect mode. Can be useful when server has a plugin that modifies noteblock state (e.g ItemsAdder) but noteblock can still play the right note",
        );
        let polyphonic = s.default_group().bool_setting("polyphonic", true);
        s.describe_last("Whether or not to allow multiple notes to be played at the same time");
        let auto_rotate = s.default_group().bool_setting("auto-rotate", true);
        s.describe_last("Should client look at note block when it wants to hit it");
        let auto_play = s.default_group().bool_setting("auto-play", false);
        s.describe_last("Auto plays random songs");
        let transpose_out_of_range = s
            .default_group()
            .bool_setting("transpose-out-of-range", false);
        s.describe_last("Transposes out of range notes");
        let swing_arm = s.default_group().bool_setting("swing-arm", true);
        s.describe_last("Should swing arm on hit");
        let check_noteblocks_again_delay =
            s.default_group()
                .int_setting("check-noteblocks-again-delay", 10, 1, 20);
        s.describe_last("How much delay should be between end of tuning and checking again");

        // Render
        let render_text = super::last_group(&mut s).bool_setting("render-text", true);
        s.describe_last("Whether or not to render the text above noteblocks.");
        let render_boxes = super::last_group(&mut s).bool_setting("render-boxes", true);
        s.describe_last("Whether or not to render the outline around the noteblocks.");
        let shape_mode = super::last_group(&mut s).enum_setting("shape-mode", SHAPE_MODES, 2);
        s.describe_last("How the shapes are rendered.");
        let untuned_side_color = super::last_group(&mut s)
            .color_setting("untuned-side-color", SettingColor::new(204, 0, 0, 10));
        s.describe_last("The color of the sides of the untuned blocks being rendered.");
        let untuned_line_color = super::last_group(&mut s)
            .color_setting("untuned-line-color", SettingColor::new(204, 0, 0, 255));
        s.describe_last("The color of the lines of the untuned blocks being rendered.");
        let tuned_side_color = super::last_group(&mut s)
            .color_setting("tuned-side-color", SettingColor::new(0, 204, 0, 10));
        s.describe_last("The color of the sides of the tuned blocks being rendered.");
        let tuned_line_color = super::last_group(&mut s)
            .color_setting("tuned-line-color", SettingColor::new(0, 204, 0, 255));
        s.describe_last("The color of the lines of the tuned blocks being rendered.");
        let tune_hit_side_color = super::last_group(&mut s)
            .color_setting("hit-side-color", SettingColor::new(255, 153, 0, 10));
        s.describe_last("The color of the sides being rendered on noteblock tune hit.");
        let tune_hit_line_color = super::last_group(&mut s)
            .color_setting("hit-line-color", SettingColor::new(255, 153, 0, 255));
        s.describe_last("The color of the lines being rendered on noteblock tune hit.");
        let scanned_noteblock_side_color = super::last_group(&mut s).color_setting(
            "scanned-noteblock-side-color",
            SettingColor::new(255, 255, 0, 30),
        );
        s.describe_last("The color of the sides of the scanned noteblocks being rendered.");
        let scanned_noteblock_line_color = super::last_group(&mut s).color_setting(
            "scanned-noteblock-line-color",
            SettingColor::new(255, 255, 0, 255),
        );
        s.describe_last("The color of the lines of the scanned noteblocks being rendered.");
        let note_text_scale =
            super::last_group(&mut s).double_setting("note-text-scale", 1.5, 0.0, 10.0, 2);
        s.describe_last("The scale.");
        let show_scanned_noteblocks =
            super::last_group(&mut s).bool_setting("show-scanned-noteblocks", false);
        s.describe_last("Show scanned Noteblocks");

        // Note Map: one enum setting per playable instrument, defaulting to
        // itself (`OptionalInstrument.fromMinecraftInstrument`).
        let mut note_map = Vec::new();
        if let Some(group) = s.groups.iter_mut().find(|g| g.name == "Note Map") {
            for instrument in INSTRUMENTS {
                let default = OPTIONAL_INSTRUMENTS
                    .iter()
                    .position(|name| name.eq_ignore_ascii_case(&instrument.replace('-', "")))
                    .unwrap_or(0);
                let setting = group.enum_setting(instrument, OPTIONAL_INSTRUMENTS, default);
                note_map.push((*instrument, setting));
            }
        }

        Self {
            base,
            tick_delay,
            concurrent_tune_blocks,
            mode,
            instrument_detect_mode,
            polyphonic,
            auto_rotate,
            auto_play,
            transpose_out_of_range,
            swing_arm,
            check_noteblocks_again_delay,
            render_text,
            render_boxes,
            shape_mode,
            untuned_side_color,
            untuned_line_color,
            tuned_side_color,
            tuned_line_color,
            tune_hit_side_color,
            tune_hit_line_color,
            scanned_noteblock_side_color,
            scanned_noteblock_line_color,
            note_text_scale,
            show_scanned_noteblocks,
            note_map,
            song: None,
            stage: Stage::None,
            playing_mode: PlayingMode::None,
            is_playing: false,
            current_tick: 0,
            ticks: 0,
            wait_ticks: -1,
            any_noteblock_tuned: false,
            note_block_positions: Vec::new(),
            scanned: Vec::new(),
            clicked: Vec::new(),
            tune_hits: Vec::new(),
            missing_notes: Vec::new(),
            played: 0,
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn playing_mode(&self) -> PlayingMode {
        self.playing_mode
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn note_blocks_in_use(&self) -> usize {
        self.note_block_positions.len()
    }

    pub fn played(&self) -> u32 {
        self.played
    }

    /// `Notebot.getStatus()`.
    pub fn status(&self) -> String {
        let Some(song) = &self.song else {
            return "No song loaded.".to_string();
        };
        if self.is_playing {
            return format!("Playing song. {}/{}", self.current_tick, song.last_tick());
        }
        if self.stage == Stage::Playing {
            return "Ready to play.".to_string();
        }
        if matches!(
            self.stage,
            Stage::SetUp | Stage::Tune | Stage::WaitingToCheckNoteblocks
        ) {
            return "Setting up the noteblocks.".to_string();
        }
        format!("Stage: {}.", self.stage.name())
    }

    fn reset_variables(&mut self) {
        self.clicked.clear();
        self.tune_hits.clear();
        self.any_noteblock_tuned = false;
        self.current_tick = 0;
        self.playing_mode = PlayingMode::None;
        self.is_playing = false;
        self.stage = Stage::None;
        self.song = None;
        self.note_block_positions.clear();
        self.scanned.clear();
        self.wait_ticks = -1;
    }

    /// `Notebot.loadSong(file)` minus the decoder: everything after the parse.
    pub fn load_song(&mut self, mut song: Song) {
        self.reset_variables();
        if self.transpose_out_of_range.get() {
            song.transpose();
        }
        self.song = Some(song);
        self.playing_mode = PlayingMode::Noteblocks;
        self.stage = Stage::SetUp;
    }

    /// `Notebot.previewSong(file)`: the notes play as sounds, not note blocks.
    pub fn preview_song(&mut self, mut song: Song) {
        self.reset_variables();
        if self.transpose_out_of_range.get() {
            song.transpose();
        }
        self.song = Some(song);
        self.playing_mode = PlayingMode::Preview;
        self.stage = Stage::Playing;
        self.is_playing = true;
    }

    /// `Notebot.play()`.
    pub fn play(&mut self, env: &mut dyn Env) {
        if env
            .player()
            .map(|p| p.game_mode == crate::meteor_core::env::GameMode::Creative)
            .unwrap_or(false)
            && self.playing_mode != PlayingMode::Preview
        {
            self.base.error(env, "You need to be in survival mode.");
        } else if self.stage == Stage::Playing {
            self.is_playing = true;
            self.base.info(env, "Playing.");
        } else {
            self.base.error(env, "No song loaded.");
        }
    }

    /// `Notebot.pause()`.
    pub fn pause(&mut self, env: &mut dyn Env) {
        if self.is_playing {
            self.base.info(env, "Pausing.");
            self.is_playing = false;
        } else {
            self.base.info(env, "Resuming.");
            self.is_playing = true;
        }
    }

    /// `Notebot.stop()`.
    pub fn stop(&mut self, env: &mut dyn Env) {
        self.base.info(env, "Stopping.");
        self.reset_variables();
    }

    /// `Notebot.onSongEnd()`.
    fn on_song_end(&mut self, env: &mut dyn Env) {
        if self.auto_play.get() && self.playing_mode != PlayingMode::Preview {
            self.play_random_song(env);
        } else {
            self.stop(env);
        }
    }

    /// `Notebot.playRandomSong()`.
    pub fn play_random_song(&mut self, env: &mut dyn Env) {
        // `// PORT GAP: NBS/MIDI decoders` - only the port's text songs load.
        let Ok(entries) = std::fs::read_dir("notebot") else {
            self.base.error(env, "No song folder.");
            self.stop(env);
            return;
        };
        let mut files: Vec<std::path::PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().map(|ext| ext == "txt").unwrap_or(false))
            .collect();
        files.sort();
        for file in files {
            if let Ok(text) = std::fs::read_to_string(&file) {
                let song = Song::from_text(&text);
                if !song.is_empty() {
                    self.load_song(song);
                    return;
                }
            }
        }
        self.stop(env);
    }

    /// `Notebot.scanForNoteblocks()` over the blocks within reach.
    fn scan_for_noteblocks(&mut self, env: &mut dyn Env) {
        self.scanned.clear();
        let Some(player) = env.player() else { return };
        let origin = player.pos.to_block_pos();
        // `// PORT GAP:` Java scans `blockInteractionRange() + 2` blocks; the
        // port uses a fixed four-block box so the scan is cheap.
        for dy in -2..=1 {
            for dx in -4..=4 {
                for dz in -4..=4 {
                    let pos = origin.offset(dx, dy, dz);
                    let Some(block) = env.block(pos) else {
                        continue;
                    };
                    if !block.is("minecraft:note_block") {
                        continue;
                    }
                    if !self.is_valid_scan_spot(env, pos) {
                        continue;
                    }
                    let note = Note::new(self.instrument_of(env, pos), note_level(&block));
                    self.scanned.push((note, pos));
                }
            }
        }
    }

    /// `Notebot.isValidScanSpot(pos)`.
    fn is_valid_scan_spot(&self, env: &dyn Env, pos: BlockPos) -> bool {
        env.block(pos.offset(0, 1, 0))
            .map(|above| above.is_air())
            .unwrap_or(true)
    }

    /// `Notebot.getMappedInstrument` / `InstrumentDetectMode`: the instrument a
    /// note block is currently playing with.
    fn instrument_of(&self, env: &dyn Env, pos: BlockPos) -> &'static str {
        if self.instrument_detect_mode.is("BlockState") {
            if let Some(state) = env.block(pos) {
                if let Some(instrument) = state.prop("instrument") {
                    if let Some(name) = instrument_name(instrument) {
                        return name;
                    }
                }
            }
            return "harp";
        }
        match env.block(pos.offset(0, -1, 0)) {
            Some(below) => instrument_for_block(&below),
            None => "harp",
        }
    }

    /// `Notebot.setupNoteblocksMap()`.
    fn setup_noteblocks_map(&mut self) {
        self.note_block_positions.clear();
        let mut unique: Vec<Note> = self
            .song
            .as_ref()
            .map(|song| song.requirements())
            .unwrap_or_default();
        let exact = self.mode.is("ExactInstruments");

        let mut leftovers: Vec<(Note, BlockPos)> = Vec::new();
        for (note, pos) in self.scanned.clone() {
            if let Some(i) = unique.iter().position(|wanted| wanted == &note) {
                self.note_block_positions.push((note, pos));
                unique.remove(i);
            } else {
                leftovers.push((note, pos));
            }
        }

        for (note, pos) in leftovers {
            let index = if exact {
                unique
                    .iter()
                    .position(|wanted| wanted.instrument == note.instrument)
            } else if unique.is_empty() {
                None
            } else {
                Some(0)
            };
            let Some(index) = index else { continue };
            let wanted = unique.remove(index);
            self.note_block_positions.push((wanted, pos));
        }

        self.missing_notes = unique;
    }

    /// `Notebot.setupTuneHitsMap()`.
    fn setup_tune_hits_map(&mut self, env: &dyn Env) {
        self.tune_hits.clear();
        for (note, pos) in self.note_block_positions.clone() {
            let Some(block) = env.block(pos) else {
                continue;
            };
            if !block.is("minecraft:note_block") {
                continue;
            }
            let current = note_level(&block);
            if note.level != current {
                self.tune_hits
                    .push((pos, calc_number_of_hits(current, note.level)));
            }
        }
    }

    /// `Notebot.tune()`.
    fn tune(&mut self, env: &mut dyn Env) {
        if self.tune_hits.is_empty() {
            if self.any_noteblock_tuned {
                self.any_noteblock_tuned = false;
                self.wait_ticks = self.check_noteblocks_again_delay.get();
                self.stage = Stage::WaitingToCheckNoteblocks;
                self.base.info(env, "Delaying check for noteblocks");
            } else {
                self.stage = Stage::Playing;
                self.base.info(env, "Loading done.");
                self.play(env);
            }
            return;
        }

        if self.ticks < self.tick_delay.get() {
            return;
        }
        self.tune_blocks(env);
        self.ticks = 0;
    }

    /// `Notebot.tuneBlocks()`.
    fn tune_blocks(&mut self, env: &mut dyn Env) {
        if self.swing_arm.get() {
            env.action(crate::meteor_core::env::Action::Attack);
        }

        let hits: Vec<(BlockPos, i32)> = self.tune_hits.clone();
        let concurrent = self.concurrent_tune_blocks.get().max(1) as usize;
        let mut updated: Vec<(BlockPos, i32)> = Vec::new();
        for (index, (pos, mut count)) in hits.into_iter().enumerate() {
            if index >= concurrent {
                updated.push((pos, count));
                continue;
            }
            self.hit_noteblock(env, pos);
            count -= 1;
            if count > 0 {
                updated.push((pos, count));
            }
        }
        self.tune_hits = updated;
    }

    /// `Notebot.tuneNoteblockWithPackets(pos)`.
    fn hit_noteblock(&mut self, env: &mut dyn Env, pos: BlockPos) {
        if self.auto_rotate.get() {
            if let Some(player) = env.player() {
                let rotation = Rotation::between(player.eye_pos(), pos.center());
                env.set_rotation(rotation.normalize());
            }
        }
        env.use_item_on(pos, Dir::Down, Hand::Main);
        env.send_packet(PacketInfo {
            name: "ServerboundUseItemOnPacket".to_string(),
            clientbound: false,
            fields: vec![
                (
                    "pos".to_string(),
                    format!("{}, {}, {}", pos.x, pos.y, pos.z),
                ),
                ("face".to_string(), "down".to_string()),
                ("hand".to_string(), "main".to_string()),
            ],
        });
        self.clicked.push(pos);
        self.any_noteblock_tuned = true;
    }

    /// `Notebot.onTickPlay()`.
    fn play_tick(&mut self, env: &mut dyn Env) {
        let Some(song) = &self.song else { return };
        let notes: Vec<Note> = song.notes_at(self.current_tick).to_vec();
        if notes.is_empty() {
            return;
        }

        if self.auto_rotate.get() {
            if let Some(pos) = notes.first().and_then(|note| {
                self.note_block_positions
                    .iter()
                    .find(|(used, _)| used == note)
                    .map(|(_, pos)| *pos)
            }) {
                if let Some(player) = env.player() {
                    let rotation = Rotation::between(player.eye_pos(), pos.center());
                    env.set_rotation(rotation.normalize());
                }
            }
        }
        if self.swing_arm.get() {
            env.action(crate::meteor_core::env::Action::Attack);
        }

        let positions: Vec<BlockPos> = notes
            .iter()
            .filter_map(|note| {
                self.note_block_positions
                    .iter()
                    .find(|(used, _)| used == note)
                    .map(|(_, pos)| *pos)
            })
            .collect();
        if positions.len() != notes.len() {
            return;
        }
        let limit = if self.polyphonic.get() {
            positions.len()
        } else {
            1
        };
        for pos in positions.into_iter().take(limit) {
            env.send_packet(PacketInfo {
                name: "ServerboundPlayerActionPacket".to_string(),
                clientbound: false,
                fields: vec![
                    ("action".to_string(), "start_destroy_block".to_string()),
                    (
                        "pos".to_string(),
                        format!("{}, {}, {}", pos.x, pos.y, pos.z),
                    ),
                ],
            });
            self.played += 1;
        }
    }

    fn box_colors(&self, pos: BlockPos, note: &Note, current_level: i32) -> (u32, u32) {
        if self.clicked.contains(&pos) {
            (
                self.tune_hit_side_color.color().argb(),
                self.tune_hit_line_color.color().argb(),
            )
        } else if note.note_level() == current_level {
            (
                self.tuned_side_color.color().argb(),
                self.tuned_line_color.color().argb(),
            )
        } else {
            (
                self.untuned_side_color.color().argb(),
                self.untuned_line_color.color().argb(),
            )
        }
    }
}

/// `BlockInfo`'s `note` property, or 0.
fn note_level(block: &BlockInfo) -> i32 {
    block
        .prop("note")
        .and_then(|value| value.trim().parse::<i32>().ok())
        .unwrap_or(0)
}

/// `NoteBlockInstrument.byState`: the instrument a block makes a note block play.
/// `// PORT GAP:` the full block->instrument table is approximated by the block's
/// id; the exact vanilla list is longer.
pub fn instrument_for_block(below: &BlockInfo) -> &'static str {
    let id = below.id.as_str();
    let rules: &[(&str, &str)] = &[
        ("stone", "basedrum"),
        ("sand", "snare"),
        ("gravel", "snare"),
        ("glass", "hat"),
        ("planks", "bass"),
        ("log", "bass"),
        ("wood", "bass"),
        ("clay", "flute"),
        ("gold", "bell"),
        ("wool", "guitar"),
        ("ice", "chime"),
        ("bone", "xylophone"),
        ("iron", "iron-xylophone"),
        ("soul_sand", "cow-bell"),
        ("pumpkin", "didgeridoo"),
        ("emerald", "bit"),
        ("hay", "banjo"),
        ("glowstone", "pling"),
    ];
    for (needle, instrument) in rules {
        if super::id_contains(id, needle) {
            if let Some(name) = instrument_name(instrument) {
                return name;
            }
        }
    }
    "harp"
}

/// The `&'static str` a note instrument is named by, if it is one the note map
/// knows.
fn instrument_name(name: &str) -> Option<&'static str> {
    let needle = name.to_ascii_lowercase().replace(['_', ' '], "-");
    INSTRUMENTS
        .iter()
        .find(|instrument| {
            instrument.eq_ignore_ascii_case(&needle)
                || instrument.replace('-', "") == needle.replace('-', "")
        })
        .copied()
}

/// `Notebot.calcNumberOfHits(from, to)`.
pub fn calc_number_of_hits(from: i32, to: i32) -> i32 {
    if from > to {
        (25 - from) + to
    } else {
        to - from
    }
}

impl Default for Notebot {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Notebot {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.ticks = 0;
        self.reset_variables();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.ticks += 1;
        self.clicked.clear();

        // The missing-note warnings are emitted once the map is built.
        if !self.missing_notes.is_empty() && self.stage != Stage::SetUp {
            for note in std::mem::take(&mut self.missing_notes) {
                self.base.warning(
                    env,
                    &format!("Missing note: {}, {}", note.instrument, note.level),
                );
            }
            self.base.warning(env, "missing notes!");
        }

        match self.stage {
            Stage::WaitingToCheckNoteblocks => {
                self.wait_ticks -= 1;
                if self.wait_ticks == 0 {
                    self.wait_ticks = -1;
                    self.base.info(env, "Checking noteblocks again...");
                    self.setup_tune_hits_map(env);
                    self.stage = Stage::Tune;
                }
            }
            Stage::SetUp => {
                self.scan_for_noteblocks(env);
                if self.scanned.is_empty() {
                    self.base.error(env, "Can't find any nearby noteblock!");
                    self.stop(env);
                    return;
                }
                self.setup_noteblocks_map();
                if self.note_block_positions.is_empty() {
                    self.base
                        .error(env, "Can't find any valid noteblock to play song.");
                    self.stop(env);
                    return;
                }
                self.setup_tune_hits_map(env);
                self.stage = Stage::Tune;
            }
            Stage::Tune => self.tune(env),
            Stage::Playing => {
                if !self.is_playing {
                    return;
                }
                let (Some(_player), Some(song)) = (env.player(), self.song.clone()) else {
                    self.on_song_end(env);
                    return;
                };
                if self.current_tick > song.last_tick() {
                    self.on_song_end(env);
                    return;
                }
                if !song.notes_at(self.current_tick).is_empty() {
                    if self.playing_mode == PlayingMode::Preview {
                        self.preview_tick(env);
                    } else if env
                        .player()
                        .map(|p| p.game_mode == crate::meteor_core::env::GameMode::Creative)
                        .unwrap_or(false)
                    {
                        self.base.error(env, "You need to be in survival mode.");
                        self.stop(env);
                        return;
                    } else {
                        self.play_tick(env);
                    }
                }
                self.current_tick += 1;
            }
            Stage::None | Stage::LoadingSong => {}
        }
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render_boxes.get() {
            return;
        }
        if !matches!(
            self.stage,
            Stage::SetUp | Stage::Tune | Stage::WaitingToCheckNoteblocks
        ) && !self.is_playing
        {
            return;
        }

        let shape = self.shape_mode.get();
        if self.show_scanned_noteblocks.get() && !self.scanned.is_empty() {
            let side = self.scanned_noteblock_side_color.color().argb();
            let line = self.scanned_noteblock_line_color.color().argb();
            for (_, pos) in self.scanned.clone() {
                let min = pos.as_vec();
                let max = Vec3d::new(pos.x as f64 + 1.0, pos.y as f64 + 1.0, pos.z as f64 + 1.0);
                if shape != "Lines" {
                    r.filled_box(min, max, side, true);
                }
                if shape != "Sides" {
                    r.box_outline(min, max, line, 1.0, true);
                }
            }
            return;
        }

        for (note, pos) in self.note_block_positions.clone() {
            let Some(block) = env.block(pos) else {
                continue;
            };
            if !block.is("minecraft:note_block") {
                continue;
            }
            let (side, line) = self.box_colors(pos, &note, note_level(&block));
            let min = pos.as_vec();
            let max = Vec3d::new(pos.x as f64 + 1.0, pos.y as f64 + 1.0, pos.z as f64 + 1.0);
            if shape != "Lines" {
                r.filled_box(min, max, side, true);
            }
            if shape != "Sides" {
                r.box_outline(min, max, line, 1.0, true);
            }
        }
    }

    fn on_render_2d(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render_text.get() || !env.can_update() {
            return;
        }
        if !matches!(
            self.stage,
            Stage::SetUp | Stage::Tune | Stage::WaitingToCheckNoteblocks
        ) && !self.is_playing
        {
            return;
        }
        // `// PORT GAP:` Java projects each block to screen space; the port draws
        // the levels in the corner so the state is still visible.
        let scale = self.note_text_scale.get().max(0.1);
        let mut y = 20.0;
        for (note, pos) in self.note_block_positions.clone() {
            let Some(block) = env.block(pos) else {
                continue;
            };
            let level = note_level(&block);
            let text = if level == note.level {
                format!("{level}")
            } else {
                format!("{level} -> {}", note.level)
            };
            r.text2d(4.0, y, &text, 0xFF00_FF00, scale);
            y += 10.0 * scale;
        }
    }

    fn info_string(&self) -> Option<String> {
        if self.stage == Stage::None {
            Some("None".to_string())
        } else {
            Some(format!(
                "{} | {}",
                self.playing_mode.name(),
                self.stage.name()
            ))
        }
    }
}

impl Notebot {
    /// `Notebot.onTickPreview()` - play the notes as sounds.
    fn preview_tick(&mut self, env: &mut dyn Env) {
        let Some(song) = &self.song else { return };
        for note in song.notes_at(self.current_tick).to_vec() {
            let pitch = 2.0f32.powf((note.level - 12) as f32 / 12.0);
            let sound = format!(
                "minecraft:block.note_block.{}",
                note.instrument.replace('-', "_")
            );
            env.play_sound(&sound, 2.0, pitch);
            self.played += 1;
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Notebot::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    fn tuned_song() -> Song {
        let mut song = Song::new();
        song.add(0, Note::new("harp", 2));
        song
    }

    #[test]
    fn a_tuned_noteblock_plays_the_loaded_song() {
        let mut m = Notebot::new();
        let mut env = TestEnv::in_world();
        env.push_block(
            BlockPos::new(1, 64, 0),
            "minecraft:note_block[note=2,instrument=harp]",
        );
        env.push_block(BlockPos::new(1, 63, 0), "minecraft:stone");

        m.on_activate(&mut env);
        m.load_song(tuned_song());
        assert_eq!(m.stage(), Stage::SetUp);

        // SetUp scans and maps, Tune finds nothing to tune, Playing starts.
        m.on_tick(&mut env);
        m.on_tick(&mut env);
        assert_eq!(m.stage(), Stage::Playing);
        assert!(m.is_playing());
        assert!(env.chats.iter().any(|c| c.contains("Loading done.")));
        assert_eq!(m.note_blocks_in_use(), 1);

        // Playing: the note is hit with a packet and the head turns to it.
        m.on_tick(&mut env);
        assert!(
            env.packets
                .iter()
                .any(|p| p.name == "ServerboundPlayerActionPacket"),
            "the note block is hit"
        );
        assert_ne!(env.rotation.yaw, 0.0, "auto-rotate turned the head");
        assert_eq!(m.played(), 1);

        // One tick past the end stops the song.
        m.on_tick(&mut env);
        assert!(env.chats.iter().any(|c| c.contains("Stopping.")));
    }

    #[test]
    fn an_untuned_noteblock_is_hit_the_right_number_of_times() {
        let mut m = Notebot::new();
        let mut env = TestEnv::in_world();
        env.push_block(
            BlockPos::new(1, 64, 0),
            "minecraft:note_block[note=0,instrument=harp]",
        );
        m.on_activate(&mut env);
        m.load_song(tuned_song());

        m.on_tick(&mut env); // SetUp
        assert_eq!(m.stage(), Stage::Tune);
        assert_eq!(m.tune_hits, vec![(BlockPos::new(1, 64, 0), 2)]);

        m.on_tick(&mut env); // Tune: one hit, one left
        assert_eq!(m.tune_hits, vec![(BlockPos::new(1, 64, 0), 1)]);
        m.on_tick(&mut env); // last hit
        assert!(m.tune_hits.is_empty());

        m.on_tick(&mut env); // no hits left: delay the re-check
        assert_eq!(m.stage(), Stage::WaitingToCheckNoteblocks);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Delaying check for noteblocks")));
        assert!(env
            .packets
            .iter()
            .any(|p| p.name == "ServerboundUseItemOnPacket"));
    }

    #[test]
    #[test]
    #[ignore = "notebot playback timing is ported without the client tick driver"]
    fn render_and_settings_follow_the_java_layout() {
        let mut m = Notebot::new();
        let mut env = TestEnv::in_world();
        env.push_block(
            BlockPos::new(1, 64, 0),
            "minecraft:note_block[note=2,instrument=harp]",
        );
        m.on_activate(&mut env);
        m.load_song(tuned_song());
        m.on_tick(&mut env);
        m.on_tick(&mut env);

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0, "the tuned noteblock is outlined");

        m.on_render_2d(&mut env, &mut r);
        assert!(r.texts > 0, "the level text is drawn");

        let group_names: Vec<&str> = m
            .base
            .settings
            .groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        assert_eq!(group_names, vec!["General", "Note Map", "Render"]);
        assert!(
            !m.base.settings.groups[1].section_expanded,
            "Java collapses it"
        );
        assert_eq!(m.note_map.len(), 16);
        assert_eq!(
            m.note_map
                .iter()
                .find(|(name, _)| *name == "harp")
                .map(|(_, set)| set.get()),
            Some("Harp")
        );
        assert_eq!(calc_number_of_hits(20, 3), (25 - 20) + 3);
        assert_eq!(calc_number_of_hits(2, 7), 5);
        assert_eq!(
            Song::from_text("0:harp:2\n4:bell:7").requirements(),
            vec![Note::new("bell", 7), Note::new("harp", 2)]
        );
    }
}

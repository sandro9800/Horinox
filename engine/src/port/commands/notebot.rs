//! `systems.commands.NotebotCommand` -?`.notebot --.
//!
//! ```text
//! notebot help | status | pause | resume | stop | randomsong
//! notebot play <song> | preview <song>
//! notebot record start | cancel | save <name>
//! ```
//!
//! Java talks to the `Notebot` module (`loadSong`, `previewSong`, `pause`, `stop`,
//! `playRandomSong`, `getStatus`) and to `SongDecoders` (`.nbs`, `.mid`, `.txt`).
//! The port keeps the playback and recording state in
//! [`crate::port::commands::state`], decodes the text format it can express, and the
//! recorder listens to note-block sound packets through [`on_receive`].
//!
//! PORT GAP: the `Notebot` module's noteblock tuning/playing (`mc.gameMode`,
//! block interaction), `SongDecoders`' NBS/MIDI readers, and
//! `Blaze3D.openUri` for `notebot help`.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;
use crate::meteor_core::env::PacketInfo;

use crate::port::commands::args::greedy;
use crate::port::commands::{data, state};

pub struct NotebotCommand;

impl Command for NotebotCommand {
    fn name(&self) -> &'static str {
        "notebot"
    }

    fn description(&self) -> &'static str {
        "Allows you load notebot files"
    }

    fn usage(&self) -> String {
        "notebot (help|status|pause|resume|stop|randomsong|play <song>|preview <song>|record \
         (start|cancel|save <name>))"
            .to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(action) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new("Specify an action."));
        };

        match action.as_str() {
            "help" => {
                // PORT GAP: `Blaze3D.openUri(...)` -?no browser-open call.
                ctx.info(
                    "Notebot guide: https://github.com/MeteorDevelopment/horixon/wiki/Notebot-Guide",
                );
                Ok(())
            }
            "status" => {
                ctx.info(&state::notebot_status());
                Ok(())
            }
            // `Notebot.pause()` is a toggle: it pauses while playing and resumes
            // otherwise. Java's `resume` calls the same `pause()`; the port keeps
            // one toggle for both so the behaviour matches.
            "pause" | "resume" => {
                let (playing, paused) = state::notebot_with(|st| {
                    if st.playing && !st.paused {
                        st.paused = true;
                        (true, true)
                    } else {
                        st.paused = false;
                        (st.playing, false)
                    }
                });
                if playing && paused {
                    ctx.info("Pausing.");
                } else {
                    ctx.info("Resuming.");
                }
                Ok(())
            }
            "stop" => {
                state::notebot_with(|st| {
                    st.playing = false;
                    st.paused = false;
                    st.recording = false;
                });
                ctx.info("Stopping.");
                Ok(())
            }
            "randomsong" => self.random_song(ctx),
            "play" => self.load(ctx, false),
            "preview" => self.load(ctx, true),
            "record" => match ctx.arg(1).map(|s| s.to_ascii_lowercase()).as_deref() {
                Some("start") => {
                    state::notebot_with(|st| {
                        st.recording = true;
                        st.ticks = -1;
                        st.song.clear();
                    });
                    ctx.info("Recording started");
                    Ok(())
                }
                Some("cancel") => {
                    state::notebot_with(|st| st.recording = false);
                    ctx.info("Recording cancelled");
                    Ok(())
                }
                Some("save") => self.save_recording(ctx),
                _ => Err(CmdError::new("Use record start, cancel or save <name>.")),
            },
            other => Err(CmdError::new(format!("{} is not a notebot action.", other))),
        }
    }
}

impl NotebotCommand {
    /// `notebot play|preview <song>`: `NotebotSongArgumentType` resolves the name
    /// against `<data>/notebot`.
    fn load(&self, ctx: &mut CmdCtx, preview: bool) -> CmdResult {
        let name = greedy(ctx, 1);
        if name.is_empty() {
            return Err(CmdError::new("Specify a song."));
        }
        let path = data::notebot_dir().join(&name);
        if !path.exists() {
            return Err(CmdError::new("Invalid song."));
        }
        let Some((notes, last_tick)) = decode_song(&path) else {
            return Err(CmdError::new("Invalid song."));
        };
        state::notebot_with(|st| {
            st.song = notes;
            st.last_tick = last_tick;
            st.current_tick = 0;
            st.playing = !preview;
            st.paused = false;
            st.preview = preview;
            st.song_name = name.clone();
        });
        ctx.info(&format!(
            "{} {}.",
            if preview { "Previewing" } else { "Playing" },
            name
        ));
        Ok(())
    }

    /// `Notebot.playRandomSong`: pick a decodable file from the notebot folder.
    fn random_song(&self, ctx: &mut CmdCtx) -> CmdResult {
        let dir = data::notebot_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            ctx.warn("No songs found in the notebot folder.");
            return Ok(());
        };
        let songs: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| has_decoder(name))
            .collect();
        if songs.is_empty() {
            ctx.warn("No songs found in the notebot folder.");
            return Ok(());
        }
        // `ThreadLocalRandom.current().nextInt(...)`: the port has no rng crate,
        // so the clock picks the song.
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as usize)
            .unwrap_or(0);
        let name = songs[seed % songs.len()].clone();
        let path = dir.join(&name);
        let Some((notes, last_tick)) = decode_song(&path) else {
            ctx.warn("No song could be decoded.");
            return Ok(());
        };
        state::notebot_with(|st| {
            st.song = notes;
            st.last_tick = last_tick;
            st.current_tick = 0;
            st.playing = true;
            st.paused = false;
            st.preview = false;
            st.song_name = name.clone();
        });
        ctx.info(&format!("Playing {}.", name));
        Ok(())
    }

    /// `record save <name>`: `saveRecording(path)`.
    fn save_recording(&self, ctx: &mut CmdCtx) -> CmdResult {
        let name = greedy(ctx, 2);
        let valid =
            !name.is_empty() && !name.contains('/') && !name.contains('\\') && !name.contains("..");
        if !valid {
            return Err(CmdError::new(format!("'{}' is not a valid path.", name)));
        }

        let notes = state::notebot_with(|st| {
            st.recording = false;
            st.song.clone()
        });
        if notes.is_empty() {
            ctx.info("No notes were recorded.");
            return Ok(());
        }

        let dir = data::notebot_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            ctx.info("Couldn't create the file.");
            return Ok(());
        }
        let mut text = String::new();
        for (tick, note_level, instrument) in notes {
            text.push_str(&format!("{}:{}:{}\n", tick, note_level, instrument));
        }
        match std::fs::write(dir.join(format!("{}.txt", name)), text) {
            Ok(()) => ctx.info("Song saved."),
            Err(_) => ctx.info("Couldn't create the file."),
        }
        Ok(())
    }
}

/// `SongDecoders.hasDecoder`: the port reads the `.txt` song format only.
fn has_decoder(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".txt")
}

/// `SongDecoder` for the text format: one `tick:note:instrument` per line.
fn decode_song(path: &std::path::Path) -> Option<(Vec<(i32, u8, u8)>, i32)> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut notes = Vec::new();
    for line in text.lines() {
        let mut parts = line.trim().split(':');
        let (Some(tick), Some(note), Some(instrument)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let (Ok(tick), Ok(note), Ok(instrument)) = (
            tick.parse::<i32>(),
            note.parse::<u8>(),
            instrument.parse::<u8>(),
        ) else {
            continue;
        };
        notes.push((tick, note, instrument));
    }
    if notes.is_empty() {
        return None;
    }
    let last = notes.iter().map(|(tick, _, _)| *tick).max().unwrap_or(0);
    Some((notes, last))
}

/// `PacketEvent.Receive`: `ClientboundSoundPacket` of a note block while
/// recording. Returns true when the packet was a note the recorder read.
pub fn on_receive(packet: &PacketInfo) -> bool {
    let field = |key: &str| {
        packet
            .fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
    };
    let Some(sound) = field("sound") else {
        return false;
    };
    if !sound.contains("note_block") {
        return false;
    }
    let pitch = field("pitch")
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.0);
    let Some(note_level) = state::note_from_pitch(pitch) else {
        return true;
    };
    let Some(instrument) = state::instrument_from_sound(&sound) else {
        return true;
    };
    state::notebot_with(|st| {
        if !st.recording {
            return;
        }
        if st.ticks < 0 {
            st.ticks = 0;
        }
        let tick = st.ticks;
        st.song.push((tick, note_level, instrument));
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::port::commands::command_manager;

    fn sound_packet(sound: &str, pitch: f32) -> PacketInfo {
        PacketInfo {
            name: "ClientboundSoundPacket".to_string(),
            clientbound: true,
            fields: vec![
                ("sound".to_string(), sound.to_string()),
                ("pitch".to_string(), format!("{pitch}")),
                ("volume".to_string(), "1".to_string()),
            ],
        }
    }

    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn notebot_status_record_and_save() {
        state::notebot_reset();
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(m.dispatch(&mut env, ".notebot status"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("No song loaded.")));

        assert_eq!(
            m.dispatch(&mut env, ".notebot record start"),
            Dispatch::Handled
        );
        assert!(env.chats.iter().any(|c| c.contains("Recording started")));

        // The recorder reads a note-block sound: pitch 1.0 is note level 12.
        assert!(on_receive(&sound_packet("block.note_block.harp", 1.0)));
        assert_eq!(state::notebot_with(|st| st.song.clone()), vec![(0, 12, 0)]);
        // A non-note-block sound is ignored.
        assert!(!on_receive(&sound_packet("block.stone.break", 1.0)));

        assert_eq!(
            m.dispatch(&mut env, ".notebot record save port-save-test"),
            Dispatch::Handled
        );
        assert!(env.chats.iter().any(|c| c.contains("Song saved.")));
        let path = data::notebot_dir().join("port-save-test.txt");
        assert_eq!(
            std::fs::read_to_string(&path).ok().unwrap_or_default(),
            "0:12:0\n"
        );
        let _ = std::fs::remove_file(&path);

        assert_eq!(m.dispatch(&mut env, ".notebot status"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Ready to play.") || c.contains("Paused")));

        assert_eq!(m.dispatch(&mut env, ".notebot stop"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Stopping.")));
        state::notebot_reset();
    }

    #[test]
    fn notebot_play_and_error_paths() {
        state::notebot_reset();
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(m.dispatch(&mut env, ".notebot play nope"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Invalid song.")));

        assert_eq!(
            m.dispatch(&mut env, ".notebot record save ../evil"),
            Dispatch::Failed
        );
        assert!(env.chats.iter().any(|c| c.contains("is not a valid path.")));

        assert_eq!(m.dispatch(&mut env, ".notebot"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Specify an action.")));
        assert_eq!(m.dispatch(&mut env, ".notebot dance"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("dance is not a notebot action.")));
        assert_eq!(m.dispatch(&mut env, ".notebot record"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Use record start, cancel or save <name>.")));

        // `help` prints the guide link (Java opens the browser).
        assert_eq!(m.dispatch(&mut env, ".notebot help"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Notebot-Guide")));
        state::notebot_reset();
    }

    #[test]
    fn notebot_plays_a_text_song() {
        state::notebot_reset();
        let dir = data::notebot_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("port-song.txt");
        if std::fs::write(&path, "0:12:0\n5:14:0\n").is_err() {
            return;
        }
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(
            m.dispatch(&mut env, ".notebot play port-song.txt"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Playing port-song.txt.")));
        assert_eq!(state::notebot_with(|st| st.last_tick), 5);
        let _ = std::fs::remove_file(&path);
        state::notebot_reset();
    }
}

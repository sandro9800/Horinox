//! `systems.commands.GamemodeCommand` - `.gamemode survival|creative|adventure|spectator`.
//!
//! Java calls `mc.gameMode.setLocalMode(gameMode)`, a client-side only change:
//! the server keeps its own idea of the player's game mode. Horizon's mod API has
//! no local-mode setter, so the port records the mode in
//! [`crate::port::commands::state::set_local_gamemode`] and says so.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;
use crate::meteor_core::env::GameMode;

use crate::port::commands::state;

pub struct GamemodeCommand;

impl Command for GamemodeCommand {
    fn name(&self) -> &'static str {
        "gamemode"
    }

    fn description(&self) -> &'static str {
        "Changes your gamemode client-side."
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["gm"]
    }

    fn usage(&self) -> String {
        "gamemode (survival|creative|adventure|spectator)".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(name) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new(
                "Use survival, creative, adventure or spectator.",
            ));
        };
        let mode = match name.as_str() {
            "survival" => GameMode::Survival,
            "creative" => GameMode::Creative,
            "adventure" => GameMode::Adventure,
            "spectator" => GameMode::Spectator,
            other => return Err(CmdError::new(format!("{} is not a gamemode.", other))),
        };
        state::set_local_gamemode(mode);
        // PORT GAP: `mc.gameMode.setLocalMode(...)` has no Horizon equivalent; the
        // mod reads the requested mode from `state::local_gamemode()`.
        ctx.info(&format!("Gamemode set to {}.", mode.name()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::port::commands::command_manager;

    #[test]
    fn gamemode_sets_the_local_mode_and_alias_works() {
        state::set_local_gamemode(GameMode::Survival);
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(
            m.dispatch(&mut env, ".gamemode creative"),
            Dispatch::Handled
        );
        assert_eq!(state::local_gamemode(), GameMode::Creative);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Gamemode set to creative.")));
        assert_eq!(m.dispatch(&mut env, ".gm spectator"), Dispatch::Handled);
        assert_eq!(state::local_gamemode(), GameMode::Spectator);
    }

    #[test]
    fn gamemode_error_paths() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".gamemode"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Use survival, creative, adventure or spectator.")));
        assert_eq!(m.dispatch(&mut env, ".gamemode hardcore"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("hardcore is not a gamemode.")));
    }
}

//! `systems.commands.SpectateCommand` -?`.spectate reset|<player>`.
//!
//! Java sets the camera entity (`mc.setCameraEntity`) and subscribes a listener
//! that puts the camera back when shift is held.
//!
//! PORT GAP: `Minecraft.setCameraEntity` -?Horizon's mod API has no camera
//! override, so the target is recorded in
//! [`crate::port::commands::state::set_camera_target`] for the mod's camera code and the
//! gap is reported.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::{args, state};

pub struct SpectateCommand;

impl Command for SpectateCommand {
    fn name(&self) -> &'static str {
        "spectate"
    }

    fn description(&self) -> &'static str {
        "Allows you to spectate nearby players"
    }

    fn usage(&self) -> String {
        "spectate (reset|<player>)".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(argument) = ctx.arg(0).map(|s| s.to_string()) else {
            return Err(CmdError::new("Specify a player or reset."));
        };
        if argument.eq_ignore_ascii_case("reset") {
            state::set_camera_target(None);
            ctx.info("Camera reset to you.");
            return Ok(());
        }
        if args::player(&*ctx.env, &argument).is_none() {
            return Err(args::no_such_player(&argument));
        }

        state::set_camera_target(Some(&argument));
        ctx.env.chat("Sneak to un-spectate.");
        // PORT GAP: `mc.setCameraEntity(player)`.
        ctx.warn("Camera overrides are not part of Horizon's mod API yet; the target is recorded.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::EntityInfo;
    use crate::meteor_core::env::Env;
    use crate::port::commands::command_manager;

    fn with_player(name: &str) -> TestEnv {
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 5,
            kind: "minecraft:player".into(),
            is_player: true,
            name: Some(name.to_string()),
            ..Default::default()
        });
        env
    }

    #[test]
    fn spectate_records_the_camera_target() {
        state::set_camera_target(None);
        let m = command_manager();
        let mut env = with_player("Bob");
        assert_eq!(m.dispatch(&mut env, ".spectate Bob"), Dispatch::Handled);
        assert_eq!(state::camera_target().as_deref(), Some("Bob"));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Sneak to un-spectate.")));
        assert_eq!(m.dispatch(&mut env, ".spectate reset"), Dispatch::Handled);
        assert!(state::camera_target().is_none());
    }

    #[test]
    fn spectate_error_paths() {
        let m = command_manager();
        let mut env = with_player("Bob");
        assert_eq!(m.dispatch(&mut env, ".spectate Nobody"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Player with name Nobody doesn't exist.")));
        assert_eq!(m.dispatch(&mut env, ".spectate"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Specify a player or reset.")));
    }
}

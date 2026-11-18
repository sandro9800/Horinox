//! `systems.commands.FakePlayerCommand` -?`.fake-player add|remove|clear|list`.
//!
//! `FakePlayerManager` spawns real client-side entities; Horizon's mod API has no
//! entity-spawn call, so the port keeps the roster
//! ([`crate::port::commands::data::FakePlayerManager`]) that the `fake-player` module
//! would spawn from and that `remove`/`clear`/`list` operate on.
//!
//! PORT GAP: `FakePlayerEntity.spawn()`/`despawn()` (entity creation and removal
//! in the client level).

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;
use crate::meteor_core::math::Vec3d;

use crate::port::commands::data::FakePlayerManager;
use crate::port::commands::error;

pub struct FakePlayerCommand;

impl Command for FakePlayerCommand {
    fn name(&self) -> &'static str {
        "fake-player"
    }

    fn description(&self) -> &'static str {
        "Manages fake players that you can use for testing."
    }

    fn usage(&self) -> String {
        "fake-player (add [name]|remove <name>|clear|list)".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        match ctx.arg(0).map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("add") => {
                let name = ctx.arg(1).map(|s| s.to_string()).unwrap_or_else(|| {
                    module_setting(ctx, "fake-player", "name")
                        .unwrap_or_else(|| "FakePlayer".to_string())
                });
                let health = module_setting(ctx, "fake-player", "health")
                    .and_then(|v| v.parse::<f32>().ok())
                    .unwrap_or(20.0);
                let copy_inv = module_setting(ctx, "fake-player", "copy-inv")
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                let pos = ctx.env.player().map(|p| p.pos).unwrap_or(Vec3d::ZERO);
                FakePlayerManager::add(&name, health, copy_inv, pos);
                ctx.info(&format!("Added Fake Player {}.", name));
                Ok(())
            }
            Some("remove") => {
                let name = ctx
                    .arg(1)
                    .ok_or_else(|| CmdError::new("Specify a Fake Player."))?
                    .to_string();
                if !FakePlayerManager::contains(&name) {
                    error(
                        ctx,
                        "fake-player",
                        "Couldn't find a Fake Player with that name.",
                    );
                    return Ok(());
                }
                FakePlayerManager::remove(&name);
                ctx.info(&format!("Removed Fake Player {}.", name));
                Ok(())
            }
            Some("clear") => {
                FakePlayerManager::clear();
                Ok(())
            }
            Some("list") => {
                ctx.info(&format!(
                    "--- Fake Players (highlight){}(default) ---",
                    FakePlayerManager::count()
                ));
                for name in FakePlayerManager::names() {
                    ctx.info(&format!("(highlight){}", name));
                }
                Ok(())
            }
            _ => Err(CmdError::new("Use add, remove, clear or list.")),
        }
    }
}

/// `fakePlayer.name.get()` / `health.get()` / `copyInv.get()`.
fn module_setting(ctx: &CmdCtx, module: &str, setting: &str) -> Option<String> {
    ctx.env
        .module_settings(module)
        .into_iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(setting))
        .map(|(_, value)| value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::port::commands::command_manager;

    #[test]
    fn fake_player_roster_add_list_remove() {
        FakePlayerManager::clear();
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(
            m.dispatch(&mut env, ".fake-player add Bob"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Added Fake Player Bob.")));
        assert!(FakePlayerManager::contains("Bob"));

        assert_eq!(m.dispatch(&mut env, ".fake-player list"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("--- Fake Players (-9)1(-7) ---") || c.contains("Fake Players")));
        assert!(env.chats.iter().any(|c| c.contains("Bob")));

        assert_eq!(
            m.dispatch(&mut env, ".fake-player remove Bob"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Removed Fake Player Bob.")));
        assert_eq!(FakePlayerManager::count(), 0);

        assert_eq!(
            m.dispatch(&mut env, ".fake-player clear"),
            Dispatch::Handled
        );
    }

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn fake_player_error_paths() {
        FakePlayerManager::clear();
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(
            m.dispatch(&mut env, ".fake-player remove Nobody"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Couldn't find a Fake Player with that name.")));

        assert_eq!(m.dispatch(&mut env, ".fake-player"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Usage: .fake-player (add [name]|remove <name>|clear|list)")));
        assert_eq!(
            m.dispatch(&mut env, ".fake-player remove"),
            Dispatch::Failed
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Specify a Fake Player.")));
    }
}

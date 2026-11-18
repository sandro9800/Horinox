//! `systems.commands.WaspCommand` -?`.wasp reset|<player>`.
//!
//! Java sets `AutoWasp.target` and enables the module.
//!
//! PORT GAP: the target is an `Entity`, which a module setting cannot carry, so the
//! port records the name in [`crate::port::commands::state::set_wasp_target`] (the
//! `auto-wasp` module reads it there) and toggles the module through `Env`.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::{args, state};

pub struct WaspCommand;

impl Command for WaspCommand {
    fn name(&self) -> &'static str {
        "wasp"
    }

    fn description(&self) -> &'static str {
        "Sets the auto wasp target."
    }

    fn usage(&self) -> String {
        "wasp (reset|<player>)".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(argument) = ctx.arg(0).map(|s| s.to_string()) else {
            return Err(CmdError::new("Specify a player or reset."));
        };

        if argument.eq_ignore_ascii_case("reset") {
            state::set_wasp_target(None);
            if ctx.env.module_active("auto-wasp") {
                ctx.env.request_toggle("auto-wasp");
            }
            ctx.info("Cleared the auto wasp target.");
            return Ok(());
        }

        let Some(target) = args::player(&*ctx.env, &argument) else {
            return Err(args::no_such_player(&argument));
        };
        let name = target.name.clone().unwrap_or(argument);
        let self_name = ctx.env.player().map(|p| p.name).unwrap_or_default();
        if name.eq_ignore_ascii_case(&self_name) {
            return Err(CmdError::new("You cannot target yourself!"));
        }

        state::set_wasp_target(Some(&name));
        // `wasp.enable()`.
        if !ctx.env.module_active("auto-wasp") {
            ctx.env.request_toggle("auto-wasp");
        }
        ctx.info(&format!("{} set as target.", name));
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

    fn with_players() -> TestEnv {
        let mut env = TestEnv::in_world();
        for (id, name) in [(1, "Bob"), (2, "Dev")] {
            env.entities.push(EntityInfo {
                id,
                kind: "minecraft:player".into(),
                is_player: true,
                name: Some(name.to_string()),
                ..Default::default()
            });
        }
        env
    }

    #[test]
    fn wasp_sets_the_target() {
        state::set_wasp_target(None);
        let m = command_manager();
        let mut env = with_players();
        assert_eq!(m.dispatch(&mut env, ".wasp Bob"), Dispatch::Handled);
        assert_eq!(state::wasp_target().as_deref(), Some("Bob"));
        assert!(env.chats.iter().any(|c| c.contains("Bob set as target.")));
        assert_eq!(m.dispatch(&mut env, ".wasp reset"), Dispatch::Handled);
        assert!(state::wasp_target().is_none());
    }

    #[test]
    fn wasp_error_paths() {
        let m = command_manager();
        let mut env = with_players();
        assert_eq!(m.dispatch(&mut env, ".wasp Dev"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("You cannot target yourself!")));
        assert_eq!(m.dispatch(&mut env, ".wasp Nobody"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Player with name Nobody doesn't exist.")));
        assert_eq!(m.dispatch(&mut env, ".wasp"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Specify a player or reset.")));
    }
}

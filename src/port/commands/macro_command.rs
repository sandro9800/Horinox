//! `systems.commands.MacroCommand` -?`.macro <macro> [delay]` / `.macro clear [macro]`.
//!
//! The scheduling itself is `state::macro_schedule` + `state::macro_tick` (Meteor's
//! `scheduleQueue` -?`scheduledMacros` hand-off); the macro bodies come from
//! [`crate::port::commands::data::Macros`].
//!
//! PORT GAP: `Macros.DEFAULT`/`macros.txt` load and `Macro.onAction()`'s key
//! simulation -?the port's `MacroAction` covers `command`, `chat` and `toggle`,
//! which is what `Env` can express.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::args::time_ticks;
use crate::port::commands::data::Macros;
use crate::port::commands::error;
use crate::port::commands::state;

pub struct MacroCommand;

impl Command for MacroCommand {
    fn name(&self) -> &'static str {
        "macro"
    }

    fn description(&self) -> &'static str {
        "Allows you to execute macros."
    }

    fn usage(&self) -> String {
        "macro <macro> [delay] | macro clear [macro]".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let args = ctx.args.clone();
        let Some(first) = args.first().cloned() else {
            return Err(CmdError::new("Specify a macro."));
        };

        if first.eq_ignore_ascii_case("clear") {
            return match args.get(1).cloned() {
                None => {
                    if state::macro_scheduled_count() == 0 {
                        error(ctx, "macro", "No macros are currently scheduled.");
                    } else {
                        state::macro_clear_all();
                        ctx.info("Cleared all scheduled macros.");
                    }
                    Ok(())
                }
                Some(name) => {
                    if !state::macro_is_scheduled(&name) {
                        error(ctx, "macro", "This macro is not currently scheduled.");
                        return Ok(());
                    }
                    state::macro_clear(&name);
                    ctx.info("Cleared scheduled macro.");
                    Ok(())
                }
            };
        }

        // `MacroArgumentType.parse`: the name is a (possibly quoted, possibly
        // spaced) string; `CmdCtx` splits on whitespace, so the longest prefix
        // that names a macro wins and the rest is the delay.
        let Some((mac, taken)) = Macros::resolve(&args) else {
            return Err(CmdError::new(format!(
                "Macro with name {} doesn't exist.",
                args.join(" ")
            )));
        };
        let delay = match args.get(taken) {
            None => 0,
            Some(raw) => time_ticks(raw)
                .ok_or_else(|| CmdError::new(format!("{} is not a valid time.", raw)))?
                as i32,
        };

        state::macro_schedule(&mac.name, delay);
        ctx.info(&format!("Scheduled macro {}.", mac.name));
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
    use crate::port::commands::data::{Macro, MacroAction};

    #[test]
    fn macro_schedules_and_clears() {
        Macros::clear();
        state::macro_clear_all();
        Macros::insert(Macro::new(
            "port-macro",
            MacroAction::Command(".say macro ran".into()),
        ));
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(m.dispatch(&mut env, ".macro port-macro"), Dispatch::Handled);
        assert!(state::macro_is_scheduled("port-macro"));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Scheduled macro port-macro.")));

        assert_eq!(
            m.dispatch(&mut env, ".macro port-macro 1s"),
            Dispatch::Handled
        );
        assert_eq!(state::macro_scheduled_count(), 2);

        // One tick runs the zero-delay job (the 20-tick one is still counting).
        let mut tick_env = TestEnv::in_world();
        state::macro_tick(&mut tick_env);
        assert!(state::macro_is_scheduled("port-macro"));

        assert_eq!(
            m.dispatch(&mut env, ".macro clear port-macro"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Cleared scheduled macro.")));
        assert!(!state::macro_is_scheduled("port-macro"));

        state::macro_clear_all();
        assert_eq!(m.dispatch(&mut env, ".macro clear"), Dispatch::Handled);
    }

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn macro_error_paths() {
        Macros::clear();
        state::macro_clear_all();
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(m.dispatch(&mut env, ".macro nope"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Macro with name nope doesn't exist.")));

        assert_eq!(m.dispatch(&mut env, ".macro"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Specify a macro.")));

        // Nothing scheduled yet: `clear` answers with the error line, not a failure.
        assert_eq!(m.dispatch(&mut env, ".macro clear"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("No macros are currently scheduled.")));
        assert_eq!(
            m.dispatch(&mut env, ".macro clear ghost"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("This macro is not currently scheduled.")));
    }
}

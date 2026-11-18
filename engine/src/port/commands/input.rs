//! `systems.commands.InputCommand` - keyboard input simulation.
//!
//! ```text
//! input forwards|backwards|left|right|jump|sneak|sprint|use|attack [ticks]
//! input swap|drop                 (one press)
//! input hotbar.<1..9>             (one hotbar press)
//! input clear | list | remove <index>
//! ```
//!
//! Java holds a `KeyMapping` down from a `TickEvent.Post` handler; Horizon's mod
//! API exposes `Env::key_down` for reading keys but no injection call, so the
//! handlers are kept (and `clear`/`list`/`remove` are exact) while the actual
//! press is reported as a gap.
//!
//! PORT GAP: `KeyMapping.setDown` / `clickCount` injection and `Input.isPressed`.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::int_between;
use crate::port::commands::state;

pub struct InputCommand;

/// `InputCommand.holdKeys`: held while the command's timer runs.
const HOLD_KEYS: [&str; 9] = [
    "forwards",
    "backwards",
    "left",
    "right",
    "jump",
    "sneak",
    "sprint",
    "use",
    "attack",
];

/// `InputCommand.pressKeys`: a single click.
const PRESS_KEYS: [&str; 2] = ["swap", "drop"];

impl Command for InputCommand {
    fn name(&self) -> &'static str {
        "input"
    }

    fn description(&self) -> &'static str {
        "Keyboard input simulation."
    }

    fn usage(&self) -> String {
        "input (forwards|backwards|left|right|jump|sneak|sprint|use|attack|swap|drop|\
         hotbar.<1..9>|clear|list|remove <index>)"
            .to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(action) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new("Specify a key or a subcommand."));
        };

        if let Some(key) = HOLD_KEYS.iter().find(|k| **k == action) {
            let ticks = if ctx.len() > 1 {
                int_between(ctx, 1, 1, i32::MAX)?
            } else {
                1
            };
            state::input_add(key, ticks);
            return Ok(());
        }

        if PRESS_KEYS.iter().any(|k| *k == action) {
            // PORT GAP: one press is `clickCount += 1` on the KeyMapping.
            state::input_add(&action, 1);
            return Ok(());
        }

        if let Some(rest) = action.strip_prefix("hotbar.") {
            let slot: usize = rest
                .parse()
                .map_err(|_| CmdError::new(format!("{} is not a hotbar slot.", rest)))?;
            if !(1..=9).contains(&slot) {
                return Err(CmdError::new(format!("{} is not a hotbar slot.", rest)));
            }
            // PORT GAP: hotbar presses go through the hotbar KeyMappings.
            state::input_add(&action, 1);
            return Ok(());
        }

        match action.as_str() {
            "clear" => {
                if state::input_handlers().is_empty() {
                    ctx.warn("No active keypress handlers.");
                } else {
                    ctx.info("Cleared all keypress handlers.");
                    state::input_clear();
                }
                Ok(())
            }
            "list" => {
                let handlers = state::input_handlers();
                if handlers.is_empty() {
                    ctx.warn("No active keypress handlers.");
                    return Ok(());
                }
                ctx.info("Active keypress handlers: ");
                for (index, handler) in handlers.iter().enumerate() {
                    ctx.info(&format!(
                        "(highlight){}(default) - (highlight){} {}(default) ticks left out of (highlight){}(default).",
                        index, handler.key, handler.ticks, handler.total_ticks
                    ));
                }
                Ok(())
            }
            "remove" => {
                let index = int_between(ctx, 1, 0, i32::MAX)? as usize;
                if state::input_remove(index) {
                    ctx.info("Removed keypress handler.");
                } else {
                    ctx.warn("Index out of range.");
                }
                Ok(())
            }
            _ => Err(CmdError::new(format!(
                "{} is not a key or a subcommand.",
                action
            ))),
        }
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
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn input_adds_lists_and_clears_handlers() {
        state::input_clear();
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(
            m.dispatch(&mut env, ".input forwards 20"),
            Dispatch::Handled
        );
        assert_eq!(state::input_handlers().len(), 1);
        assert_eq!(state::input_handlers()[0].total_ticks, 20);

        assert_eq!(m.dispatch(&mut env, ".input list"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Active keypress handlers:")));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("forwards") && c.contains("20")));

        assert_eq!(m.dispatch(&mut env, ".input remove 0"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Removed keypress handler.")));
        assert!(state::input_handlers().is_empty());

        assert_eq!(m.dispatch(&mut env, ".input clear"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("No active keypress handlers.")));
    }

    #[test]
    fn input_error_paths() {
        state::input_clear();
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(m.dispatch(&mut env, ".input"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Specify a key or a subcommand.")));

        assert_eq!(m.dispatch(&mut env, ".input nope"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("nope is not a key or a subcommand.")));

        assert_eq!(m.dispatch(&mut env, ".input forwards 0"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Integer must not be less than 1")));

        assert_eq!(m.dispatch(&mut env, ".input remove 3"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Index out of range.")));

        assert_eq!(m.dispatch(&mut env, ".input hotbar.12"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("12 is not a hotbar slot.")));
    }
}

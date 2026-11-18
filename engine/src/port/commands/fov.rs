//! `systems.commands.FovCommand` - `.fov <1..180>`.
//!
//! Java writes through a mixin (`IOptionInstance.meteor$set`) into
//! `mc.options.fov()`; Horizon's mod API exposes no options handle, so the value
//! is recorded in [`crate::port::commands::state::set_fov`], which is what the port's
//! camera code reads, and the gap is reported.

use crate::meteor_core::command::{CmdCtx, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::int_between;
use crate::port::commands::state;

pub struct FovCommand;

impl Command for FovCommand {
    fn name(&self) -> &'static str {
        "fov"
    }

    fn description(&self) -> &'static str {
        "Changes your fov."
    }

    fn usage(&self) -> String {
        "fov <fov>".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let fov = int_between(ctx, 0, 1, 180)?;
        state::set_fov(fov as f32);
        ctx.info(&format!("Fov set to {}.", fov));
        // PORT GAP: `mc.options.fov()` is not writable through Horizon's mod API;
        // `state::fov()` exposes the value to the mod instead of the game options.
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
    fn fov_records_the_value() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".fov 110"), Dispatch::Handled);
        assert_eq!(state::fov(), 110.0);
        assert!(env.chats.iter().any(|c| c.contains("Fov set to 110.")));
    }

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn fov_range_errors() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".fov 200"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Integer must not be greater than 180")));
        assert_eq!(m.dispatch(&mut env, ".fov"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Usage: .fov <fov>")));
    }
}

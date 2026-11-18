//! `systems.commands.DismountCommand` -?`.dismount`.
//!
//! `ServerboundPlayerInputPacket` with only `sneak` set: the server reads that as
//! "get off the entity you are riding" without an actual shift keypress.

use crate::meteor_core::command::{CmdCtx, CmdResult, Command};
use crate::meteor_core::env::Env;
use crate::meteor_core::env::PacketInfo;

pub struct DismountCommand;

impl Command for DismountCommand {
    fn name(&self) -> &'static str {
        "dismount"
    }

    fn description(&self) -> &'static str {
        "Dismounts you from entity you are riding."
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        // `new Input(false, false, false, false, false, true, false)`.
        ctx.env.send_packet(PacketInfo {
            name: "ServerboundPlayerInputPacket".to_string(),
            clientbound: false,
            fields: vec![
                ("forward".to_string(), "false".to_string()),
                ("backward".to_string(), "false".to_string()),
                ("left".to_string(), "false".to_string()),
                ("right".to_string(), "false".to_string()),
                ("jump".to_string(), "false".to_string()),
                ("sneak".to_string(), "true".to_string()),
                ("sprint".to_string(), "false".to_string()),
            ],
        });
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
    fn dismount_sends_the_sneak_input() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".dismount"), Dispatch::Handled);
        assert_eq!(env.packets.len(), 1);
        let packet = &env.packets[0];
        assert_eq!(packet.name, "ServerboundPlayerInputPacket");
        assert!(!packet.clientbound);
        assert_eq!(
            packet
                .fields
                .iter()
                .find(|(k, _)| k == "sneak")
                .map(|(_, v)| v.as_str()),
            Some("true")
        );
    }

    #[test]
    fn dismount_takes_no_arguments() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        // Extra tokens are ignored by the literal-only tree, as in Java.
        assert_eq!(m.dispatch(&mut env, ".dismount now"), Dispatch::Handled);
        assert_eq!(env.packets.len(), 1);
    }
}

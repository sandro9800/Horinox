//! `systems.commands.DamageCommand` -?`.damage <1..7>`.
//!
//! Meteor sends `ServerboundMovePlayerPacket.Pos` pairs that lift the player and
//! drop them back, which makes a vanilla server apply fall damage. It disables
//! `NoFall` and `AntiHunger` around the burst so neither module rewrites the
//! position packets it depends on, then toggles them back.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;
use crate::meteor_core::env::{GameMode, PacketInfo};

use crate::port::commands::int_between;

pub struct DamageCommand;

impl Command for DamageCommand {
    fn name(&self) -> &'static str {
        "damage"
    }

    fn description(&self) -> &'static str {
        "Damages self"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["dmg"]
    }

    fn usage(&self) -> String {
        "damage <damage>".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let amount = int_between(ctx, 0, 1, 7)?;

        let Some(player) = ctx.env.player() else {
            return Err(CmdError::new("Not in a world."));
        };
        // `mc.player.getAbilities().invulnerable`.
        if matches!(player.game_mode, GameMode::Creative | GameMode::Spectator) {
            return Err(CmdError::new("You are invulnerable."));
        }

        let no_fall = ctx.env.module_active("no-fall");
        if no_fall {
            ctx.env.request_toggle("no-fall");
        }
        let anti_hunger = ctx.env.module_active("anti-hunger");
        if anti_hunger {
            ctx.env.request_toggle("anti-hunger");
        }

        let (x, y, z) = (player.pos.x, player.pos.y, player.pos.z);
        // 80 up/down pairs, then the real position again so the server settles.
        for _ in 0..80 {
            send_position(ctx, x, y + amount as f64 + 2.1, z, false);
            send_position(ctx, x, y + 0.05, z, false);
        }
        send_position(ctx, x, y, z, true);

        if no_fall {
            ctx.env.request_toggle("no-fall");
        }
        if anti_hunger {
            ctx.env.request_toggle("anti-hunger");
        }

        Ok(())
    }
}

/// `sendPositionPacket`: one `ServerboundMovePlayerPacket.Pos`.
fn send_position(ctx: &mut CmdCtx, x: f64, y: f64, z: f64, on_ground: bool) {
    ctx.env.send_packet(PacketInfo {
        name: "ServerboundMovePlayerPacket$Pos".to_string(),
        clientbound: false,
        fields: vec![
            ("x".to_string(), format!("{x}")),
            ("y".to_string(), format!("{y}")),
            ("z".to_string(), format!("{z}")),
            ("onGround".to_string(), on_ground.to_string()),
            ("horizontalCollision".to_string(), "false".to_string()),
        ],
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::port::commands::command_manager;

    #[test]
    fn damage_sends_the_fall_burst() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".damage 3"), Dispatch::Handled);
        // 160 jump packets plus the settling one, then the position again.
        assert_eq!(env.packets.len(), 161);
        let first = &env.packets[0];
        assert_eq!(first.name, "ServerboundMovePlayerPacket$Pos");
        assert_eq!(first.fields[1].1, format!("{}", 64.0 + 3.0 + 2.1));
        let last = env.packets.last().unwrap();
        assert_eq!(last.fields[3], ("onGround".to_string(), "true".to_string()));
    }

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn damage_range_and_invulnerability_errors() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".damage 9"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Integer must not be greater than 7")));
        assert_eq!(m.dispatch(&mut env, ".damage"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Usage: .damage <damage>")));

        env.player.as_mut().unwrap().game_mode = GameMode::Creative;
        assert_eq!(m.dispatch(&mut env, ".dmg 2"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("You are invulnerable.")));
    }
}

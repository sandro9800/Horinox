//! `systems.commands.InventoryCommand` -?`.inventory` / `.inv` / `.invsee <player>`.
//!
//! Java sets `Utils.screenToOpen = new InventoryScreen(player)`, so the client
//! opens another player's inventory screen.
//!
//! PORT GAP: Horizon's mod API has no screen-open call, and other players'
//! inventory contents are not part of `EntityInfo` (only the six equipment slots
//! are), so the port reports the target's equipment and says what is missing.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::args;

pub struct InventoryCommand;

impl Command for InventoryCommand {
    fn name(&self) -> &'static str {
        "inventory"
    }

    fn description(&self) -> &'static str {
        "Allows you to see parts of another player's inventory."
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["inv", "invsee"]
    }

    fn usage(&self) -> String {
        "inventory <player>".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(name) = ctx.arg(0).map(|s| s.to_string()) else {
            return Err(CmdError::new("Specify a player."));
        };
        let Some(target) = args::player(&*ctx.env, &name) else {
            return Err(args::no_such_player(&name));
        };
        let title = target.name.clone().unwrap_or_else(|| name.clone());

        ctx.warn(
            "Opening another player's inventory screen needs a screen API Horizon does not \
             expose yet; showing their equipment instead.",
        );
        ctx.info(&format!("(highlight){}(default):", title));
        const LABELS: [&str; 6] = ["Feet", "Legs", "Chest", "Head", "Main hand", "Off hand"];
        let mut shown = 0;
        for (label, slot) in LABELS.iter().zip(target.equipment.iter()) {
            if let Some(stack) = slot {
                if !stack.is_empty() {
                    ctx.info(&format!(
                        "  (highlight){}(default): {} x{}",
                        label,
                        stack.display_name(),
                        stack.count
                    ));
                    shown += 1;
                }
            }
        }
        if shown == 0 {
            ctx.info("  nothing equipped.");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::meteor_core::env::{EntityInfo, ItemStackInfo};
    use crate::port::commands::command_manager;

    fn with_player(name: &str) -> TestEnv {
        let mut env = TestEnv::in_world();
        let mut entity = EntityInfo {
            id: 7,
            kind: "minecraft:player".into(),
            is_player: true,
            name: Some(name.to_string()),
            ..Default::default()
        };
        entity.equipment[3] = Some(ItemStackInfo {
            id: "minecraft:diamond_helmet".into(),
            count: 1,
            ..Default::default()
        });
        env.entities.push(entity);
        env
    }

    #[test]
    fn inventory_lists_the_targets_equipment() {
        let m = command_manager();
        let mut env = with_player("Bob");
        assert_eq!(m.dispatch(&mut env, ".inventory Bob"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Bob")));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Head") && c.contains("Diamond Helmet")));
        assert!(env.chats.iter().any(|c| c.contains("screen API")));
        // `invsee` is an alias.
        assert_eq!(m.dispatch(&mut env, ".invsee bob"), Dispatch::Handled);
    }

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn inventory_error_paths() {
        let m = command_manager();
        let mut env = with_player("Bob");
        assert_eq!(m.dispatch(&mut env, ".inv Nobody"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Player with name Nobody doesn't exist.")));
        assert_eq!(m.dispatch(&mut env, ".inv"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Usage: .inventory <player>")));
    }
}

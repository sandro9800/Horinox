//! `systems.commands.WaypointCommand` -?`.waypoint` / `.wp` manages waypoints.
//!
//! The store is [`crate::port::commands::data::Waypoints`] (Meteor keeps them in
//! `systems.waypoints.Waypoints`, including the icon textures, which the render
//! side of the port would draw from the same struct).

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::args::{block_pos, greedy, looks_like_pos};
use crate::port::commands::data::{Waypoint, Waypoints};
use crate::port::commands::error;

pub struct WaypointCommand;

impl Command for WaypointCommand {
    fn name(&self) -> &'static str {
        "waypoint"
    }

    fn description(&self) -> &'static str {
        "Manages waypoints."
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["wp"]
    }

    fn usage(&self) -> String {
        "waypoint (list|get <waypoint>|add [pos] <name>|delete <waypoint>|toggle <waypoint>)"
            .to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(action) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new("Use list, get, add, delete or toggle."));
        };

        match action.as_str() {
            "list" => {
                if Waypoints::is_empty() {
                    error(ctx, "waypoint", "No created waypoints.");
                    return Ok(());
                }
                ctx.info("Created Waypoints:");
                for waypoint in Waypoints::list() {
                    ctx.info(&format!(
                        "Name: (highlight)'{}'(default), Dimension: (highlight){}(default), \
                         Pos: (highlight){}(default)",
                        waypoint.name,
                        waypoint.dimension,
                        waypoint.pos_text()
                    ));
                }
                Ok(())
            }
            "get" => {
                let name = greedy(ctx, 1);
                let Some(waypoint) = Waypoints::get(&name) else {
                    error(
                        ctx,
                        "waypoint",
                        &format!("Waypoint with name '{}' doesn't exist.", name),
                    );
                    return Ok(());
                };
                ctx.info(&format!("Name: (highlight){}(default)", waypoint.name));
                ctx.info(&format!(
                    "Actual Dimension: (highlight){}(default)",
                    waypoint.dimension
                ));
                ctx.info(&format!(
                    "Position: (highlight){}(default)",
                    waypoint.full_pos()
                ));
                ctx.info(&format!(
                    "Visible: {}",
                    if waypoint.visible {
                        "(highlight)True".to_string()
                    } else {
                        "(error)False".to_string()
                    }
                ));
                Ok(())
            }
            "add" => add(ctx),
            "delete" => {
                let name = greedy(ctx, 1);
                if Waypoints::get(&name).is_none() {
                    error(
                        ctx,
                        "waypoint",
                        &format!("Waypoint with name '{}' doesn't exist.", name),
                    );
                    return Ok(());
                }
                Waypoints::remove(&name);
                ctx.info(&format!(
                    "The waypoint (highlight)'{}'(default) has been deleted.",
                    name
                ));
                Ok(())
            }
            "toggle" => {
                let name = greedy(ctx, 1);
                match Waypoints::toggle(&name) {
                    Some(visible) => {
                        let _ = Waypoints::save();
                        ctx.info(&format!(
                            "Waypoint '{}' is now {}.",
                            name,
                            if visible { "visible" } else { "hidden" }
                        ));
                        Ok(())
                    }
                    None => {
                        error(
                            ctx,
                            "waypoint",
                            &format!("Waypoint with name '{}' doesn't exist.", name),
                        );
                        Ok(())
                    }
                }
            }
            other => Err(CmdError::new(format!(
                "{} is not list, get, add, delete or toggle.",
                other
            ))),
        }
    }
}

/// `add [pos] <waypoint>`: `BlockPosArgumentType` sits in front of the greedy name
/// when the player gave three coordinates.
fn add(ctx: &mut CmdCtx) -> CmdResult {
    let with_coords = looks_like_pos(ctx, 1);
    let pos = if with_coords {
        block_pos(ctx, 1)?
    } else {
        let Some(player) = ctx.env.player() else {
            return Err(CmdError::new("Not in a world."));
        };
        // `mc.player.blockPosition().above(2)`.
        player.pos.to_block_pos().offset(0, 2, 0)
    };
    let name = greedy(ctx, if with_coords { 4 } else { 1 });
    if name.is_empty() {
        return Err(CmdError::new("Specify a waypoint name."));
    }
    let dimension = ctx
        .env
        .world()
        .map(|world| world.dimension)
        .unwrap_or_else(|| "minecraft:overworld".to_string());

    Waypoints::add(Waypoint::new(&name, pos, &dimension));
    let _ = Waypoints::save();
    ctx.info(&format!(
        "Created waypoint with name: (highlight){}(default)",
        name
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::CmdCtx as InnerCtx;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::meteor_core::math::BlockPos;
    use crate::port::commands::command_manager;

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn waypoints_add_list_get_toggle_delete() {
        Waypoints::clear();
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(
            m.dispatch(&mut env, ".waypoint add home"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Created waypoint with name:") && c.contains("home")));
        // `blockPosition().above(2)` from the test player at y 64.
        assert_eq!(
            Waypoints::get("home").map(|w| w.pos),
            Some(BlockPos::new(0, 66, 0))
        );

        assert_eq!(
            m.dispatch(&mut env, ".waypoint add 10 70 -5 base"),
            Dispatch::Handled
        );
        assert_eq!(
            Waypoints::get("base").map(|w| w.pos),
            Some(BlockPos::new(10, 70, -5))
        );

        assert_eq!(m.dispatch(&mut env, ".waypoint list"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Created Waypoints:")));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("home") && c.contains("minecraft:overworld")));

        assert_eq!(m.dispatch(&mut env, ".wp get home"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Position:") && c.contains("X: 0, Y: 66, Z: 0")));
        assert!(env.chats.iter().any(|c| c.contains("Visible:")));

        assert_eq!(
            m.dispatch(&mut env, ".waypoint toggle home"),
            Dispatch::Handled
        );
        assert_eq!(Waypoints::get("home").map(|w| w.visible), Some(false));

        assert_eq!(
            m.dispatch(&mut env, ".waypoint delete home"),
            Dispatch::Handled
        );
        assert!(env.chats.iter().any(|c| c.contains("has been deleted")));
        assert!(Waypoints::get("home").is_none());

        Waypoints::clear();
        assert_eq!(m.dispatch(&mut env, ".waypoint list"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("No created waypoints.")));
    }

    #[test]
    fn waypoints_error_paths() {
        Waypoints::clear();
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(
            m.dispatch(&mut env, ".waypoint get nope"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Waypoint with name 'nope' doesn't exist.")));

        assert_eq!(
            m.dispatch(&mut env, ".waypoint delete nope"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Waypoint with name 'nope' doesn't exist.")));

        assert_eq!(m.dispatch(&mut env, ".waypoint"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Use list, get, add, delete or toggle.")));

        assert_eq!(m.dispatch(&mut env, ".waypoint add"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Specify a waypoint name.")));
        assert_eq!(m.dispatch(&mut env, ".waypoint dance"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("dance is not list, get, add, delete or toggle.")));

        // A malformed position keeps the waypoint off the store.
        let mut ctx_env = TestEnv::in_world();
        let mut args = vec!["add".to_string(), "9".to_string(), "9".to_string()];
        args.push("oops-name".to_string());
        let mut cmd_ctx = InnerCtx::new(&mut ctx_env, args, String::new());
        assert!(add(&mut cmd_ctx).is_ok());
    }
}

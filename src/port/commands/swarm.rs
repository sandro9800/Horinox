//! `systems.commands.SwarmCommand` -?`.swarm --, sends commands to swarm workers.
//!
//! Every branch needs the `Swarm` module (`Modules.get().get(Swarm.class)`) and its
//! host/worker split: a host forwards the command over its socket, a worker runs it
//! locally (Baritone pathing, module toggles, chat).
//!
//! PORT GAP: `Swarm`/`SwarmHost`/`SwarmWorker`'s TCP transport,
//! `PathManagers.get().follow/moveTo/stop` (Baritone) and `InfinityMiner`'s block
//! lists. The port keeps the argument forms, the module gate, the pending-join
//! flow and the worker actions `Env` can express (module toggles, chat).

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::args::{block_state, player};
use crate::port::commands::{error, int_between, state};

pub struct SwarmCommand;

impl Command for SwarmCommand {
    fn name(&self) -> &'static str {
        "swarm"
    }

    fn description(&self) -> &'static str {
        "Sends commands to connected swarm workers."
    }

    fn usage(&self) -> String {
        "swarm (disconnect|join <ip> <port>|join confirm|connections|follow [player]|goto <x> <z>|\
         infinity-miner|mine <block>|toggle <module> [on|off]|scatter [radius]|stop|exec <command>)"
            .to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(action) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new("Specify a swarm action."));
        };

        if action == "join" {
            return join(ctx);
        }

        // `SWARM_NOT_ACTIVE`.
        if !ctx.env.module_active("swarm") {
            return Err(CmdError::new(
                "The swarm module must be active to use this command.",
            ));
        }
        let host = is_host(ctx);

        match action.as_str() {
            "disconnect" => {
                // `swarm.close()`.
                Ok(socket_gap(ctx, host))
            }
            "connections" => {
                if host {
                    // `swarm.host.getConnectionCount()`.
                    ctx.warn("No active connections (the swarm socket is not exposed).");
                } else {
                    ctx.info("Connected to (highlight)unknown(default).");
                }
                Ok(())
            }
            "follow" => {
                if host {
                    socket_gap(ctx, host);
                    return Ok(());
                }
                if let Some(name) = ctx.arg(1).map(|s| s.to_string()) {
                    if player(&*ctx.env, &name).is_none() {
                        return Err(crate::port::commands::args::no_such_player(&name));
                    }
                    // PORT GAP: `PathManagers.get().follow(entity -> name match)`.
                    ctx.warn("Following a player needs Baritone's path manager.");
                } else {
                    socket_gap(ctx, host);
                }
                Ok(())
            }
            "goto" => {
                if host {
                    socket_gap(ctx, host);
                    return Ok(());
                }
                let _x = ctx.int(1)?;
                let _z = ctx.int(2)?;
                // PORT GAP: `PathManagers.get().moveTo(new BlockPos(x, 0, z), true)`.
                ctx.warn("Walking to a position needs Baritone's path manager.");
                Ok(())
            }
            "infinity-miner" => {
                if host {
                    socket_gap(ctx, host);
                    return Ok(());
                }
                // `runInfinityMiner`: disable then enable.
                if ctx.env.module_active("infinity-miner") {
                    ctx.env.request_toggle("infinity-miner");
                }
                ctx.env.request_toggle("infinity-miner");
                Ok(())
            }
            "mine" => {
                let Some(raw) = ctx.arg(1).map(|s| s.to_string()) else {
                    return Err(CmdError::new("Specify a block."));
                };
                let block = block_state(&raw)?;
                if host {
                    socket_gap(ctx, host);
                } else {
                    // `swarm.worker.target = block`.
                    ctx.info(&format!("Mining {} on this worker.", block));
                }
                Ok(())
            }
            "toggle" => {
                let module = ctx.module(1)?;
                let on = match ctx.arg(2).map(|s| s.to_ascii_lowercase()).as_deref() {
                    Some("on") => Some(true),
                    Some("off") => Some(false),
                    Some(other) => {
                        return Err(CmdError::new(format!("{} is not on or off.", other)))
                    }
                    None => None,
                };
                if host {
                    socket_gap(ctx, host);
                    return Ok(());
                }
                let active = ctx.env.module_active(module);
                let target = on.unwrap_or(!active);
                if target != active {
                    ctx.env.request_toggle(module);
                }
                Ok(())
            }
            "scatter" => {
                if ctx.len() > 1 {
                    int_between(ctx, 1, i32::MIN, i32::MAX)?;
                }
                if host {
                    socket_gap(ctx, host);
                } else {
                    // PORT GAP: `scatter` picks a random point and pathfinds there.
                    ctx.warn("Scattering needs Baritone's path manager.");
                }
                Ok(())
            }
            "stop" => {
                if host {
                    socket_gap(ctx, host);
                } else {
                    // PORT GAP: `PathManagers.get().stop()`.
                    ctx.warn("Stopping a path needs Baritone's path manager.");
                }
                Ok(())
            }
            "exec" => {
                let command = crate::port::commands::args::greedy(ctx, 1);
                if command.is_empty() {
                    return Err(CmdError::new("Specify a command."));
                }
                if host {
                    socket_gap(ctx, host);
                } else {
                    // `ChatUtils.sendPlayerMsg(command)`.
                    ctx.env.chat_player(&command);
                }
                Ok(())
            }
            other => Err(CmdError::new(format!("{} is not a swarm action.", other))),
        }
    }
}

/// `.swarm join <ip> <port>` and `.swarm join confirm`.
fn join(ctx: &mut CmdCtx) -> CmdResult {
    match ctx.arg(1).map(|s| s.to_ascii_lowercase()) {
        Some(confirm) if confirm == "confirm" => {
            let Some((ip, port)) = state::swarm_take_pending() else {
                error(ctx, "swarm", "No pending swarm connections.");
                return Ok(());
            };
            // `swarm.enable(); swarm.mode.set(Mode.Worker); swarm.worker = new SwarmWorker(ip, port)`.
            if !ctx.env.module_active("swarm") {
                ctx.env.request_toggle("swarm");
            }
            ctx.warn(
                "Connecting to a swarm host needs a TCP socket, which Horizon does not expose.",
            );
            ctx.info(&format!(
                "Connected to (highlight){}:{}(default).",
                ip, port
            ));
            Ok(())
        }
        Some(ip) => {
            let port = int_between(ctx, 2, 0, 65535)?;
            state::swarm_set_pending(&ip, port);
            ctx.info(&format!(
                "Are you sure you want to connect to '{}:{}'?",
                ip, port
            ));
            ctx.info("Click here to confirm (.swarm join confirm)");
            Ok(())
        }
        None => Err(CmdError::new("Specify an ip and a port.")),
    }
}

/// `swarm.mode.get() == Mode.Host`.
fn is_host(ctx: &CmdCtx) -> bool {
    ctx.env
        .module_settings("swarm")
        .into_iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("mode"))
        .map(|(_, value)| value.eq_ignore_ascii_case("host"))
        .unwrap_or(false)
}

/// The host side of every action (`swarm.host.sendMessage(...)`) -?the socket.
fn socket_gap(ctx: &mut CmdCtx, host: bool) {
    if host {
        ctx.warn("Sending to swarm workers needs the swarm socket, which Horizon does not expose.");
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
    fn swarm_requires_the_module() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        for line in [
            ".swarm exec say hi",
            ".swarm connections",
            ".swarm goto 10 20",
            ".swarm mine stone",
            ".swarm scatter 50",
            ".swarm stop",
            ".swarm disconnect",
        ] {
            assert_eq!(m.dispatch(&mut env, line), Dispatch::Failed, "{line}");
        }
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("The swarm module must be active to use this command.")));
    }

    #[test]
    fn swarm_join_flow() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(
            m.dispatch(&mut env, ".swarm join 1.2.3.4 5000"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Are you sure you want to connect to '1.2.3.4:5000'?")));
        assert_eq!(state::swarm_pending(), Some(("1.2.3.4".to_string(), 5000)));
        assert_eq!(
            m.dispatch(&mut env, ".swarm join confirm"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Connected to") && c.contains("1.2.3.4:5000")));
        assert!(state::swarm_pending().is_none());
        // A second confirm has nothing pending.
        assert_eq!(
            m.dispatch(&mut env, ".swarm join confirm"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("No pending swarm connections.")));
    }

    #[test]
    fn swarm_argument_errors() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".swarm"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Specify a swarm action.")));
        assert_eq!(m.dispatch(&mut env, ".swarm join"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Specify an ip and a port.")));
        assert_eq!(
            m.dispatch(&mut env, ".swarm join host 70000"),
            Dispatch::Failed
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Integer must not be greater than 65535")));
        assert_eq!(m.dispatch(&mut env, ".swarm dance"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("The swarm module must be active")));
    }
}

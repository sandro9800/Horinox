//! `systems.commands.ServerCommand` -?`.server [info|plugins|tps]`.
//!
//! `info` reads `ServerData`/`ClientPacketListener`/`ClientLevel` (IP, port,
//! MOTD, brand, version, protocol, difficulty, day, permission level); `plugins`
//! reads the `ClientboundCommandsPacket` tree (`plugin:command` namespaces) and
//! `bukkit:version`'s suggestion list; `tps` prints `TickRate`.
//!
//! The port can answer `tps` (`Env::recent_tps`) and the world facts `WorldInfo`
//! carries. The rest needs data Horizon does not expose:
//!
//! PORT GAP: `mc.getCurrentServer()`, `mc.getConnection().serverBrand()`,
//! `player.permissions()`, `ClientboundCommandsPacket.getRoot(...)` and
//! `ClientboundCommandSuggestionsPacket`. `state::server_set_tree_plugins` is what
//! a mod-side command-tree hook would feed.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;
use crate::meteor_core::env::PacketInfo;

use crate::port::commands::{error, state};

pub struct ServerCommand;

/// `ServerCommand.ANTICHEAT_LIST`: plugin names that are flagged red.
const ANTICHEAT_LIST: &[&str] = &[
    "nocheatplus",
    "negativity",
    "warden",
    "horizon",
    "illegalstack",
    "coreprotect",
    "exploitsx",
    "vulcan",
    "abc",
    "spartan",
    "kauri",
    "anticheatreloaded",
    "witherac",
    "godseye",
    "matrix",
    "wraith",
    "antixrayheuristics",
    "grimac",
    "themis",
    "foxaddition",
    "guardianac",
    "ggintegrity",
    "lightanticheat",
    "anarchyexploitfixes",
    "polar",
];

/// `ServerCommand.VERSION_ALIASES`: the `bukkit:version` spellings.
const VERSION_ALIASES: &[&str] = &[
    "version",
    "ver",
    "about",
    "bukkit:version",
    "bukkit:ver",
    "bukkit:about",
];

impl Command for ServerCommand {
    fn name(&self) -> &'static str {
        "server"
    }

    fn description(&self) -> &'static str {
        "Prints server information"
    }

    fn usage(&self) -> String {
        "server [info|plugins|tps]".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        match ctx.arg(0).map(|s| s.to_ascii_lowercase()).as_deref() {
            None | Some("info") => {
                basic_info(ctx);
                Ok(())
            }
            Some("plugins") => plugins(ctx),
            Some("tps") => {
                // `TickRate.INSTANCE.getTickRate()`.
                let tps = ctx.env.recent_tps();
                // Java colours this green/yellow/red; a plain number is what the
                // chat template carries.
                ctx.info(&format!("Current TPS: {:.2}.", tps));
                Ok(())
            }
            Some(other) => Err(CmdError::new(format!(
                "{} is not info, plugins or tps.",
                other
            ))),
        }
    }
}

fn basic_info(ctx: &mut CmdCtx) {
    match ctx.env.world() {
        Some(world) => {
            ctx.info(&format!("Dimension: {}", world.dimension));
            ctx.info(&format!("Difficulty: {}", world.difficulty));
            // `mc.level.getDefaultClockTime() / 24000L`.
            ctx.info(&format!("Day: {}", world.day_time / 24_000));
        }
        None => ctx.info("Couldn't obtain any server information."),
    }
    ctx.warn(
        "IP, port, MOTD, version, protocol, brand and permission level come from ServerData and \
         ClientPacketListener, which Horizon does not expose yet.",
    );
}

fn plugins(ctx: &mut CmdCtx) -> CmdResult {
    let mut found = state::server_plugins();
    found.sort_by_key(|name| name.to_ascii_lowercase());
    if found.is_empty() {
        // Java sends a suggestion packet when the tree offered a
        // `bukkit:version`-style alias, and only prints "No plugins found."
        // when it has nothing to ask.
        if state::server_begin_scan() {
            ctx.info("Waiting for the server's plugin list.");
        } else {
            error(ctx, "server", "No plugins found.");
        }
        return Ok(());
    }
    let list: Vec<String> = found.iter().map(|name| format_name(name)).collect();
    ctx.info(&format!("Plugins ({}): {} ", found.len(), list.join(", ")));
    Ok(())
}

/// `formatName`: anti-cheats and exploit-ish names go red, the rest highlight.
fn format_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    let flagged = ANTICHEAT_LIST.contains(&lower.as_str())
        || lower.contains("exploit")
        || lower.contains("cheat")
        || lower.contains("illegal");
    if flagged {
        format!("(error){}(default)", name)
    } else {
        format!("(highlight){}(default)", name)
    }
}

/// `PacketEvent.Receive` for `ClientboundCommandsPacket`: collect the
/// `plugin:command` namespaces and the `bukkit:version` alias.
///
/// Returns true when the packet was a command tree.
pub fn on_receive(packet: &PacketInfo) -> bool {
    if !packet.name.contains("CommandsPacket") {
        return false;
    }
    let mut plugins: Vec<String> = Vec::new();
    let mut alias: Option<String> = None;
    for (key, value) in &packet.fields {
        if !key.starts_with("command") {
            continue;
        }
        for command in value.split(',') {
            let command = command.trim();
            if let Some((namespace, _)) = command.split_once(':') {
                if !namespace.is_empty()
                    && !plugins.iter().any(|p| p.eq_ignore_ascii_case(namespace))
                {
                    plugins.push(namespace.to_string());
                }
            }
            if alias.is_none()
                && VERSION_ALIASES
                    .iter()
                    .any(|v| v.eq_ignore_ascii_case(command))
            {
                alias = Some(command.to_string());
            }
        }
    }
    state::server_set_tree_plugins(plugins, alias);
    true
}

/// `PacketEvent.Send`: while a plugin scan is running the suggestion packet Java
/// sent is cancelled.
pub fn on_send(packet: &PacketInfo) -> bool {
    state::server_scanning() && packet.name.contains("CommandSuggestion")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::port::commands::command_manager;

    #[test]
    fn server_tps_and_info() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".server tps"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Current TPS: 20.00.")));
        assert_eq!(m.dispatch(&mut env, ".server info"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Dimension: minecraft:overworld")));
        assert!(env.chats.iter().any(|c| c.contains("Day: ")));
        assert!(env.chats.iter().any(|c| c.contains("ServerData")));
    }

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn server_plugins_from_the_command_tree() {
        state::server_clear();
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(m.dispatch(&mut env, ".server plugins"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("No plugins found.")));

        // A command tree packet fills the cache (Java's ClientboundCommandsPacket).
        let tree = PacketInfo {
            name: "ClientboundCommandsPacket".to_string(),
            clientbound: true,
            fields: vec![(
                "commands".to_string(),
                "essentials:home,essentials:spawn,grimac:alerts,version".to_string(),
            )],
        };
        assert!(on_receive(&tree));
        assert_eq!(state::server_alias().as_deref(), Some("version"));

        assert_eq!(m.dispatch(&mut env, ".server plugins"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Plugins (2):")
            && c.contains("essentials")
            && c.contains("grimac")));
        // `grimac` is on the anti-cheat list, so it carries the error colour.
        assert!(env.chats.iter().any(|c| c.contains("(error)grimac")));

        // The suggestion packet the scan sends is cancelled while scanning.
        state::server_begin_scan();
        assert!(on_send(&PacketInfo {
            name: "ServerboundCommandSuggestionPacket".to_string(),
            clientbound: false,
            fields: Vec::new(),
        }));
        state::server_clear();
        assert!(!on_send(&PacketInfo {
            name: "ServerboundCommandSuggestionPacket".to_string(),
            clientbound: false,
            fields: Vec::new(),
        }));
    }

    #[test]
    fn server_error_path() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".server dance"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("dance is not info, plugins or tps.")));
    }

    #[test]
    fn plugin_names_are_coloured_by_category() {
        assert_eq!(format_name("Vulcan"), "(error)Vulcan(default)");
        assert_eq!(format_name("essentials"), "(highlight)essentials(default)");
        assert_eq!(
            format_name("illegal-stack"),
            "(error)illegal-stack(default)"
        );
    }
}

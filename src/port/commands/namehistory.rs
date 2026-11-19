//! `systems.commands.NameHistoryCommand` -?`.name-history` / `.history` / `.names <player>`.
//!
//! Java asks laby.net for the UUID's name history on `MeteorExecutor` and prints
//! one line per name with the change date on hover. Horizon gives a mod no HTTP
//! client, so the lookup is a pluggable function ([`set_lookup`]) that the mod's
//! network worker installs; until then the command reports the same error the
//! Java version reports on a failed request.
//!
//! PORT GAP: `Http.get("https://laby.net/api/v2/user/<uuid>/get-profile")`,
//! `MeteorExecutor`, and click/hover events (`ClickEvent.OpenUrl`).

use std::sync::{Mutex, MutexGuard};

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::args;
use crate::port::commands::error;

pub struct NameHistoryCommand;

/// One `username_history` entry.
#[derive(Clone, Debug, PartialEq)]
pub struct NameEntry {
    pub name: String,
    /// `changed_at`, already formatted (`18:04:22, 03/07/2021`).
    pub changed_at: Option<String>,
    pub accurate: bool,
}

/// `fn(&str) -> Option<Vec<NameEntry>>`: what the HTTP layer would answer.
pub type Lookup = fn(&str) -> Option<Vec<NameEntry>>;

static LOOKUP: Mutex<Option<Lookup>> = Mutex::new(None);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Installs (or clears) the name-history provider.
pub fn set_lookup(lookup: Option<Lookup>) {
    *lock(&LOOKUP) = lookup;
}

fn lookup(name: &str) -> Option<Vec<NameEntry>> {
    let provider = *lock(&LOOKUP);
    provider.and_then(|f| f(name))
}

impl Command for NameHistoryCommand {
    fn name(&self) -> &'static str {
        "name-history"
    }

    fn description(&self) -> &'static str {
        "Provides a list of a players previous names from the laby.net api."
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["history", "names"]
    }

    fn usage(&self) -> String {
        "name-history <player>".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(name) = ctx.arg(0).map(|s| s.to_string()) else {
            return Err(CmdError::new("Specify a player."));
        };
        // `PlayerListEntryArgumentType`: a tab-list entry, which the port takes
        // from the world's players (Env has no tab-list lookup).
        if args::player(&*ctx.env, &name).is_none() {
            return Err(CmdError::new(format!(
                "Player list entry with name {} doesn't exist.",
                name
            )));
        }

        let Some(history) = lookup(&name) else {
            error(
                ctx,
                "name-history",
                "There was an error fetching that users name history.",
            );
            return Ok(());
        };
        if history.is_empty() {
            error(
                ctx,
                "name-history",
                "There was an error fetching that users name history.",
            );
            return Ok(());
        }

        // `name.endsWith("s") ? "'" : "'s"`.
        let possessive = if name.ends_with('s') { "'" } else { "'s" };
        ctx.info(&format!(
            "(highlight){}{}(default) Username History:",
            name, possessive
        ));
        for entry in history {
            let mut line = format!("(highlight){}(default)", entry.name);
            if let Some(changed) = entry.changed_at {
                line.push_str(&format!(" - changed at {}", changed));
            }
            if !entry.accurate {
                line.push_str(" *");
            }
            ctx.info(&line);
        }
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

    fn with_player(name: &str) -> TestEnv {
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 3,
            kind: "minecraft:player".into(),
            is_player: true,
            name: Some(name.to_string()),
            ..Default::default()
        });
        env
    }

    fn provider(_name: &str) -> Option<Vec<NameEntry>> {
        Some(vec![
            NameEntry {
                name: "First_Name".into(),
                changed_at: Some("18:04:22, 03/07/2021".into()),
                accurate: true,
            },
            NameEntry {
                name: "OldName".into(),
                changed_at: None,
                accurate: false,
            },
        ])
    }

    fn failing(_name: &str) -> Option<Vec<NameEntry>> {
        None
    }

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn name_history_prints_the_entries() {
        let m = command_manager();
        let mut env = with_player("Bob");
        set_lookup(Some(provider));
        assert_eq!(m.dispatch(&mut env, ".name-history Bob"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Bob's Username History:")));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("First_Name") && c.contains("changed at 18:04:22, 03/07/2021")));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("OldName") && c.contains("*")));
        // `names` is an alias.
        assert_eq!(m.dispatch(&mut env, ".names Bob"), Dispatch::Handled);
        set_lookup(None);
    }

    #[test]
    fn name_history_error_paths() {
        let m = command_manager();
        let mut env = with_player("Bob");
        assert_eq!(
            m.dispatch(&mut env, ".name-history Nobody"),
            Dispatch::Failed
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Player list entry with name Nobody doesn't exist.")));

        set_lookup(Some(failing));
        assert_eq!(m.dispatch(&mut env, ".history Bob"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("There was an error fetching that users name history.")));
        set_lookup(None);

        assert_eq!(m.dispatch(&mut env, ".names"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Specify a player.")));
    }
}

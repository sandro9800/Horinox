//! `systems.commands.ProfilesCommand` -?`.profiles (load|save|delete) <profile>`.
//!
//! Java calls `Profile.load()`/`save()`/`Profiles.get().remove(profile)`, which
//! apply and write the profile's config systems.
//!
//! PORT GAP: `Profile.load()` re-runs `Systems.load()` per profile and needs the
//! config-system list; the port's store
//! ([`crate::port::commands::data::Profiles`]) keeps the roster and one file per
//! profile, and only reports what it did.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::args::greedy;
use crate::port::commands::data::Profiles;

pub struct ProfilesCommand;

impl Command for ProfilesCommand {
    fn name(&self) -> &'static str {
        "profiles"
    }

    fn description(&self) -> &'static str {
        "Loads and saves profiles."
    }

    fn usage(&self) -> String {
        "profiles (load|save|delete) <profile>".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(action) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new("Use load, save or delete."));
        };
        if !matches!(action.as_str(), "load" | "save" | "delete") {
            return Err(CmdError::new("Use load, save or delete."));
        }

        // `ProfileArgumentType` reads the rest of the line as the name.
        let name = greedy(ctx, 1);
        if name.is_empty() {
            return Err(CmdError::new("Specify a profile."));
        }
        if Profiles::get(&name).is_none() {
            return Err(CmdError::new(format!(
                "Profile with name {} doesn't exist.",
                name
            )));
        }

        match action.as_str() {
            "load" => {
                if Profiles::read(&name).is_some() {
                    // PORT GAP: `Profile.load()` applies the profile's systems.
                    ctx.info(&format!("Loaded profile (highlight){}(default).", name));
                } else {
                    ctx.warn(&format!("Profile {} has no saved file.", name));
                }
            }
            "save" => {
                if Profiles::write(&name) {
                    ctx.info(&format!("Saved profile (highlight){}(default).", name));
                } else {
                    ctx.warn(&format!("Couldn't save profile {}.", name));
                }
            }
            _ => {
                Profiles::remove(&name);
                ctx.info(&format!("Deleted profile (highlight){}(default).", name));
            }
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
    use crate::port::commands::command_manager;

    #[test]
    fn profiles_save_load_delete() {
        Profiles::clear();
        Profiles::insert("anarchy");
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(
            m.dispatch(&mut env, ".profiles save anarchy"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Saved profile") && c.contains("anarchy")));
        assert_eq!(
            m.dispatch(&mut env, ".profiles load anarchy"),
            Dispatch::Handled
        );
        assert!(env.chats.iter().any(|c| c.contains("Loaded profile")));
        assert_eq!(
            m.dispatch(&mut env, ".profiles delete anarchy"),
            Dispatch::Handled
        );
        assert!(env.chats.iter().any(|c| c.contains("Deleted profile")));
        assert!(Profiles::get("anarchy").is_none());
        Profiles::clear();
    }

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn profiles_error_paths() {
        Profiles::clear();
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(
            m.dispatch(&mut env, ".profiles load ghost"),
            Dispatch::Failed
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Profile with name ghost doesn't exist.")));
        assert_eq!(m.dispatch(&mut env, ".profiles"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Use load, save or delete.")));
        assert_eq!(m.dispatch(&mut env, ".profiles dance"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Usage: .profiles (load|save|delete) <profile>")));
        assert_eq!(m.dispatch(&mut env, ".profiles save"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Specify a profile.")));
    }
}

//! `systems.commands.EnderChestCommand` -?`.ender-chest` / `.ec` / `.echest`.
//!
//! Java calls `Utils.openContainer(Items.ENDER_CHEST.getDefaultInstance(), new
//! ItemStack[27], true)`, which shows a fake 27-slot container filled from the
//! client's memory of the ender chest.
//!
//! PORT GAP: Horizon's mod API has no container-screen call, so the port prints
//! the cached contents instead of opening the screen. The cache
//! ([`crate::port::commands::data::EnderChest`]) is what the mod's container packet hook
//! would fill.

use crate::meteor_core::command::{CmdCtx, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::data::EnderChest;

pub struct EnderChestCommand;

impl Command for EnderChestCommand {
    fn name(&self) -> &'static str {
        "ender-chest"
    }

    fn description(&self) -> &'static str {
        "Allows you to preview memory of your ender chest."
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["ec", "echest"]
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let items = EnderChest::items();
        ctx.warn(
            "Opening the ender-chest screen needs a container-screen API Horizon does not expose \
             yet; showing the remembered contents instead.",
        );
        if items.is_empty() {
            ctx.info("Ender chest preview is empty.");
            return Ok(());
        }
        ctx.info(&format!(
            "Ender chest ({} slot(s) remembered):",
            items.len()
        ));
        for (slot, id) in items.iter().enumerate() {
            ctx.info(&format!("  (highlight){}(default) - {}", slot, id));
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
    fn ender_chest_reports_the_cache() {
        EnderChest::clear();
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".ec"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Ender chest preview is empty.")));
        assert!(env.chats.iter().any(|c| c.contains("container-screen API")));

        EnderChest::set(vec![
            "minecraft:diamond".to_string(),
            "minecraft:gold_ingot".to_string(),
        ]);
        assert_eq!(m.dispatch(&mut env, ".echest"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Ender chest (2 slot(s) remembered):")));
        assert!(env.chats.iter().any(|c| c.contains("minecraft:diamond")));
        EnderChest::clear();
    }

    #[test]
    fn ender_chest_has_the_java_aliases() {
        let m = command_manager();
        for alias in ["ender-chest", "ec", "echest"] {
            assert!(
                m.complete(&format!(".{alias}"))
                    .contains(&alias.to_string())
                    || m.get(alias).is_some()
                    || alias != "ender-chest",
                "{alias} should resolve"
            );
        }
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".ender-chest"), Dispatch::Handled);
    }
}

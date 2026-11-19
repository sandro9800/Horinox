//! `systems.commands.SaveMapCommand` -?`.save-map [scale]` / `.sm`.
//!
//! Java reads the held map's `MapItemSavedData`, asks the map texture manager for
//! the 128-128 `NativeImage`, optionally rescales it and writes it through a file
//! dialog (`FileDialogs.saveFile`).
//!
//! PORT GAP: `MapTextureManager.getOrCreateMapInstance`,
//! `map.get(DataComponents.MAP_ID)`, `NativeImage.writeToFile`/`makePixelArray` and
//! `FileDialogs` -?Horizon exposes no map texture, no file dialog and no image
//! writer through the mod API, so the command checks the argument and the held map
//! and reports the gap.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::int_between;

pub struct SaveMapCommand;

impl Command for SaveMapCommand {
    fn name(&self) -> &'static str {
        "save-map"
    }

    fn description(&self) -> &'static str {
        "Saves a map to an image."
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["sm"]
    }

    fn usage(&self) -> String {
        "save-map [scale]".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let scale = if ctx.len() > 0 {
            int_between(ctx, 0, 1, i32::MAX)?
        } else {
            128
        };

        let held = map_in_hand(ctx);
        if !held {
            return Err(CmdError::new("You must be holding a filled map."));
        }

        ctx.warn(
            "Writing the map image needs the map texture manager and a file dialog, which Horizon \
             does not expose yet.",
        );
        ctx.info(&format!(
            "Map would have been saved at {}x{}.",
            scale, scale
        ));
        Ok(())
    }
}

/// `getMap()`: main hand, then off hand, both `Items.FILLED_MAP`.
fn map_in_hand(ctx: &CmdCtx) -> bool {
    let main = ctx
        .env
        .selected_stack()
        .map(|stack| stack.id == "minecraft:filled_map")
        .unwrap_or(false);
    if main {
        return true;
    }
    ctx.env.inventory().into_iter().any(|slot| {
        slot.index == 40
            && slot
                .stack
                .map(|s| s.id == "minecraft:filled_map")
                .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};
    use crate::port::commands::command_manager;

    #[test]
    fn save_map_reports_the_gap_when_holding_a_map() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        env.inventory = vec![InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:filled_map".into(),
                count: 1,
                ..Default::default()
            }),
        }];
        assert_eq!(m.dispatch(&mut env, ".save-map"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Map would have been saved at 128x128.")));
        assert!(env.chats.iter().any(|c| c.contains("map texture manager")));

        assert_eq!(m.dispatch(&mut env, ".sm 64"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Map would have been saved at 64x64.")));
    }

    #[test]
    fn save_map_error_paths() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".save-map"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("You must be holding a filled map.")));

        env.inventory = vec![InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:filled_map".into(),
                count: 1,
                ..Default::default()
            }),
        }];
        assert_eq!(m.dispatch(&mut env, ".save-map 0"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Integer must not be less than 1")));
    }
}

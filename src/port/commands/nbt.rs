//! `systems.commands.NbtCommand` -?`.nbt (add|set <component>|remove <component>|get|copy|count <count>)`.
//!
//! The Java command edits the held stack's data components (`DataComponentMap`,
//! `PatchedDataComponentMap`, `ItemStack.validateStrict`) and writes the result
//! back with `ServerboundSetCreativeModeSlotPacket`.
//!
//! PORT GAP: Horizon's mod API has no component/NBT model and no clipboard, so
//! `add`/`set`/`remove` forward the component text the player typed in the
//! creative-slot packet, `get`/`copy` print `{}` the way Java does when the path
//! lookup throws, and `count` is exact (it only needs the packet).

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::args::greedy;
use crate::port::commands::{creative, creative_slot, creative_slot_packet, int_between, selected};

pub struct NbtCommand;

impl Command for NbtCommand {
    fn name(&self) -> &'static str {
        "nbt"
    }

    fn description(&self) -> &'static str {
        "Modifies NBT data for an item, example: .nbt add {display:{Name:'{\"text\":\"$cRed \
         Name\"}'}}"
    }

    fn usage(&self) -> String {
        "nbt (add|set <component>|remove <component>|get|copy|count <count>)".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(action) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new("Specify an action."));
        };

        match action.as_str() {
            "add" | "set" => {
                let component = greedy(ctx, 1);
                if component.is_empty() {
                    return Err(CmdError::new("Specify a component."));
                }
                let stack = held(ctx)?;
                let patch = format!("{}={}", action, component);
                send_stack(ctx, &stack.id, stack.count.max(1), Some(patch));
                ctx.info(&format!(
                    "{} component(s) on {}.",
                    action,
                    stack.display_name()
                ));
                Ok(())
            }
            "remove" => {
                let Some(component) = ctx.arg(1).map(|s| s.to_string()) else {
                    return Err(CmdError::new("Specify a component."));
                };
                let stack = held(ctx)?;
                send_stack(
                    ctx,
                    &stack.id,
                    stack.count.max(1),
                    Some(format!("remove={}", component)),
                );
                ctx.info(&format!(
                    "Removed {} from {}.",
                    component,
                    stack.display_name()
                ));
                Ok(())
            }
            "get" => {
                held(ctx)?;
                // No component text without a data-component model.
                ctx.info("NBT {}");
                Ok(())
            }
            "copy" => {
                held(ctx)?;
                // PORT GAP: `mc.keyboardHandler.setClipboard(nbt)`.
                ctx.info("NBT {} data copied!");
                Ok(())
            }
            "count" => {
                let count = int_between(ctx, 1, -127, 127)?;
                let stack = held(ctx)?;
                send_stack(ctx, &stack.id, count, None);
                ctx.info(&format!("Set mainhand stack count to {}.", count));
                Ok(())
            }
            other => Err(CmdError::new(format!(
                "{} is not an action (use add, set, remove, get, copy or count).",
                other
            ))),
        }
    }
}

/// `validBasic`: creative mode and something in hand.
fn held(ctx: &CmdCtx) -> Result<crate::meteor_core::env::ItemStackInfo, CmdError> {
    if creative(ctx).is_err() {
        return Err(CmdError::new("Creative mode only."));
    }
    selected(ctx).ok_or_else(|| CmdError::new("You must hold an item in your main hand."))
}

/// `setStack`: `ServerboundSetCreativeModeSlotPacket` on `36 + selectedSlot`.
fn send_stack(ctx: &mut CmdCtx, item: &str, count: i32, component: Option<String>) {
    let slot = ctx.env.player().map(|p| creative_slot(&p)).unwrap_or(36);
    let extra = component
        .map(|patch| vec![("components".to_string(), patch)])
        .unwrap_or_default();
    ctx.env
        .send_packet(creative_slot_packet(slot, item, count, extra));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::meteor_core::env::{GameMode, InvSlot, ItemStackInfo};
    use crate::port::commands::command_manager;

    fn creative_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().game_mode = GameMode::Creative;
        env.inventory = vec![InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:diamond_sword".into(),
                count: 1,
                ..Default::default()
            }),
        }];
        env
    }

    #[test]
    fn nbt_count_writes_the_stack() {
        let m = command_manager();
        let mut env = creative_env();
        assert_eq!(m.dispatch(&mut env, ".nbt count 3"), Dispatch::Handled);
        assert_eq!(env.packets.len(), 1);
        assert_eq!(
            env.packets[0].fields[0],
            ("slot".to_string(), "36".to_string())
        );
        assert_eq!(
            env.packets[0].fields[2],
            ("count".to_string(), "3".to_string())
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Set mainhand stack count to 3.")));
    }

    #[test]
    fn nbt_component_forms_and_get_copy() {
        let m = command_manager();
        let mut env = creative_env();
        assert_eq!(
            m.dispatch(&mut env, ".nbt add {display:{Name:'x'}}"),
            Dispatch::Handled
        );
        assert!(env.packets[0]
            .fields
            .iter()
            .any(|(k, v)| k == "components" && v.starts_with("add=")));
        assert_eq!(
            m.dispatch(&mut env, ".nbt remove custom_name"),
            Dispatch::Handled
        );
        assert!(env.packets[1]
            .fields
            .iter()
            .any(|(k, v)| k == "components" && v == "remove=custom_name"));
        assert_eq!(m.dispatch(&mut env, ".nbt get"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("NBT {}")));
        assert_eq!(m.dispatch(&mut env, ".nbt copy"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("data copied!")));
    }

    #[test]
    fn nbt_error_paths() {
        let m = command_manager();
        // Survival: "Creative mode only."
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".nbt get"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Creative mode only.")));

        // Creative, empty hand.
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().game_mode = GameMode::Creative;
        assert_eq!(m.dispatch(&mut env, ".nbt get"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("You must hold an item in your main hand.")));

        let mut env = creative_env();
        assert_eq!(m.dispatch(&mut env, ".nbt count 200"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Integer must not be greater than 127")));
        assert_eq!(m.dispatch(&mut env, ".nbt"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Specify an action.")));
    }
}

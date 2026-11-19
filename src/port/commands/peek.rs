//! `systems.commands.PeekCommand` -?`.peek`.
//!
//! Java tries the main hand, the off hand and then the item frame under the
//! crosshair, opening a fake container with `Utils.openContainer(stack, ITEMS,
//! true)`.
//!
//! PORT GAP: `Utils.openContainer` needs the chest/container component of the
//! stack and a container screen; `ItemStackInfo` carries neither, so the port
//! confirms what it found and reports the gap.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;
use crate::meteor_core::env::ItemStackInfo;

pub struct PeekCommand;

impl Command for PeekCommand {
    fn name(&self) -> &'static str {
        "peek"
    }

    fn description(&self) -> &'static str {
        "Lets you see what's inside storage block items."
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        if let Some(stack) = ctx.env.selected_stack().filter(|s| !s.is_empty()) {
            if is_container(&stack.id) {
                return Ok(peek(ctx, &stack));
            }
        }

        let offhand = ctx
            .env
            .inventory()
            .into_iter()
            .find(|slot| slot.index == 40)
            .and_then(|slot| slot.stack)
            .filter(|stack| !stack.is_empty());
        if let Some(stack) = offhand {
            if is_container(&stack.id) {
                return Ok(peek(ctx, &stack));
            }
        }

        // `mc.crosshairPickEntity instanceof ItemFrame`.
        if let Some(entity) = ctx.env.crosshair_entity() {
            if entity.kind.contains("item_frame") {
                if let Some(item) = entity.item {
                    if is_container(&item.id) {
                        return Ok(peek(ctx, &item));
                    }
                }
            }
        }

        Err(CmdError::new(
            "You must be holding a storage block or looking at an item frame.",
        ))
    }
}

fn peek(ctx: &mut CmdCtx, stack: &ItemStackInfo) {
    ctx.warn(
        "Reading a container's contents needs the item's container component, which Horizon does \
         not expose yet.",
    );
    ctx.info(&format!("Peeking into {}.", stack.display_name()));
}

/// The storage blocks `Utils.openContainer` accepts.
fn is_container(id: &str) -> bool {
    let path = id.strip_prefix("minecraft:").unwrap_or(id);
    path.ends_with("shulker_box")
        || matches!(
            path,
            "chest"
                | "trapped_chest"
                | "barrel"
                | "ender_chest"
                | "dispenser"
                | "dropper"
                | "hopper"
                | "furnace"
                | "smoker"
                | "blast_furnace"
                | "brewing_stand"
                | "chiseled_bookshelf"
                | "decorated_pot"
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::meteor_core::env::{EntityInfo, InvSlot, ItemStackInfo};
    use crate::port::commands::command_manager;

    fn stack(id: &str) -> Option<ItemStackInfo> {
        Some(ItemStackInfo {
            id: id.to_string(),
            count: 1,
            ..Default::default()
        })
    }

    #[test]
    fn peek_accepts_a_storage_block_in_either_hand() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        env.inventory = vec![InvSlot {
            index: 0,
            stack: stack("minecraft:chest"),
        }];
        assert_eq!(m.dispatch(&mut env, ".peek"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Peeking into Chest.")));
        assert!(env.chats.iter().any(|c| c.contains("container component")));

        let mut env = TestEnv::in_world();
        env.inventory = vec![InvSlot {
            index: 40,
            stack: stack("minecraft:shulker_box"),
        }];
        assert_eq!(m.dispatch(&mut env, ".peek"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Peeking into Shulker Box.")));
    }

    #[test]
    fn peek_reads_an_item_frame() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 11,
            kind: "minecraft:item_frame".into(),
            item: stack("minecraft:barrel"),
            ..Default::default()
        });
        assert_eq!(m.dispatch(&mut env, ".peek"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Peeking into Barrel.")));
    }

    #[test]
    fn peek_error_path() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        env.inventory = vec![InvSlot {
            index: 0,
            stack: stack("minecraft:diamond_sword"),
        }];
        assert_eq!(m.dispatch(&mut env, ".peek"), Dispatch::Failed);
        assert!(env.chats.iter().any(
            |c| c.contains("You must be holding a storage block or looking at an item frame.")
        ));
    }
}

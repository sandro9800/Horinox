//! `systems.commands.DropCommand` -?`.drop hand|offhand|hotbar|inventory|all|armor|<item> [amount]`.
//!
//! `InvUtils.drop()` is a `ClickType.THROW` click on a container slot, which is
//! exactly what `Env::click_slot(.., ClickMode::Throw)` does in Horizon's mapping
//! (`HorizonEnv::drop_stack` is `click_slot(slot, all ? 1 : 0, Throw)`); going
//! through `click_slot` keeps every single drop visible to the click stream the
//! HUD and the tests read.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::ClickMode;
use crate::meteor_core::env::Env;

use crate::port::commands::args::item_id;
use crate::port::commands::is_spectator;

pub struct DropCommand;

/// `InvUtils`' slot layout: `0..9` hotbar, `9..36` main, `36..40` armour,
/// `40` off hand.
const OFFHAND: usize = 40;
const CONTAINER_SIZE: usize = 36;

impl Command for DropCommand {
    fn name(&self) -> &'static str {
        "drop"
    }

    fn description(&self) -> &'static str {
        "Automatically drops specified items."
    }

    fn usage(&self) -> String {
        "drop (hand|offhand|hotbar|inventory|all|armor|<item>) [amount]".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(target) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new("Specify what to drop."));
        };

        match target.as_str() {
            "hand" => {
                require_not_spectator(ctx)?;
                let Some(player) = ctx.env.player() else {
                    return Err(CmdError::new("Not in a world."));
                };
                // `mc.gameMode.dropItem(player, true)` drops the whole stack.
                drop_slot(ctx, player.selected_slot, true);
                Ok(())
            }
            "offhand" => {
                require_not_spectator(ctx)?;
                drop_slot(ctx, OFFHAND, true);
                Ok(())
            }
            "hotbar" => {
                require_not_spectator(ctx)?;
                for slot in 0..9 {
                    drop_slot(ctx, slot, true);
                }
                Ok(())
            }
            "inventory" => {
                require_not_spectator(ctx)?;
                for slot in 9..CONTAINER_SIZE {
                    drop_slot(ctx, slot, true);
                }
                Ok(())
            }
            "all" => {
                require_not_spectator(ctx)?;
                for slot in 0..CONTAINER_SIZE {
                    drop_slot(ctx, slot, true);
                }
                if ctx.env.selected_stack().is_some() || !offhand_is_empty(ctx) {
                    drop_slot(ctx, OFFHAND, true);
                }
                Ok(())
            }
            "armor" => {
                require_not_spectator(ctx)?;
                for slot in 36..40 {
                    drop_slot(ctx, slot, true);
                }
                Ok(())
            }
            _ => drop_item(ctx, &target),
        }
    }
}

fn require_not_spectator(ctx: &CmdCtx) -> CmdResult {
    if is_spectator(ctx) {
        return Err(CmdError::new("Can't drop items while in spectator."));
    }
    Ok(())
}

fn drop_slot(ctx: &mut CmdCtx, slot: usize, all: bool) {
    ctx.env
        .click_slot(slot, if all { 1 } else { 0 }, ClickMode::Throw);
}

fn offhand_is_empty(ctx: &CmdCtx) -> bool {
    !ctx.env
        .inventory()
        .into_iter()
        .any(|slot| slot.index == OFFHAND && slot.stack.map(|s| !s.is_empty()).unwrap_or(false))
}

/// `dropItem`: drop `amount` of one item id, slot by slot.
fn drop_item(ctx: &mut CmdCtx, raw: &str) -> CmdResult {
    require_not_spectator(ctx)?;
    let id = item_id(raw)?;
    let amount = if ctx.len() > 1 {
        crate::port::commands::int_between(ctx, 1, 1, i32::MAX)?
    } else {
        i32::MAX
    };

    let inventory = ctx.env.inventory();
    let mut left = amount;
    for slot in inventory {
        if left <= 0 {
            break;
        }
        let Some(stack) = slot.stack else { continue };
        if stack.is_empty() || stack.id != id {
            continue;
        }
        let count = stack.count.min(left);
        if count == stack.count {
            drop_slot(ctx, slot.index, true);
        } else {
            for _ in 0..count {
                drop_slot(ctx, slot.index, false);
            }
        }
        left -= count;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::meteor_core::env::{GameMode, InvSlot, ItemStackInfo};
    use crate::port::commands::command_manager;

    fn stack(id: &str, count: i32) -> Option<ItemStackInfo> {
        Some(ItemStackInfo {
            id: id.to_string(),
            count,
            ..Default::default()
        })
    }

    #[test]
    fn drop_hand_offhand_and_hotbar() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".drop hand"), Dispatch::Handled);
        assert_eq!(env.clicks, vec![(0, 1, ClickMode::Throw)]);

        assert_eq!(m.dispatch(&mut env, ".drop offhand"), Dispatch::Handled);
        assert_eq!(env.clicks.last().unwrap().0, 40);

        env.clicks.clear();
        assert_eq!(m.dispatch(&mut env, ".drop hotbar"), Dispatch::Handled);
        assert_eq!(env.clicks.len(), 9);
        assert!(env
            .clicks
            .iter()
            .all(|(_, button, mode)| *button == 1 && *mode == ClickMode::Throw));
    }

    #[test]
    fn drop_by_item_and_amount() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        env.inventory = vec![
            InvSlot {
                index: 0,
                stack: stack("minecraft:dirt", 5),
            },
            InvSlot {
                index: 9,
                stack: stack("minecraft:diamond", 2),
            },
        ];
        // Two dirt out of a stack of five: two single throws.
        assert_eq!(m.dispatch(&mut env, ".drop dirt 2"), Dispatch::Handled);
        assert_eq!(
            env.clicks,
            vec![(0, 0, ClickMode::Throw), (0, 0, ClickMode::Throw)]
        );
        env.clicks.clear();
        // The whole stack in one throw.
        assert_eq!(m.dispatch(&mut env, ".drop diamond 9"), Dispatch::Handled);
        assert_eq!(env.clicks, vec![(9, 1, ClickMode::Throw)]);
    }

    #[test]
    fn drop_error_paths() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".drop"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Specify what to drop")));
        assert_eq!(m.dispatch(&mut env, ".drop air"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Could not find an item with that name!")));

        env.player.as_mut().unwrap().game_mode = GameMode::Spectator;
        assert_eq!(m.dispatch(&mut env, ".drop hand"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Can't drop items while in spectator.")));
    }
}

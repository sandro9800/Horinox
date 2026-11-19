//! `systems.commands.GiveCommand` -?`.give <item> [1..99]`, creative only.
//!
//! `ServerboundSetCreativeModeSlotPacket` on the first empty hotbar slot, which is
//! exactly what Meteor does (`36 + fir.slot()`), plus the matching client-side
//! slot write; Horizon writes the slot through the same packet.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;

use crate::port::commands::args::item_id;
use crate::port::commands::{creative, creative_slot_packet, int_between};

pub struct GiveCommand;

impl Command for GiveCommand {
    fn name(&self) -> &'static str {
        "give"
    }

    fn description(&self) -> &'static str {
        "Gives you any item."
    }

    fn usage(&self) -> String {
        "give <item> [number]".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(raw) = ctx.arg(0).map(|s| s.to_string()) else {
            return Err(CmdError::new("Specify an item."));
        };
        let item = item_id(&raw)?;
        let count = if ctx.len() > 1 {
            int_between(ctx, 1, 1, 99)?
        } else {
            1
        };
        // `mc.player.getAbilities().instabuild`.
        creative(ctx)?;

        let inventory = ctx.env.inventory();
        let empty = |slot: usize| {
            !inventory.iter().any(|s| {
                s.index == slot && s.stack.as_ref().map(|st| !st.is_empty()).unwrap_or(false)
            })
        };
        let Some(hotbar_slot) = (0..9).find(|slot| empty(*slot)) else {
            return Err(CmdError::new("No space in hotbar."));
        };

        ctx.env.send_packet(creative_slot_packet(
            36 + hotbar_slot,
            &item,
            count,
            Vec::new(),
        ));
        ctx.info(&format!("Gave {} x{}.", item, count));
        Ok(())
    }
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
        env
    }

    #[test]
    fn give_fills_the_first_empty_hotbar_slot() {
        let m = command_manager();
        let mut env = creative_env();
        env.inventory = vec![
            InvSlot {
                index: 0,
                stack: Some(ItemStackInfo {
                    id: "minecraft:stone".into(),
                    count: 64,
                    ..Default::default()
                }),
            },
            InvSlot {
                index: 1,
                stack: None,
            },
        ];
        assert_eq!(m.dispatch(&mut env, ".give diamond"), Dispatch::Handled);
        assert_eq!(env.packets.len(), 1);
        let packet = &env.packets[0];
        assert_eq!(packet.name, "ServerboundSetCreativeModeSlotPacket");
        // Slot 0 is taken, so the packet addresses hotbar slot 1 => menu slot 37.
        assert_eq!(packet.fields[0], ("slot".to_string(), "37".to_string()));
        assert_eq!(
            packet.fields[1],
            ("item".to_string(), "minecraft:diamond".to_string())
        );
        assert_eq!(packet.fields[2], ("count".to_string(), "1".to_string()));

        assert_eq!(m.dispatch(&mut env, ".give diamond 16"), Dispatch::Handled);
        assert_eq!(
            env.packets[1].fields[2],
            ("count".to_string(), "16".to_string())
        );
    }

    #[test]
    fn give_error_paths() {
        let m = command_manager();
        // Survival.
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".give diamond"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("You must be in creative mode to use this.")));

        // Creative with a full hotbar.
        let mut env = creative_env();
        env.inventory = (0..9)
            .map(|index| InvSlot {
                index,
                stack: Some(ItemStackInfo {
                    id: "minecraft:stone".into(),
                    count: 64,
                    ..Default::default()
                }),
            })
            .collect();
        assert_eq!(m.dispatch(&mut env, ".give diamond"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("No space in hotbar.")));
        assert!(env.packets.is_empty());

        // Missing and invalid arguments.
        assert_eq!(m.dispatch(&mut env, ".give"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Specify an item.")));
        assert_eq!(m.dispatch(&mut env, ".give stone 200"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Integer must not be greater than 99")));
    }
}

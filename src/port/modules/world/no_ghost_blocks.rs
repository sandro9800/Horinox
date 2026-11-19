//! `systems.modules.world.NoGhostBlocks` - "attempts to prevent ghost blocks
//! arising" by re-syncing the block state after a break or a place.
//!
//! Java cancels `BreakBlockEvent` / `PlaceBlockEvent` - the client's *local*
//! prediction - so the block stays as the server last described it, and calls
//! `BlockState.playerWillDestroy` for the destroy effects. Horizon has no local
//! prediction hook, so the port asks the host for the block's authoritative
//! state after each action (`block_state_request`), which clears the same ghosts.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::BoolSet;

use super::{packet_pos, player_action_kind, pos_packet, PLAYER_ACTION, USE_ITEM_ON};

pub struct NoGhostBlocks {
    base: ModuleBase,
    breaking: BoolSet,
    pub placing: BoolSet,
    /// Positions the module asked to re-sync, newest last.
    resynced: Vec<crate::meteor_core::math::BlockPos>,
}

impl NoGhostBlocks {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "no-ghost-blocks",
            "Attempts to prevent ghost blocks arising.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let breaking = g.bool_setting("breaking", true);
        super::describe(g, "Whether to apply for block breaking actions.");

        let placing = g.bool_setting("placing", true);
        super::describe(g, "Whether to apply for block placement actions.");

        Self {
            base,
            breaking,
            placing,
            resynced: Vec::new(),
        }
    }

    /// The positions whose state the module has asked the server to repeat.
    pub fn resynced(&self) -> &[crate::meteor_core::math::BlockPos] {
        &self.resynced
    }

    fn resync(&mut self, env: &mut dyn Env, pos: crate::meteor_core::math::BlockPos) {
        self.resynced.push(pos);
        env.send_packet(pos_packet("block_state_request", pos));
    }
}

impl Default for NoGhostBlocks {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for NoGhostBlocks {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if packet.clientbound {
            return true;
        }

        if packet.name.eq_ignore_ascii_case(PLAYER_ACTION) && self.breaking.get() {
            let destroy = player_action_kind(packet).is_some_and(|kind| {
                kind.contains("destroy_block") || kind == "abort_destroy_block"
            });
            if destroy {
                if let Some(pos) = packet_pos(packet) {
                    // PORT GAP: `BreakBlockEvent.cancel()` +
                    // `blockState.getBlock().playerWillDestroy(...)` - the port
                    // re-reads the block instead of suppressing the prediction.
                    self.resync(env, pos);
                }
            }
            return true;
        }

        if packet.name.eq_ignore_ascii_case(USE_ITEM_ON) && self.placing.get() {
            if let Some(pos) = packet_pos(packet) {
                // PORT GAP: `PlaceBlockEvent.cancel()`.
                self.resync(env, pos);
            }
        }

        true
    }

    fn on_game_joined(&mut self, _env: &mut dyn Env) {
        self.resynced.clear();
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(NoGhostBlocks::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::BlockPos;

    fn action_packet(name: &str, action: &str, pos: BlockPos) -> PacketInfo {
        let mut packet = pos_packet(name, pos);
        packet
            .fields
            .push(("action".to_string(), action.to_string()));
        packet
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = NoGhostBlocks::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["breaking", "placing"]);
        assert_eq!(m.base.title, "No Ghost Blocks");
        assert_eq!(
            m.base.description,
            "Attempts to prevent ghost blocks arising."
        );
    }

    #[test]
    fn breaking_asks_the_server_to_repeat_the_block_state() {
        let mut m = NoGhostBlocks::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(4, 70, -4);

        let mut packet = action_packet("player_action", "STOP_DESTROY_BLOCK", pos);
        assert!(
            m.on_send_packet(&mut env, &mut packet),
            "the packet still goes"
        );

        assert_eq!(env.packets.len(), 1, "one re-sync request");
        assert_eq!(env.packets[0].name, "block_state_request");
        assert_eq!(super::super::packet_pos(&env.packets[0]), Some(pos));
        assert_eq!(m.resynced(), &[pos]);
    }

    #[test]
    fn the_two_toggles_gate_their_own_action() {
        let mut m = NoGhostBlocks::new();
        let mut env = TestEnv::in_world();

        m.breaking.set(false);
        let mut packet = action_packet(
            "player_action",
            "START_DESTROY_BLOCK",
            BlockPos::new(1, 2, 3),
        );
        m.on_send_packet(&mut env, &mut packet);
        assert!(env.packets.is_empty(), "breaking is off");

        let mut place = pos_packet("use_item_on", BlockPos::new(5, 6, 7));
        m.on_send_packet(&mut env, &mut place);
        assert_eq!(env.packets.len(), 1, "placing is on by default");

        m.placing.set(false);
        let mut place = pos_packet("use_item_on", BlockPos::new(5, 6, 7));
        m.on_send_packet(&mut env, &mut place);
        assert_eq!(env.packets.len(), 1, "placing is off now");

        // Unrelated packets are ignored either way.
        let mut other = pos_packet("chat_message", BlockPos::new(0, 0, 0));
        m.on_send_packet(&mut env, &mut other);
        assert_eq!(env.packets.len(), 1);
    }

    #[test]
    fn joining_a_game_forgets_the_old_requests() {
        let mut m = NoGhostBlocks::new();
        let mut env = TestEnv::in_world();
        let mut packet = action_packet(
            "player_action",
            "ABORT_DESTROY_BLOCK",
            BlockPos::new(1, 1, 1),
        );
        m.on_send_packet(&mut env, &mut packet);
        assert_eq!(m.resynced().len(), 1);
        m.on_game_joined(&mut env);
        assert!(m.resynced().is_empty());
    }
}

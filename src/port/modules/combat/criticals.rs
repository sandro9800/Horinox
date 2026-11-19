//! `systems.modules.combat.Criticals` -?Performs critical attacks when you hit your target.
//!
//! Java rewrites the outgoing attack into a position-packet burst (or a jump) so the
//! server sees the hit while falling. `Env` can send raw packets, so the burst is
//! reproduced as `move_player` packets; the jump modes use the action set.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env, PacketInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet};

pub struct Criticals {
    base: ModuleBase,
    mode: EnumSet,
    ka: BoolSet,
    mace: BoolSet,
    extra_height: DoubleSet,
    // Runtime
    send_packets: bool,
    send_timer: i32,
}

/// `Criticals.Mode`.
const MODES: &[&str] = &[
    "none",
    "packet",
    "updated-ncp",
    "old-ncp",
    "jump",
    "mini-jump",
];

impl Criticals {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "criticals",
            "Performs critical attacks when you hit your target.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let mode = g.enum_setting("mode", MODES, 1);
        let ka = g.bool_setting("only-killaura", false);
        s.create_group("Mace");
        let mace = super::last_group(s).bool_setting("smash-attack", true);
        let extra_height =
            super::last_group(s).double_setting("additional-height", 0.0, 0.0, 100.0, 1);
        Self {
            base,
            mode,
            ka,
            mace,
            extra_height,
            send_packets: false,
            send_timer: 0,
        }
    }

    /// `Criticals.skipCrit`.
    fn skip_crit(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return true;
        };
        let jump_mode = self.mode.is("jump") || self.mode.is("mini-jump");
        if jump_mode
            && env
                .block(player.pos.to_block_pos())
                .map(|b| b.id == "minecraft:cobweb")
                .unwrap_or(false)
        {
            return true;
        }
        !player.on_ground || player.in_water || player.in_lava
    }

    /// `Criticals.sendPacket(height)`: a `ServerboundMovePlayerPacket.Pos`.
    fn send_move(env: &mut dyn Env, height: f64) {
        let Some(player) = env.player() else { return };
        env.send_packet(PacketInfo {
            name: "minecraft:move_player".into(),
            clientbound: false,
            fields: vec![
                (
                    "pos".into(),
                    format!(
                        "{}, {}, {}",
                        player.pos.x,
                        player.pos.y + height,
                        player.pos.z
                    ),
                ),
                ("on_ground".into(), "false".into()),
                // `IServerboundMovePlayerPacket.meteor$setTag(1337)`
                ("tag".into(), "1337".into()),
            ],
        });
    }

    fn is_mace(env: &dyn Env) -> bool {
        env.selected_stack()
            .map(|s| s.id == "minecraft:mace")
            .unwrap_or(false)
    }
}

impl Default for Criticals {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Criticals {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.send_packets = false;
        self.send_timer = 0;
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !packet.name.contains("attack") {
            return true;
        }
        if self.mace.get() && Self::is_mace(env) {
            if env
                .player()
                .map(|p| p.velocity.y.abs() > 0.0)
                .unwrap_or(false)
            {
                return true;
            }
            Self::send_move(env, 0.0);
            Self::send_move(env, 1.501 + self.extra_height.get());
            Self::send_move(env, 0.0);
            return true;
        }
        if self.skip_crit(env) {
            return true;
        }
        if self.ka.get() {
            // PORT GAP: `Modules.get(KillAura.class).getTarget()` is a cross-module
            // field read; `Env` can only ask whether KillAura is active.
            if !env.module_active("kill-aura") {
                return true;
            }
        }
        match self.mode.index() {
            1 => {
                Self::send_move(env, 0.0625);
                Self::send_move(env, 0.0);
            }
            2 => {
                Self::send_move(env, 0.0000008);
                Self::send_move(env, 0.0);
            }
            3 => {
                Self::send_move(env, 0.11);
                Self::send_move(env, 0.1100013579);
                Self::send_move(env, 0.0000013579);
            }
            4 | 5 => {
                if !self.send_packets {
                    self.send_packets = true;
                    // PORT GAP: `ServerboundAttackPacket` cannot be cancelled and
                    // replayed from here (`on_send_packet` sees a decoded name, not
                    // the packet object), so the jump is issued alongside it.
                    if self.mode.index() == 4 {
                        env.action(Action::Jump);
                    } else {
                        self.send_timer = 4;
                        env.action(Action::Jump);
                    }
                }
            }
            _ => {}
        }
        true
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {
        if !self.send_packets {
            return;
        }
        if self.mode.index() == 4 {
            // Wait for the peaking tick before hitting.
            return;
        }
        if self.send_timer <= 0 {
            self.send_packets = false;
        } else {
            self.send_timer -= 1;
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.mode.get().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};

    fn attack_packet() -> PacketInfo {
        PacketInfo {
            name: "minecraft:attack".into(),
            clientbound: false,
            fields: vec![("entity_id".into(), "7".into())],
        }
    }

    #[test]
    fn packet_mode_bursts_position_packets() {
        let mut m = Criticals::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_send_packet(&mut env, &mut attack_packet());

        assert_eq!(env.packets.len(), 2, "two position packets");
        assert!(env.packets[0].name == "minecraft:move_player");
        assert_eq!(m.info_string().as_deref(), Some("packet"));
    }

    #[test]
    fn mace_smash_uses_the_full_height() {
        let mut m = Criticals::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:mace".into(),
                count: 1,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        m.extra_height.set(2.0);
        m.on_send_packet(&mut env, &mut attack_packet());
        assert_eq!(env.packets.len(), 3);
        assert!(env.packets[1]
            .fields
            .iter()
            .any(|(_, v)| v.contains("67.501")));
    }

    #[test]
    fn airborne_attacks_are_skipped() {
        let mut m = Criticals::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().on_ground = false;
        m.on_send_packet(&mut env, &mut attack_packet());
        assert!(env.packets.is_empty());
    }
}

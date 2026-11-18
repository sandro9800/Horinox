//! `systems.modules.player.AntiHunger` - reduces (does NOT remove) hunger
//! consumption by spoofing the sprint and `onGround` flags.
//!
//! PORT GAP: `Player.isPassenger()`/`fallDistance`/`gameMode.isDestroying()` are
//! not part of `PlayerInfo`, so the two guards those feed only check the state the
//! port carries (water and `on_ground`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::BoolSet;

use super::{describe, field, set_field};

pub struct AntiHunger {
    base: ModuleBase,
    sprint: BoolSet,
    on_ground: BoolSet,
    last_on_ground: bool,
    ignore_packet: bool,
}

impl AntiHunger {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "anti-hunger",
            "Reduces (does NOT remove) hunger consumption.",
        );
        let (sprint, on_ground);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            sprint = g.bool_setting("sprint", true);
            on_ground = g.bool_setting("on-ground", true);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "sprint", "Spoofs sprinting packets.");
        describe(&mut s, "on-ground", "Spoofs the onGround flag.");

        Self {
            base,
            sprint,
            on_ground,
            last_on_ground: false,
            ignore_packet: false,
        }
    }
}

impl Default for AntiHunger {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AntiHunger {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.last_on_ground = env.player().map(|p| p.on_ground).unwrap_or(false);
        self.ignore_packet = false;
    }

    /// `SendMovementPacketsEvent.Pre`: remember the landing so the packet that
    /// reports it is left alone (otherwise the player stops taking fall damage).
    fn on_player_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        if player.on_ground && !self.last_on_ground && self.on_ground.get() {
            self.ignore_packet = true;
        }
        self.last_on_ground = player.on_ground;
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        let is_move = packet.name.eq_ignore_ascii_case("move_player");
        if self.ignore_packet && is_move {
            self.ignore_packet = false;
            return true;
        }

        let Some(player) = env.player() else {
            return true;
        };
        if player.in_water || player.in_lava {
            return true;
        }

        if packet.name.eq_ignore_ascii_case("player_command") && self.sprint.get() {
            if field(packet, "action")
                .map(|a| a.eq_ignore_ascii_case("start_sprinting"))
                .unwrap_or(false)
            {
                return false;
            }
        }

        if is_move && self.on_ground.get() && player.on_ground {
            set_field(packet, "on_ground", "false");
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn packet(name: &str, key: &str, value: &str) -> PacketInfo {
        PacketInfo {
            name: name.to_string(),
            clientbound: false,
            fields: vec![(key.to_string(), value.to_string())],
        }
    }

    #[test]
    fn sprint_packets_are_cancelled_and_on_ground_is_spoofed() {
        let mut m = AntiHunger::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);

        let mut sprint = packet("player_command", "action", "start_sprinting");
        assert!(
            !m.on_send_packet(&mut env, &mut sprint),
            "sprint packets are cancelled"
        );

        let mut move_packet = packet("move_player", "on_ground", "true");
        assert!(m.on_send_packet(&mut env, &mut move_packet));
        assert_eq!(field(&move_packet, "on_ground"), Some("false"));
    }

    #[test]
    fn water_leaves_the_packets_alone() {
        let mut m = AntiHunger::new();
        let mut env = TestEnv::in_world();
        if let Some(player) = env.player.as_mut() {
            player.in_water = true;
        }
        m.on_activate(&mut env);

        let mut move_packet = packet("move_player", "on_ground", "true");
        assert!(m.on_send_packet(&mut env, &mut move_packet));
        assert_eq!(field(&move_packet, "on_ground"), Some("true"));
    }
}

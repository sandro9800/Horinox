//! `systems.modules.movement.Sprint` - automatically sprints.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env, PacketInfo, Screen};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet};

use super::{forward_input, strafe_input};

pub struct Sprint {
    base: ModuleBase,
    // General
    /// `sprint-mode`
    pub mode: EnumSet,
    /// `keep-sprint`
    keep_sprint: BoolSet,
    /// `unsprint-on-hit`
    unsprint_on_hit: BoolSet,
    /// `unsprint-in-water`
    pub unsprint_in_water: BoolSet,
    /// `sprint-while-stationary`
    perma_sprint: BoolSet,
}

impl Sprint {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Movement, "sprint", "Automatically sprints.");
        let mut s = base.settings_mut();
        let mode = s
            .default_group()
            .enum_setting("sprint-mode", &["Strict", "Rage"], 0);
        s.describe_last("What mode of sprinting.");
        let keep_sprint = s.default_group().bool_setting("keep-sprint", false);
        s.describe_last("Whether to keep sprinting after attacking.");
        let unsprint_on_hit = s.default_group().bool_setting("unsprint-on-hit", false);
        s.describe_last(
            "Whether to stop sprinting before attacking, to ensure you get crits and sweep attacks.",
        );
        let unsprint_in_water = s.default_group().bool_setting("unsprint-in-water", true);
        s.describe_last("Whether to stop sprinting when in water.");
        {
            let c = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("Rage")));
        }
        let perma_sprint = s
            .default_group()
            .bool_setting("sprint-while-stationary", false);
        s.describe_last("Sprint even when not moving.");
        {
            let c = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("Rage")));
        }

        Self {
            base,
            mode,
            keep_sprint,
            unsprint_on_hit,
            unsprint_in_water,
            perma_sprint,
        }
    }

    /// `shouldSprint()`.
    pub fn should_sprint(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };

        // `mc.gui.screen() != null && !GUIMove.sprint`.
        if env.screen() != Screen::Playing && !env.module_active("gui-move") {
            return false;
        }

        let movement = if self.mode.is("Rage") {
            forward_input(env).abs() + strafe_input(env).abs()
        } else {
            forward_input(env)
        };

        if movement <= if player.in_water { 1.0e-5 } else { 0.8 } {
            if self.mode.is("Strict") || !self.perma_sprint.get() {
                return false;
            }
        }

        // `isInShallowWater`, `isMobilityRestricted`, `isPassenger`,
        // `hasEnoughFood` and `horizontalCollision` are not in `PlayerInfo`:
        // PORT GAP. The food gate is the one Meteor's strict mode relies on most.
        let strict_sprint = player.food > 6;

        self.mode.is("Rage") || strict_sprint
    }

    /// `rageSprint()`.
    pub fn rage_sprint(&self) -> bool {
        self.mode.is("Rage")
    }

    /// `stopSprinting()`.
    pub fn stop_sprinting(&self) -> bool {
        !self.keep_sprint.get()
    }

    fn unsprint(&self, env: &mut dyn Env) {
        env.send_packet(PacketInfo {
            name: "ServerboundPlayerCommandPacket".to_string(),
            clientbound: false,
            fields: vec![("action".to_string(), "STOP_SPRINTING".to_string())],
        });
        env.action(Action::Sprint(false));
    }

    fn resume_sprint(&self, env: &mut dyn Env) {
        env.send_packet(PacketInfo {
            name: "ServerboundPlayerCommandPacket".to_string(),
            clientbound: false,
            fields: vec![("action".to_string(), "START_SPRINTING".to_string())],
        });
        env.action(Action::Sprint(true));
    }
}

impl Default for Sprint {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Sprint {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.unsprint_in_water.get() && env.player().map(|p| p.in_water).unwrap_or(false) {
            return;
        }

        let sprint = self.should_sprint(env);
        env.action(Action::Sprint(sprint));
    }

    /// `onPacketSend`: stop sprinting right before an attack lands.
    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !self.unsprint_on_hit.get() {
            return true;
        }
        if packet.name != "ServerboundAttackPacket" && packet.name != "ServerboundInteractPacket" {
            return true;
        }
        self.unsprint(env);
        true
    }

    fn info_string(&self) -> Option<String> {
        Some(self.mode.get().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Sprint::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn walking_sprints() {
        let mut m = Sprint::new();
        let mut env = TestEnv::in_world();
        env.keys = vec![super::super::keys::W];

        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Sprint(true)]);
    }

    #[test]
    fn standing_still_does_not_sprint() {
        let mut m = Sprint::new();
        let mut env = TestEnv::in_world();

        assert!(!m.should_sprint(&env));
        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Sprint(false)]);
    }

    #[test]
    fn rage_mode_sprints_while_stationary() {
        let mut m = Sprint::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(1); // Rage
        m.perma_sprint.set(true);

        assert!(m.rage_sprint());
        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Sprint(true)]);
    }

    #[test]
    fn attacking_can_stop_the_sprint() {
        let mut m = Sprint::new();
        let mut env = TestEnv::in_world();
        m.unsprint_on_hit.set(true);

        let mut packet = PacketInfo {
            name: "ServerboundAttackPacket".to_string(),
            clientbound: false,
            fields: Vec::new(),
        };
        assert!(m.on_send_packet(&mut env, &mut packet));
        assert_eq!(
            env.packets[0].fields[0].1, "STOP_SPRINTING",
            "packets were {:?}",
            env.packets
        );
        assert_eq!(env.actions, vec![Action::Sprint(false)]);
    }

    #[test]
    fn water_stops_the_tick() {
        let mut m = Sprint::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().in_water = true;
        env.keys = vec![super::super::keys::W];

        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
    }
}

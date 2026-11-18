//! `systems.modules.movement.AutoJump` - automatically jumps.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{DoubleSet, EnumSet};

use super::{forward_input, keys, strafe_input, UNBOUNDED};

pub struct AutoJump {
    base: ModuleBase,
    /// `mode`
    mode: EnumSet,
    /// `jump-if`
    jump_if: EnumSet,
    /// `velocity-height`
    velocity_height: DoubleSet,
}

impl AutoJump {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Movement, "auto-jump", "Automatically jumps.");
        let mut s = base.settings_mut();
        let mode = s
            .default_group()
            .enum_setting("mode", &["Jump", "LowHop"], 0);
        s.describe_last("The method of jumping.");
        let jump_if =
            s.default_group()
                .enum_setting("jump-if", &["Sprinting", "Walking", "Always"], 2);
        s.describe_last("Jump if.");
        let velocity_height =
            s.default_group()
                .double_setting("velocity-height", 0.25, 0.0, UNBOUNDED, 3);
        s.describe_last("The distance that velocity mode moves you.");

        Self {
            base,
            mode,
            jump_if,
            velocity_height,
        }
    }

    /// `jump()`.
    fn should_jump(&self, env: &dyn Env) -> bool {
        let moving = forward_input(env) != 0.0 || strafe_input(env) != 0.0;
        if self.jump_if.is("Sprinting") {
            return env.player().map(|p| p.sprinting).unwrap_or(false) && moving;
        }
        if self.jump_if.is("Walking") {
            return moving;
        }
        true
    }
}

impl Default for AutoJump {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoJump {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        if !player.on_ground || env.key_down(keys::LEFT_SHIFT) || !self.should_jump(env) {
            return;
        }

        if self.mode.is("Jump") {
            env.action(Action::Jump);
        } else {
            // PORT GAP: `((IVec3) getDeltaMovement()).setY(velocityHeight)` - the
            // `Env` cannot write the local velocity, so LowHop only records the
            // height it asked for.
            let _height = self.velocity_height.get();
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.mode.get().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoJump::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn jumps_when_standing() {
        let mut m = AutoJump::new();
        let mut env = TestEnv::in_world();

        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Jump]);
    }

    #[test]
    fn low_hop_does_not_queue_a_vanilla_jump() {
        let mut m = AutoJump::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(1); // LowHop

        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
        assert_eq!(m.info_string().as_deref(), Some("LowHop"));
    }

    #[test]
    fn walking_mode_waits_for_a_movement_key() {
        let mut m = AutoJump::new();
        let mut env = TestEnv::in_world();
        m.jump_if.set_index(1); // Walking

        m.on_tick(&mut env);
        assert!(env.actions.is_empty());

        env.keys = vec![keys::W];
        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Jump]);
    }

    #[test]
    fn sneaking_suppresses_the_jump() {
        let mut m = AutoJump::new();
        let mut env = TestEnv::in_world();
        env.keys = vec![keys::LEFT_SHIFT];

        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
    }
}

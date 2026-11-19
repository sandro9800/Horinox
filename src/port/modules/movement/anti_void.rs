//! `systems.modules.movement.AntiVoid` - attempts to prevent you from falling
//! into the void.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::EnumSet;

pub struct AntiVoid {
    base: ModuleBase,
    /// `mode`
    mode: EnumSet,
    was_flight_enabled: bool,
    has_run: bool,
}

impl AntiVoid {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "anti-void",
            "Attempts to prevent you from falling into the void.",
        );
        let mut s = base.settings_mut();
        let mode = s
            .default_group()
            .enum_setting("mode", &["Flight", "Jump"], 1);
        s.describe_last("The method to prevent you from falling into the void.");

        Self {
            base,
            mode,
            was_flight_enabled: false,
            has_run: false,
        }
    }
}

impl Default for AntiVoid {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AntiVoid {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        if self.mode.is("Flight") {
            self.was_flight_enabled = env.module_active("flight");
        }
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        if !self.was_flight_enabled && self.mode.is("Flight") && env.can_update() {
            env.request_toggle("flight");
        }
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let min_y = env.world().map(|w| w.min_y).unwrap_or(-64);
        let Some(player) = env.player() else { return };
        let y = player.pos.y;

        if y > min_y as f64 || y < (min_y - 15) as f64 {
            if self.has_run && self.mode.is("Flight") {
                env.request_toggle("flight");
                self.has_run = false;
            }
            return;
        }

        if self.mode.is("Flight") {
            env.request_toggle("flight");
            self.has_run = true;
        } else {
            env.action(Action::Jump);
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AntiVoid::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn below_the_world_height_it_jumps() {
        let mut m = AntiVoid::new();
        let mut env = TestEnv::in_world();
        // The test world's min_y is -64, so -65 is one block into the void.
        env.player.as_mut().unwrap().pos.y = -65.0;

        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Jump]);
    }

    #[test]
    fn above_the_void_nothing_happens() {
        let mut m = AntiVoid::new();
        let mut env = TestEnv::in_world();

        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
        assert!(env.commands.is_empty());
    }

    #[test]
    fn flight_mode_asks_for_the_flight_module() {
        let mut m = AntiVoid::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(0); // Flight
        env.player.as_mut().unwrap().pos.y = -70.0;

        m.on_tick(&mut env);
        assert!(m.has_run);
        assert!(env.actions.is_empty(), "flight mode does not jump");
    }
}

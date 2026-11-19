//! `systems.modules.movement.AirJump` - lets you jump in the air.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env, Screen};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::BoolSet;

use super::keys;

pub struct AirJump {
    base: ModuleBase,
    /// `maintain-level`
    maintain_level: BoolSet,
    /// `level` - the Y level the module keeps you at.
    level: i32,
}

impl AirJump {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Movement, "air-jump", "Lets you jump in the air.");
        let mut s = base.settings_mut();
        let maintain_level = s.default_group().bool_setting("maintain-level", false);
        s.describe_last("Maintains your current Y level when holding the jump key.");

        Self {
            base,
            maintain_level,
            level: 0,
        }
    }

    fn block_y(env: &dyn Env) -> i32 {
        env.player().map(|p| p.pos.to_block_pos().y).unwrap_or(0)
    }

    /// `Modules.get().isActive(Freecam.class)`.
    fn freecam(env: &dyn Env) -> bool {
        env.module_active("freecam")
    }
}

impl Default for AirJump {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AirJump {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.level = Self::block_y(env);
    }

    /// `onKey(KeyInputEvent)`: a press of the jump key jumps, sneak lowers the
    /// maintained level by one.
    fn on_key(&mut self, env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        if !is_key || !pressed {
            return false;
        }
        let Some(player) = env.player() else {
            return false;
        };
        if Self::freecam(env) || env.screen() != Screen::Playing || player.on_ground {
            return false;
        }

        if value == keys::SPACE {
            self.level = player.pos.to_block_pos().y;
            env.action(Action::Jump);
        } else if value == keys::LEFT_SHIFT {
            self.level -= 1;
        }
        false
    }

    /// `onTick(TickEvent.Pre)`: hold jump to stay at the level you jumped at.
    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        if Self::freecam(env) || player.on_ground {
            return;
        }

        if self.maintain_level.get()
            && player.pos.to_block_pos().y == self.level
            && env.key_down(keys::SPACE)
        {
            env.action(Action::Jump);
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AirJump::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn jumping_in_the_air_queues_a_jump() {
        let mut m = AirJump::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().on_ground = false;

        m.on_key(&mut env, true, keys::SPACE, true);
        assert_eq!(env.actions, vec![Action::Jump]);
        assert_eq!(m.level, 64);
    }

    #[test]
    fn jumping_on_the_ground_is_left_to_vanilla() {
        let mut m = AirJump::new();
        let mut env = TestEnv::in_world();

        m.on_key(&mut env, true, keys::SPACE, true);
        assert!(env.actions.is_empty());
    }

    #[test]
    fn maintain_level_holds_the_y_level() {
        let mut m = AirJump::new();
        let mut env = TestEnv::in_world();
        m.maintain_level.set(true);
        env.player.as_mut().unwrap().on_ground = false;
        m.level = 64;
        env.keys = vec![keys::SPACE];

        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Jump]);

        // One block lower: nothing happens until the level matches again.
        env.actions.clear();
        env.player.as_mut().unwrap().pos.y = 70.0;
        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
    }
}

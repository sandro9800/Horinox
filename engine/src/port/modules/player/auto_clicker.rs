//! `systems.modules.player.AutoClicker` - automatically clicks.
//!
//! PORT GAP: `Utils.leftClick()`/`Utils.rightClick()` and `keyAttack.setDown`
//! are client input; the port sends the matching [`Action`] instead, so `Hold`
//! repeats the action every tick rather than holding the key down.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env, Screen};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet};

use super::{describe, visible};

pub struct AutoClicker {
    base: ModuleBase,
    in_screens: BoolSet,
    left_mode: EnumSet,
    left_delay: IntSet,
    right_mode: EnumSet,
    right_delay: IntSet,
    right_timer: i32,
    left_timer: i32,
}

impl AutoClicker {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Player, "auto-clicker", "Automatically clicks.");
        let (in_screens, left_mode, left_delay, right_mode, right_delay);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            in_screens = g.bool_setting("while-in-screens", true);
            left_mode = g.enum_setting("mode-left", &["disabled", "hold", "press"], 2);
            left_delay = g.int_setting("delay-left", 2, 0, 60);
            right_mode = g.enum_setting("mode-right", &["disabled", "hold", "press"], 2);
            right_delay = g.int_setting("delay-right", 2, 0, 60);
        }
        let mut s = base.settings_mut();
        describe(
            &mut s,
            "while-in-screens",
            "Whether to click while a screen is open.",
        );
        describe(
            &mut s,
            "mode-left",
            "The method of clicking for left clicks.",
        );
        describe(
            &mut s,
            "delay-left",
            "The amount of delay between left clicks in ticks.",
        );
        describe(
            &mut s,
            "mode-right",
            "The method of clicking for right clicks.",
        );
        describe(
            &mut s,
            "delay-right",
            "The amount of delay between right clicks in ticks.",
        );
        visible(&mut s, "delay-left", {
            let c = left_mode.clone();
            move || c.is("press")
        });
        visible(&mut s, "delay-right", {
            let c = right_mode.clone();
            move || c.is("press")
        });

        Self {
            base,
            in_screens,
            left_mode,
            left_delay,
            right_mode,
            right_delay,
            right_timer: 0,
            left_timer: 0,
        }
    }
}

impl Default for AutoClicker {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoClicker {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.right_timer = 0;
        self.left_timer = 0;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if !self.in_screens.get() && env.screen() != Screen::Playing {
            return;
        }

        match self.left_mode.index() {
            1 => env.action(Action::Attack),
            2 => {
                self.left_timer += 1;
                if self.left_timer > self.left_delay.get() {
                    env.action(Action::Attack);
                    self.left_timer = 0;
                }
            }
            _ => {}
        }

        match self.right_mode.index() {
            1 => env.action(Action::Use),
            2 => {
                self.right_timer += 1;
                if self.right_timer > self.right_delay.get() {
                    env.action(Action::Use);
                    self.right_timer = 0;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn press_mode_clicks_on_the_configured_delay() {
        let mut m = AutoClicker::new();
        let mut env = TestEnv::in_world();
        m.left_delay.set(0);
        m.right_delay.set(0);

        m.on_tick(&mut env);
        assert!(env.actions.contains(&Action::Attack), "left click");
        assert!(env.actions.contains(&Action::Use), "right click");
    }

    #[test]
    fn screens_are_skipped_unless_allowed() {
        let mut m = AutoClicker::new();
        let mut env = TestEnv::in_world();
        env.screen = Screen::Chat;
        m.in_screens.set(false);
        m.left_delay.set(0);
        m.right_delay.set(0);

        m.on_tick(&mut env);
        assert!(env.actions.is_empty());

        m.in_screens.set(true);
        m.on_tick(&mut env);
        assert_eq!(env.actions.len(), 2);
    }
}

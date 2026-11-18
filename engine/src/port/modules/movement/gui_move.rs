//! `systems.modules.movement.GUIMove` - allows you to perform various actions
//! while in GUIs.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Screen};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Rotation;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet};

use super::{keys, UNBOUNDED};

pub struct GuiMove {
    base: ModuleBase,
    // General
    /// `guis`
    screens: EnumSet,
    /// `jump`
    pub jump: BoolSet,
    /// `sneak`
    pub sneak: BoolSet,
    /// `sprint`
    pub sprint: BoolSet,
    /// `arrows-rotate`
    arrows_rotate: BoolSet,
    /// `rotate-speed`
    rotate_speed: DoubleSet,

    was_skipped: bool,
}

impl GuiMove {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "gui-move",
            "Allows you to perform various actions while in GUIs.",
        );
        let mut s = base.settings_mut();
        let screens = s
            .default_group()
            .enum_setting("guis", &["GUI", "Inventory", "Both"], 1);
        s.describe_last("Which GUIs to move in.");
        let jump = s.default_group().bool_setting("jump", true);
        s.describe_last("Allows you to jump while in GUIs.");
        let sneak = s.default_group().bool_setting("sneak", true);
        s.describe_last("Allows you to sneak while in GUIs.");
        let sprint = s.default_group().bool_setting("sprint", true);
        s.describe_last("Allows you to sprint while in GUIs.");
        let arrows_rotate = s.default_group().bool_setting("arrows-rotate", true);
        s.describe_last("Allows you to use your arrow keys to rotate while in GUIs.");
        let rotate_speed = s
            .default_group()
            .double_setting("rotate-speed", 4.0, 0.0, UNBOUNDED, 3);
        s.describe_last("Rotation speed while in GUIs.");

        Self {
            base,
            screens,
            jump,
            sneak,
            sprint,
            arrows_rotate,
            rotate_speed,
            was_skipped: true,
        }
    }

    /// `skip()`:
    ///
    /// PORT GAP: `WidgetScreen` (Meteor's own screens) and
    /// `AbstractContainerScreen` (inventories) are one `Screen::Other` in the
    /// `Env`, so `guis` cannot tell them apart; the chat, sign, anvil, command
    /// block and edit-box exclusions are all `Screen::Chat`/`Other` here too.
    pub fn skip(&self, env: &dyn Env) -> bool {
        matches!(env.screen(), Screen::Playing | Screen::Chat | Screen::Death)
    }

    pub fn disable_space(&self, env: &dyn Env) -> bool {
        self.jump.get() && !self.skip(env)
    }

    pub fn disable_arrows(&self, env: &dyn Env) -> bool {
        self.arrows_rotate.get() && !self.skip(env)
    }

    fn unpress(&mut self) {
        // PORT GAP: releasing `keyUp/keyDown/keyLeft/keyRight` has no `Env`
        // write; only the recorded state is dropped.
        self.was_skipped = true;
    }
}

impl Default for GuiMove {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for GuiMove {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.was_skipped = true;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.was_skipped = true;
    }

    /// The arrow-key rotation the original runs in `Render3DEvent`; a client tick
    /// is the port's per-frame hook.
    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.skip(env) {
            if !self.was_skipped {
                self.unpress();
            }
            return;
        }
        self.was_skipped = false;

        let _ = &self.screens;

        if !self.arrows_rotate.get() {
            return;
        }

        // `Math.min(rotateSpeed * frameTime * 20, 100)`; one tick is 1/20 s.
        let delta = self.rotate_speed.get() as f32;

        let (mut yaw, mut pitch) = match env.player() {
            Some(p) => (p.yaw, p.pitch),
            None => (env.rotation().yaw, env.rotation().pitch),
        };

        if env.key_down(keys::LEFT) {
            yaw -= delta;
        }
        if env.key_down(keys::RIGHT) {
            yaw += delta;
        }
        if env.key_down(keys::UP) {
            pitch -= delta;
        }
        if env.key_down(keys::DOWN) {
            pitch += delta;
        }

        pitch = pitch.clamp(-90.0, 90.0);
        env.set_rotation(Rotation::new(yaw, pitch));
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(GuiMove::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn arrow_keys_rotate_inside_a_gui() {
        let mut m = GuiMove::new();
        let mut env = TestEnv::in_world();
        env.screen = Screen::Other;
        env.keys = vec![keys::LEFT];

        m.on_tick(&mut env);
        assert!(
            (env.rotation.yaw + 4.0).abs() < 1.0e-4,
            "yaw {}",
            env.rotation.yaw
        );
        assert!(!m.was_skipped);
    }

    #[test]
    fn nothing_rotates_in_the_world() {
        let mut m = GuiMove::new();
        let mut env = TestEnv::in_world();
        env.keys = vec![keys::LEFT];

        m.on_tick(&mut env);
        assert_eq!(env.rotation.yaw, 0.0);
        assert!(m.was_skipped);
    }

    #[test]
    fn pitch_is_clamped() {
        let mut m = GuiMove::new();
        let mut env = TestEnv::in_world();
        env.screen = Screen::Other;
        env.player.as_mut().unwrap().pitch = 88.0;
        m.rotate_speed.set(20.0);
        env.keys = vec![keys::DOWN];

        m.on_tick(&mut env);
        assert_eq!(env.rotation.pitch, 90.0);
    }

    #[test]
    fn the_chat_screen_is_skipped() {
        let mut m = GuiMove::new();
        let mut env = TestEnv::in_world();
        env.screen = Screen::Chat;

        assert!(m.skip(&env));
    }
}

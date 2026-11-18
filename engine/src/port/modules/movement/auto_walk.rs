//! `systems.modules.movement.AutoWalk` - automatically walks forward.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Screen};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet};

use super::keys;

pub struct AutoWalk {
    base: ModuleBase,
    // General
    /// `mode`
    mode: EnumSet,
    /// `simple-direction`
    direction: EnumSet,
    /// `disable-on-input`
    disable_on_input: BoolSet,
    /// `disable-on-y-change`
    disable_on_y: BoolSet,
    /// `no-unloaded-chunks`
    wait_for_chunks: BoolSet,

    /// The direction key simple mode is holding, as `0..4`.
    pressed: Option<usize>,
}

impl AutoWalk {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "auto-walk",
            "Automatically walks forward.",
        );
        let mut s = base.settings_mut();
        let mode = s
            .default_group()
            .enum_setting("mode", &["Simple", "Smart"], 1);
        s.describe_last("Walking mode.");
        let direction = s.default_group().enum_setting(
            "simple-direction",
            &["Forwards", "Backwards", "Left", "Right"],
            0,
        );
        s.describe_last("The direction to walk in simple mode.");
        {
            let c = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("Simple")));
        }
        let disable_on_input = s.default_group().bool_setting("disable-on-input", false);
        s.describe_last("Disable module on manual movement input");
        let disable_on_y = s.default_group().bool_setting("disable-on-y-change", false);
        s.describe_last("Disable module if player moves vertically");
        {
            let c = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("Simple")));
        }
        let wait_for_chunks = s.default_group().bool_setting("no-unloaded-chunks", true);
        s.describe_last("Do not allow movement into unloaded chunks");
        {
            let c = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("Simple")));
        }

        Self {
            base,
            mode,
            direction,
            disable_on_input,
            disable_on_y,
            wait_for_chunks,
            pressed: None,
        }
    }

    /// `unpress()` - the port keeps the intent, not the key state.
    fn unpress(&mut self) {
        self.pressed = None;
    }

    fn is_movement_key(value: i32) -> bool {
        matches!(
            value,
            keys::W | keys::A | keys::S | keys::D | keys::LEFT_SHIFT | keys::SPACE
        )
    }
}

impl Default for AutoWalk {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoWalk {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        if self.mode.is("Simple") {
            self.unpress();
        }
        // PORT GAP: Smart mode stops `PathManagers` (Baritone) in Java; the port
        // has no path manager.
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.mode.is("Simple") {
            let Some(player) = env.player() else { return };
            if self.disable_on_y.get() && player.prev_pos.y != player.pos.y {
                env.request_toggle("auto-walk");
                return;
            }

            // PORT GAP: `mc.options.keyUp.setDown(true)` - the `Env` cannot hold
            // a movement key down, so the chosen direction is recorded and the
            // engine side of the hook (`movement_input`) is what would consume it.
            let _ = self.wait_for_chunks.get();
            self.pressed = Some(self.direction.index());
        } else {
            // `PathManagers.get() instanceof NopPathManager`.
            self.base.info(env, "Smart mode requires Baritone");
            env.request_toggle("auto-walk");
        }
    }

    /// `onKey`: walking off manually disables the module.
    fn on_key(&mut self, env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        if !is_key || !pressed || !Self::is_movement_key(value) || !self.disable_on_input.get() {
            return false;
        }
        if env.screen() != Screen::Playing && env.module_active("gui-move") {
            return false;
        }
        env.request_toggle("auto-walk");
        false
    }

    fn info_string(&self) -> Option<String> {
        Some(match self.pressed {
            Some(0) => "Forwards".to_string(),
            Some(1) => "Backwards".to_string(),
            Some(2) => "Left".to_string(),
            Some(3) => "Right".to_string(),
            _ => self.mode.get().to_string(),
        })
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoWalk::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn smart_mode_reports_the_missing_path_manager() {
        let mut m = AutoWalk::new();
        let mut env = TestEnv::in_world();

        m.on_tick(&mut env);
        assert!(
            env.chats
                .iter()
                .any(|c| c.contains("Smart mode requires Baritone")),
            "chats were {:?}",
            env.chats
        );
    }

    #[test]
    fn simple_mode_holds_the_chosen_direction() {
        let mut m = AutoWalk::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(0); // Simple
        m.direction.set_index(2); // Left

        m.on_tick(&mut env);
        assert_eq!(m.pressed, Some(2));
        assert_eq!(m.info_string().as_deref(), Some("Left"));
        assert!(env.chats.is_empty(), "simple mode does not chat");
    }

    #[test]
    fn y_change_disables_in_simple_mode() {
        let mut m = AutoWalk::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(0);
        m.disable_on_y.set(true);
        env.player.as_mut().unwrap().pos.y = 64.5;

        m.on_tick(&mut env);
        assert_eq!(m.pressed, None);
    }
}

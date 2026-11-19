//! `systems.modules.Module` - the base every module carries.
//!
//! Java's `Module` is an abstract class with an event bus subscription and a
//! `Settings` bag. Here the state lives in [`ModuleBase`] and the behaviour in
//! the [`Module`] trait: a module struct holds `base: ModuleBase` plus whatever
//! fields its logic needs, and implements the hooks it cares about.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo, Renderer};
use crate::meteor_core::math::Rotation;
use crate::meteor_core::setting::Settings;
use crate::meteor_core::util::{name_to_title, ChatMsg, Keybind, Prefix};

/// `Module`'s non-behavioural state.
pub struct ModuleBase {
    /// `Module.name` - the id commands and config files use.
    pub name: &'static str,
    /// `Module.title`.
    pub title: String,
    /// `Module.description`.
    pub description: &'static str,
    /// `Module.aliases` - extra names the `toggle`/`bind` commands accept.
    pub aliases: &'static [&'static str],
    pub category: Category,
    pub settings: Settings,
    /// `Module.keybind`.
    pub keybind: Keybind,
    /// `toggleOnBindRelease`.
    pub toggle_on_bind_release: bool,
    /// `chatFeedback`.
    pub chat_feedback: bool,
    /// `favorite`, shown with a star in the GUI.
    pub favorite: bool,
    /// `runInMainMenu`.
    pub run_in_main_menu: bool,
    /// `serialize`.
    pub serialize: bool,
    /// `autoSubscribe` - modules that only want manual ticks set this false.
    pub auto_subscribe: bool,
    /// The accent colour the GUI paints this module's bar with.
    pub color: crate::meteor_core::color::Color,
    /// The last info string shown next to the module in the GUI/HUD.
    pub info_cache: String,
}

impl ModuleBase {
    pub fn new(category: Category, name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            title: name_to_title(name),
            description,
            aliases: &[],
            category,
            settings: Settings::new(),
            keybind: Keybind::none(),
            toggle_on_bind_release: false,
            chat_feedback: true,
            favorite: false,
            run_in_main_menu: false,
            serialize: true,
            auto_subscribe: true,
            color: crate::meteor_core::util::module_color(name),
            info_cache: String::new(),
        }
    }

    pub fn with_aliases(mut self, aliases: &'static [&'static str]) -> Self {
        self.aliases = aliases;
        self
    }

    pub fn with_keybind(mut self, keybind: Keybind) -> Self {
        self.keybind = keybind;
        self
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    /// `Module.info(...)` - a line in chat with this module's prefix.
    pub fn info(&self, env: &mut dyn Env, msg: &str) {
        let chat = ChatMsg::new(Prefix::Module(self.title.clone()), msg, &[]);
        env.chat(&chat.text);
    }

    pub fn warning(&self, env: &mut dyn Env, msg: &str) {
        let mut chat = ChatMsg::new(Prefix::Module(self.title.clone()), "(warning)", &[]);
        chat.text.push_str(msg);
        env.chat(&chat.text);
    }

    pub fn error(&self, env: &mut dyn Env, msg: &str) {
        let mut chat = ChatMsg::new(Prefix::Module(self.title.clone()), "(error)", &[]);
        chat.text.push_str(msg);
        env.chat(&chat.text);
    }
}

/// What every module can be asked to do. Default impls keep a module that only
/// wants `on_tick` from writing seven empty methods.
pub trait Module: Send + Sync {
    fn base(&self) -> &ModuleBase;
    fn base_mut(&mut self) -> &mut ModuleBase;

    // ---- lifecycle (Java: toggle() -> onActivate/onDeactivate) ----
    fn on_activate(&mut self, _env: &mut dyn Env) {}
    fn on_deactivate(&mut self, _env: &mut dyn Env) {}
    /// The client joined a world (`GameJoinedEvent`).
    fn on_game_joined(&mut self, _env: &mut dyn Env) {}
    fn on_game_left(&mut self, _env: &mut dyn Env) {}

    // ---- per-tick (ClientTickEvent / TickEvent.Pre / .Post in Java) ----
    /// Client tick, 20/s.
    fn on_tick(&mut self, _env: &mut dyn Env) {}
    /// The player's own tick, after movement input was read.
    fn on_player_tick(&mut self, _env: &mut dyn Env) {}

    // ---- render ----
    /// World-space drawing, once a frame (`Render3DEvent`).
    fn on_render_world(&mut self, _env: &mut dyn Env, _r: &mut dyn Renderer) {}
    /// Screen-space drawing, once a frame (`Render2DEvent`).
    fn on_render_2d(&mut self, _env: &mut dyn Env, _r: &mut dyn Renderer) {}
    /// Before the HUD draws (`Render2DEvent` reads the same pass in Meteor; the
    /// split keeps `NoRender` and `Hud` able to veto).
    fn on_render_hud(&mut self, _env: &mut dyn Env, _r: &mut dyn Renderer) {}

    // ---- packets ----
    /// A decoded play packet. Returning `false` cancels it, as a Meteor module
    /// that sets `event.cancel()` does.
    fn on_receive_packet(&mut self, _env: &mut dyn Env, _packet: &mut PacketInfo) -> bool {
        true
    }
    fn on_send_packet(&mut self, _env: &mut dyn Env, _packet: &mut PacketInfo) -> bool {
        true
    }

    // ---- input ----
    /// A key or mouse event. Returning `true` consumes it.
    fn on_key(&mut self, _env: &mut dyn Env, _is_key: bool, _value: i32, _pressed: bool) -> bool {
        false
    }

    // ---- misc ----
    /// The text the GUI and `ActiveModulesHud` show next to the title.
    fn info_string(&self) -> Option<String> {
        None
    }

    /// `Module.getWidget` - extra settings-panel widgets. The port's GUI draws
    /// a text area instead, so a module hands over lines to show.
    fn extra_widget(&self) -> Option<Vec<String>> {
        None
    }

    /// The rotation the module wants this tick, when it aims (Kill Aura et al.).
    fn wanted_rotation(&self) -> Option<Rotation> {
        None
    }
}

/// Reads `setup`, `tick`-style settings without the `base().settings` walk.
#[macro_export]
macro_rules! setting_get {
    ($m:expr, $kind:ident, $name:expr) => {
        $m.base().settings.get($name).and_then(|s| match &s.value {
            $crate::meteor_core::setting::Value::$kind(v) => Some(v),
            _ => None,
        })
    };
}

#[cfg(test)]
pub mod tests {
    //! A tiny module used by the manager's tests.

    use super::*;
    use crate::meteor_core::category::Category;
    use crate::meteor_core::env::Env;

    pub struct Counter {
        pub base: ModuleBase,
        pub ticks: u32,
        pub activated: u32,
        pub deactivated: u32,
    }

    impl Counter {
        pub fn new() -> Self {
            Self {
                base: ModuleBase::new(Category::Misc, "counter", "Counts ticks."),
                ticks: 0,
                activated: 0,
                deactivated: 0,
            }
        }
    }

    impl Module for Counter {
        fn base(&self) -> &ModuleBase {
            &self.base
        }
        fn base_mut(&mut self) -> &mut ModuleBase {
            &mut self.base
        }
        fn on_activate(&mut self, _env: &mut dyn Env) {
            self.activated += 1;
        }
        fn on_deactivate(&mut self, _env: &mut dyn Env) {
            self.deactivated += 1;
        }
        fn on_tick(&mut self, _env: &mut dyn Env) {
            self.ticks += 1;
        }
        fn info_string(&self) -> Option<String> {
            Some(self.ticks.to_string())
        }
    }
}

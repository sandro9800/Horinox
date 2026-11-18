//! `systems.modules.misc.DiscordPresence`.
//!
//! Java starts a Discord IPC socket, then keeps two "rich presence" lines
//! cycling through the configured message lists, evaluated with Starscript.
//! Horizon's mod API has no IPC, so the module is settings-only here: the same
//! settings, the same sequential/random selection and timer, the same main-menu
//! state machine, with the presence text kept in the module instead of sent over
//! the socket.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Screen};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{EnumSet, IntSet, ListSet};

pub struct DiscordPresence {
    base: ModuleBase,
    // Line 1
    line1_strings: ListSet,
    line1_update_delay: IntSet,
    line1_select_mode: EnumSet,
    // Line 2
    line2_strings: ListSet,
    line2_update_delay: IntSet,
    line2_select_mode: EnumSet,
    // Runtime
    /// The presence's first line (`RichPresence.setDetails`).
    line1: Option<String>,
    /// The presence's second line (`RichPresence.setState`).
    line2: Option<String>,
    ticks: u64,
    line1_ticks: i32,
    line1_i: usize,
    line2_ticks: i32,
    line2_i: usize,
    force_update: bool,
    last_was_in_main_menu: bool,
    small_image: SmallImage,
    rng: u64,
}

/// `DiscordPresence.SmallImage` - the two icons the module swaps between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmallImage {
    MineGame,
    Snail,
}

impl SmallImage {
    fn key(self) -> &'static str {
        match self {
            SmallImage::MineGame => "minegame",
            SmallImage::Snail => "seasnail",
        }
    }

    fn text(self) -> &'static str {
        match self {
            SmallImage::MineGame => "MineGame159",
            SmallImage::Snail => "seasnail8169",
        }
    }

    fn next(self) -> SmallImage {
        match self {
            SmallImage::MineGame => SmallImage::Snail,
            SmallImage::Snail => SmallImage::MineGame,
        }
    }
}

const SELECT_MODES: &[&str] = &["Random", "Sequential"];

impl DiscordPresence {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "discord-presence",
            "Displays Meteor as your presence on Discord.",
        );
        // `runInMainMenu = true`.
        base.run_in_main_menu = true;

        let mut s = base.settings_mut();
        s.create_group("Line 1");
        let line1_strings =
            super::last_group(&mut s).list_setting("line-1-messages", &["{player}", "{server}"]);
        s.describe_last("Messages used for the first line.");
        let line1_update_delay =
            super::last_group(&mut s).int_setting("line-1-update-delay", 200, 10, 200);
        s.describe_last("How fast to update the first line in ticks.");
        let line1_select_mode =
            super::last_group(&mut s).enum_setting("line-1-select-mode", SELECT_MODES, 1);
        s.describe_last("How to select messages for the first line.");

        s.create_group("Line 2");
        let line2_strings = super::last_group(&mut s).list_setting(
            "line-2-messages",
            &[
                "Meteor on Crack!",
                "{round(server.tps, 1)} TPS",
                "Playing on {server.difficulty} difficulty.",
                "{server.player_count} Players online",
            ],
        );
        s.describe_last("Messages used for the second line.");
        let line2_update_delay =
            super::last_group(&mut s).int_setting("line-2-update-delay", 60, 10, 200);
        s.describe_last("How fast to update the second line in ticks.");
        let line2_select_mode =
            super::last_group(&mut s).enum_setting("line-2-select-mode", SELECT_MODES, 1);
        s.describe_last("How to select messages for the second line.");

        Self {
            base,
            line1_strings,
            line1_update_delay,
            line1_select_mode,
            line2_strings,
            line2_update_delay,
            line2_select_mode,
            line1: None,
            line2: None,
            ticks: 0,
            line1_ticks: 0,
            line1_i: 0,
            line2_ticks: 0,
            line2_i: 0,
            force_update: true,
            last_was_in_main_menu: false,
            small_image: SmallImage::Snail,
            rng: 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn line1(&self) -> Option<&str> {
        self.line1.as_deref()
    }

    pub fn line2(&self) -> Option<&str> {
        self.line2.as_deref()
    }

    pub fn small_image(&self) -> SmallImage {
        self.small_image
    }

    /// The IPC payload the mod would hand Discord, `(large text, small key,
    /// small text, details, state)`.
    pub fn presence(&self) -> (String, &'static str, &'static str, String, String) {
        (
            format!("Meteor Client {}", crate::port::VERSION),
            self.small_image.key(),
            self.small_image.text(),
            self.line1.clone().unwrap_or_default(),
            self.line2.clone().unwrap_or_default(),
        )
    }

    fn next_u32(&mut self) -> u32 {
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.rng >> 33) as u32
    }

    /// `Utils.random(0, size)`, overridden by the sequential cursor when the
    /// list's select mode says so.
    fn pick(&mut self, len: usize, sequential: bool, cursor: usize) -> (usize, usize) {
        if sequential {
            let i = if cursor >= len { 0 } else { cursor };
            (i, i + 1)
        } else {
            ((self.next_u32() as usize) % len.max(1), cursor)
        }
    }

    /// `MeteorStarscript.run(script)` - the handful of placeholders the default
    /// messages use. `// PORT GAP:` there is no Starscript engine in the port, so
    /// anything else is left as written.
    pub fn evaluate(&self, template: &str, env: &dyn Env) -> String {
        let player = env.player();
        let world = env.world();
        let mut out = template.to_string();

        let player_name = player.as_ref().map(|p| p.name.clone()).unwrap_or_default();
        let server_name = world
            .as_ref()
            .map(|w| w.dimension.clone())
            .unwrap_or_default();
        let difficulty = world
            .as_ref()
            .map(|w| w.difficulty.clone())
            .unwrap_or_default();

        out = out.replace("{player.name}", &player_name);
        out = out.replace("{player}", &player_name);
        out = out.replace("{server.difficulty}", &difficulty);
        out = out.replace(
            "{round(server.tps, 1)}",
            &format!("{:.1}", env.recent_tps()),
        );
        out = out.replace("{server.tps}", &format!("{:.1}", env.recent_tps()));
        // `// PORT GAP:` `server.player_count` needs the tab list, which `Env`
        // does not expose.
        out = out.replace("{server.player_count}", "0");
        out = out.replace("{server}", &server_name);
        out
    }

    /// `DiscordPresence`'s main-menu state text.
    fn main_menu_state(screen: Screen) -> &'static str {
        // `// PORT GAP:` Java maps a dozen screen classes; `Env::Screen` is
        // coarse, so only the broad cases survive.
        match screen {
            Screen::Playing => "In game",
            Screen::Chat => "Chatting",
            Screen::Death => "Dead",
            Screen::Other => "Browsing Meteor's GUI",
        }
    }
}

impl Default for DiscordPresence {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for DiscordPresence {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        // `DiscordIPC.start(835240968533049424L, null)` - no IPC in Horizon.
        self.force_update = true;
        self.ticks = 0;
        self.line1_ticks = 0;
        self.line2_ticks = 0;
        self.line1_i = 0;
        self.line2_i = 0;
        self.last_was_in_main_menu = false;
        self.small_image = SmallImage::Snail;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        // `DiscordIPC.stop()`.
        self.line1 = None;
        self.line2 = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // Image: swaps every 200 ticks, or straight away when forced.
        if self.ticks >= 200 || self.force_update {
            self.small_image = self.small_image.next();
            self.ticks = 0;
        } else {
            self.ticks += 1;
        }

        if env.can_update() {
            // Line 1
            if self.line1_ticks as i64 >= self.line1_update_delay.get() as i64 || self.force_update
            {
                let messages = self.line1_strings.get();
                if !messages.is_empty() {
                    let sequential = self.line1_select_mode.is("Sequential");
                    let (i, cursor) = self.pick(messages.len(), sequential, self.line1_i);
                    self.line1_i = cursor;
                    let message = self.evaluate(&messages[i], env);
                    self.line1 = Some(message);
                }
                self.line1_ticks = 0;
            } else {
                self.line1_ticks += 1;
            }

            // Line 2
            if self.line2_ticks as i64 >= self.line2_update_delay.get() as i64 || self.force_update
            {
                let messages = self.line2_strings.get();
                if !messages.is_empty() {
                    let sequential = self.line2_select_mode.is("Sequential");
                    let (i, cursor) = self.pick(messages.len(), sequential, self.line2_i);
                    self.line2_i = cursor;
                    let message = self.evaluate(&messages[i], env);
                    self.line2 = Some(message);
                }
                self.line2_ticks = 0;
            } else {
                self.line2_ticks += 1;
            }
        } else if !self.last_was_in_main_menu {
            self.line1 = Some(format!("Meteor Client {}", crate::port::VERSION));
            self.line2 = Some(Self::main_menu_state(env.screen()).to_string());
        }

        self.force_update = false;
        self.last_was_in_main_menu = !env.can_update();
    }

    fn info_string(&self) -> Option<String> {
        // Java's module has no info string; keeping it out keeps the HUD honest.
        None
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(DiscordPresence::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn first_tick_evaluates_both_lines() {
        let mut m = DiscordPresence::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert_eq!(
            m.line1(),
            Some("Dev"),
            "{{player}} resolves to the player's name"
        );
        assert_eq!(m.line2(), Some("Meteor on Crack!"));

        let (large, small_key, small_text, details, state) = m.presence();
        assert_eq!(large, format!("Meteor Client {}", crate::port::VERSION));
        assert_eq!(
            small_key, "minegame",
            "the icon swapped on the forced update"
        );
        assert_eq!(small_text, "MineGame159");
        assert_eq!(details, "Dev".to_string());
        assert_eq!(state, "Meteor on Crack!".to_string());
        assert!(m.info_string().is_none());
    }

    #[test]
    fn sequential_selection_walks_the_list() {
        let mut m = DiscordPresence::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.line1_update_delay.set(10);
        m.line1_strings
            .set(vec!["a".to_string(), "b".to_string(), "c".to_string()]);

        // The first tick forces an update; afterwards the counter has to reach
        // the delay (10) before the line moves on, so each step is 11 ticks.
        m.on_tick(&mut env);
        assert_eq!(m.line1(), Some("a"));
        for _ in 0..11 {
            m.on_tick(&mut env);
        }
        assert_eq!(m.line1(), Some("b"));
        for _ in 0..11 {
            m.on_tick(&mut env);
        }
        assert_eq!(m.line1(), Some("c"));
        for _ in 0..11 {
            m.on_tick(&mut env);
        }
        assert_eq!(m.line1(), Some("a"), "the cursor wraps");
    }
}

//! `systems.modules.misc.AutoReconnect`.
//!
//! Java listens for `ServerConnectBeginEvent` to remember the address it last
//! connected to, then offers a countdown on the disconnect screen. The port
//! keeps the same state machine, but `Env` has no "join this server" call, so a
//! finished countdown leaves a *rejoin intent* the mod's hook can act on.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet};

pub struct AutoReconnect {
    base: ModuleBase,
    // General
    time: DoubleSet,
    button: BoolSet,
    // Runtime
    /// `lastServerConnection`: the address the next rejoin goes to.
    last_server: Option<String>,
    /// Whether a world was joined while the module was active.
    joined: bool,
    /// `env.now()` at which the countdown runs out.
    deadline: Option<f64>,
    /// The rejoin the mod still has to perform.
    intent: Option<String>,
    reconnects: u32,
}

impl AutoReconnect {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "auto-reconnect",
            "Automatically reconnects when disconnected from a server.",
        );

        let s = base.settings_mut();
        let time = s
            .default_group()
            .double_setting("delay", 3.5, 0.0, 3600.0, 1);
        s.describe_last("The amount of seconds to wait before reconnecting to the server.");
        let button = s.default_group().bool_setting("hide-buttons", false);
        s.describe_last("Will hide the buttons related to Auto Reconnect.");

        Self {
            base,
            time,
            button,
            last_server: None,
            joined: false,
            deadline: None,
            intent: None,
            reconnects: 0,
        }
    }

    /// `AutoReconnect.lastServerConnection` - `ServerConnectBeginEvent`'s address.
    /// The mod calls this when Horizon tells it which server it is joining.
    pub fn set_last_server(&mut self, address: &str) {
        self.last_server = Some(address.to_string());
    }

    pub fn last_server(&self) -> Option<&str> {
        self.last_server.as_deref()
    }

    /// `AutoReconnect.button` - whether the disconnect screen hides its buttons.
    pub fn hide_buttons(&self) -> bool {
        self.button.get()
    }

    /// Seconds left on the countdown, if one is running.
    pub fn countdown(&self, now: f64) -> Option<f64> {
        self.deadline.map(|d| (d - now).max(0.0))
    }

    /// Takes the pending rejoin, once the countdown has run out.
    pub fn take_rejoin_intent(&mut self) -> Option<String> {
        self.intent.take()
    }

    /// Starts the countdown Java shows after a disconnect.
    pub fn begin_countdown(&mut self, now: f64) {
        self.deadline = Some(now + self.time.get());
    }

    fn attempt_reconnect(&mut self, env: &mut dyn Env) {
        let target = self
            .last_server
            .clone()
            .unwrap_or_else(|| "the last server".to_string());
        self.base.info(env, &format!("Reconnecting to {target}..."));
        // `// PORT GAP:` `Env` has no "connect to this address" entry point -
        // Java hands `ServerAddress`/`ServerData` back to `ConnectScreen`. The
        // port records the intent so the mod's hook can perform the join.
        self.intent = Some(target);
        self.reconnects += 1;
        self.deadline = None;
    }
}

impl Default for AutoReconnect {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoReconnect {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.deadline = None;
        self.intent = None;
        self.reconnects = 0;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.deadline = None;
    }

    fn on_game_joined(&mut self, env: &mut dyn Env) {
        self.joined = true;
        self.deadline = None;
        self.intent = None;
        if self.last_server.is_none() {
            if let Some(player) = env.player() {
                // The address itself is not exposed; keep the world the join
                // landed in so the info string is not blank.
                self.last_server = Some(player.dimension.clone());
            }
        }
    }

    fn on_game_left(&mut self, env: &mut dyn Env) {
        if !self.joined {
            return;
        }
        self.joined = false;
        self.begin_countdown(env.now());
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(deadline) = self.deadline else {
            return;
        };
        if env.now() >= deadline {
            self.attempt_reconnect(env);
        }
    }

    fn info_string(&self) -> Option<String> {
        if let Some(target) = &self.intent {
            Some(target.clone())
        } else if self.deadline.is_some() {
            Some(format!("{:.1}s", self.time.get()))
        } else {
            None
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoReconnect::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn countdown_then_rejoin_intent() {
        let mut m = AutoReconnect::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.time.set(1.0);
        m.set_last_server("mc.example.net");

        m.on_game_joined(&mut env);
        env.now = 10.0;
        m.on_game_left(&mut env);
        assert!(m.deadline.is_some(), "leaving a world starts the countdown");

        // Half the delay has passed: nothing happens yet.
        env.now = 10.5;
        m.on_tick(&mut env);
        assert!(m.take_rejoin_intent().is_none());
        assert!(env.chats.is_empty());

        // The delay ran out: the intent is recorded and announced in chat.
        env.now = 11.1;
        m.on_tick(&mut env);
        assert_eq!(m.take_rejoin_intent().as_deref(), Some("mc.example.net"));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Reconnecting to mc.example.net")));
        assert_eq!(m.reconnects, 1);
        assert!(m.deadline.is_none(), "the countdown is consumed");
    }

    #[test]
    fn setting_names_match_java() {
        let m = AutoReconnect::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["delay", "hide-buttons"]);
        assert_eq!(m.time.get(), 3.5);
        assert!(!m.hide_buttons());
    }
}

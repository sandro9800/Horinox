//! `systems.modules.render.TimeChanger` - sets a custom world time.
//!
//! Ported from `render/TimeChanger.java`: it remembers the server's time, cancels
//! the time packets and pushes its own clock each tick.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::DoubleSet;

pub struct TimeChanger {
    base: ModuleBase,
    time: DoubleSet,
    /// `oldTime` - the time the server last reported.
    old_time: i64,
    /// The time the last tick asked for.
    wanted_time: i64,
}

impl TimeChanger {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "time-changer",
            "Makes you able to set a custom time.",
        );
        let s = base.settings_mut();
        let time = s
            .default_group()
            .double_setting("time", 0.0, -20000.0, 20000.0, 0);
        s.describe_last("The specified time to be set.");
        Self {
            base,
            time,
            old_time: 0,
            wanted_time: 0,
        }
    }

    pub fn old_time(&self) -> i64 {
        self.old_time
    }

    pub fn wanted_time(&self) -> i64 {
        self.wanted_time
    }

    /// `ClientboundSetTimePacket`'s closest port spelling.
    pub fn is_time_packet(name: &str) -> bool {
        matches!(
            name,
            "SetTime" | "UpdateTime" | "ClientboundSetTimePacket" | "set_time"
        )
    }
}

impl Default for TimeChanger {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for TimeChanger {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        // PORT GAP: no `Level.setDefaultClockTime`/clock write - the wanted time
        // only exists as module state and in the cancelled packet.
        self.old_time = env.world().map(|w| w.day_time).unwrap_or(0);
        self.wanted_time = self.time.get() as i64;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.wanted_time = self.old_time;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let _ = env;
        self.wanted_time = self.time.get() as i64;
    }

    fn on_receive_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !Self::is_time_packet(&packet.name) {
            return true;
        }
        // The server's time is remembered, then the packet is dropped so the
        // client keeps rendering our own clock.
        if let Some((_, value)) = packet
            .fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("time") || k.eq_ignore_ascii_case("gameTime"))
        {
            if let Ok(v) = value.parse::<i64>() {
                self.old_time = v;
            }
        }
        false
    }

    fn info_string(&self) -> Option<String> {
        Some(self.wanted_time.to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(TimeChanger::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_table() {
        let m = TimeChanger::new();
        assert!(m.base.settings.get("time").is_some());
        assert_eq!(m.base.settings.groups.len(), 1);
        assert_eq!(m.time.get(), 0.0);
    }

    #[test]
    fn the_wanted_time_follows_the_setting() {
        let mut m = TimeChanger::new();
        let mut env = TestEnv::in_world();
        if let Some(w) = env.world.as_mut() {
            w.day_time = 1200;
        }
        m.on_activate(&mut env);
        assert_eq!(m.old_time(), 1200);

        m.time.set(18000.0);
        m.on_tick(&mut env);
        assert_eq!(m.wanted_time(), 18000);
        assert_eq!(m.info_string().as_deref(), Some("18000"));

        m.on_deactivate(&mut env);
        assert_eq!(m.wanted_time(), 1200, "the old time is restored");
    }

    #[test]
    fn time_packets_are_cancelled() {
        let mut m = TimeChanger::new();
        let mut env = TestEnv::in_world();
        let mut packet = PacketInfo {
            name: "SetTime".into(),
            clientbound: true,
            fields: vec![("gameTime".into(), "900".into())],
        };
        assert!(!m.on_receive_packet(&mut env, &mut packet));
        assert_eq!(m.old_time(), 900, "the server time is recorded");

        let mut other = PacketInfo {
            name: "ChatMessage".into(),
            clientbound: true,
            fields: Vec::new(),
        };
        assert!(m.on_receive_packet(&mut env, &mut other));
    }
}

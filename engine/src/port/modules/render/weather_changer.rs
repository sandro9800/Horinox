//! `systems.modules.render.WeatherChanger` - overrides the world's weather.
//!
//! Ported from `render/WeatherChanger.java`: it remembers the server's rain and
//! thunder levels, swallows the game-event packets that change them and pushes the
//! configured levels every tick.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::DoubleSet;

pub struct WeatherChanger {
    base: ModuleBase,
    rain_level: DoubleSet,
    thunder_level: DoubleSet,
    /// `oldRainLevel` / `oldThunderLevel`.
    old_rain_level: f32,
    old_thunder_level: f32,
    /// The levels the last tick asked for.
    applied_rain: f32,
    applied_thunder: f32,
}

impl WeatherChanger {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "weather-changer",
            "Allows you to override the world's current weather.",
        );
        let s = base.settings_mut();
        let rain_level = s
            .default_group()
            .double_setting("rain-level", 0.0, 0.0, 1.0, 2);
        s.describe_last("The specified rain level to be set.");
        let thunder_level = s
            .default_group()
            .double_setting("thunder-level", 0.0, 0.0, 1.0, 2);
        s.describe_last("The specified thunder level to be set.");
        Self {
            base,
            rain_level,
            thunder_level,
            old_rain_level: 0.0,
            old_thunder_level: 0.0,
            applied_rain: 0.0,
            applied_thunder: 0.0,
        }
    }

    pub fn applied(&self) -> (f32, f32) {
        (self.applied_rain, self.applied_thunder)
    }

    pub fn old_levels(&self) -> (f32, f32) {
        (self.old_rain_level, self.old_thunder_level)
    }

    /// `ClientboundGameEventPacket` types this module cancels.
    pub fn is_weather_event(name: &str, fields: &[(String, String)]) -> bool {
        let packet = matches!(
            name,
            "GameEvent" | "ChangeGameState" | "ClientboundGameEventPacket" | "game_event"
        );
        if !packet {
            return false;
        }
        let kind = fields
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("event")
                    || k.eq_ignore_ascii_case("reason")
                    || k.eq_ignore_ascii_case("type")
            })
            .map(|(_, v)| v.to_ascii_lowercase())
            .unwrap_or_default();
        kind.contains("rain") || kind.contains("thunder") || kind.contains("weather")
    }
}

impl Default for WeatherChanger {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for WeatherChanger {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        if let Some(w) = env.world() {
            self.old_rain_level = w.rain_level;
            self.old_thunder_level = w.thunder_level;
        }
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        // PORT GAP: no `Level.setRainLevel`/`setThunderLevel` write, so the
        // remembered levels can only be handed back to the engine by the host.
        self.applied_rain = self.old_rain_level;
        self.applied_thunder = self.old_thunder_level;
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {
        self.applied_rain = self.rain_level.get_float();
        self.applied_thunder = self.thunder_level.get_float();
    }

    fn on_receive_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !Self::is_weather_event(&packet.name, &packet.fields) {
            return true;
        }
        let value = packet
            .fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("param") || k.eq_ignore_ascii_case("level"))
            .and_then(|(_, v)| v.parse::<f32>().ok())
            .unwrap_or(0.0);
        let kind = packet
            .fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("event") || k.eq_ignore_ascii_case("reason"))
            .map(|(_, v)| v.to_ascii_lowercase())
            .unwrap_or_default();
        if kind.contains("thunder") {
            self.old_thunder_level = value;
        } else {
            self.old_rain_level = value;
        }
        false
    }

    fn info_string(&self) -> Option<String> {
        Some(format!(
            "{:.1}/{:.1}",
            self.applied_rain, self.applied_thunder
        ))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(WeatherChanger::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_table() {
        let m = WeatherChanger::new();
        assert!(m.base.settings.get("rain-level").is_some());
        assert!(m.base.settings.get("thunder-level").is_some());
        assert_eq!(m.base.settings.groups.len(), 1);
        assert_eq!(m.rain_level.get(), 0.0);
    }

    #[test]
    fn the_levels_are_remembered_and_applied() {
        let mut m = WeatherChanger::new();
        let mut env = TestEnv::in_world();
        if let Some(w) = env.world.as_mut() {
            w.rain_level = 0.25;
            w.thunder_level = 0.5;
        }
        m.on_activate(&mut env);
        assert_eq!(m.old_levels(), (0.25, 0.5));

        m.rain_level.set(1.0);
        m.thunder_level.set(0.75);
        m.on_tick(&mut env);
        assert_eq!(m.applied(), (1.0, 0.75));

        m.on_deactivate(&mut env);
        assert_eq!(m.applied(), (0.25, 0.5));
    }

    #[test]
    fn weather_packets_are_cancelled_and_other_packets_pass() {
        let mut m = WeatherChanger::new();
        let mut env = TestEnv::in_world();
        let mut rain = PacketInfo {
            name: "GameEvent".into(),
            clientbound: true,
            fields: vec![
                ("event".into(), "RAIN_LEVEL_CHANGE".into()),
                ("param".into(), "1.0".into()),
            ],
        };
        assert!(!m.on_receive_packet(&mut env, &mut rain));
        assert_eq!(m.old_levels().0, 1.0);

        let mut chat = PacketInfo {
            name: "GameEvent".into(),
            clientbound: true,
            fields: vec![("event".into(), "CHANGE_GAMEMODE".into())],
        };
        assert!(m.on_receive_packet(&mut env, &mut chat));
    }
}

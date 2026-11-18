//! `systems.modules.render.Fullbright` - lights up your world.
//!
//! Ported from `render/Fullbright.java`. Java has three modes: `gamma` writes the
//! brightness option, `potion` keeps a night-vision effect alive and `luminance`
//! patches the light map. The port keeps the mode machinery and hands the host the
//! values it would need.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{EnumSet, IntSet};

/// `Fullbright.Mode`.
pub const MODES: &[&str] = &["gamma", "potion", "luminance"];
/// `LightLayer`.
pub const LIGHT_LAYERS: &[&str] = &["block", "sky"];

pub struct Fullbright {
    base: ModuleBase,
    mode: EnumSet,
    light_type: EnumSet,
    minimum_light_level: IntSet,
    /// Ticks the potion mode has kept night vision alive.
    potion_ticks: u64,
}

impl Fullbright {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Render, "fullbright", "Lights up your world!");
        let s = base.settings_mut();
        let mode = s.default_group().enum_setting("mode", MODES, 0);
        s.describe_last("The mode to use for Fullbright.");
        let light_type = s
            .default_group()
            .enum_setting("light-type", LIGHT_LAYERS, 0);
        s.describe_last("Which type of light to use for Luminance mode.");
        let m = mode.clone();
        s.set_visible_last(std::sync::Arc::new(move || m.is("luminance")));
        let minimum_light_level = s
            .default_group()
            .int_setting("minimum-light-level", 8, 0, 15);
        s.describe_last("Minimum light level when using Luminance mode.");
        let m = mode.clone();
        s.set_visible_last(std::sync::Arc::new(move || m.is("luminance")));

        Self {
            base,
            mode,
            light_type,
            minimum_light_level,
            potion_ticks: 0,
        }
    }

    /// `Fullbright.getLuminance(LightLayer)`.
    pub fn luminance(&self, layer: &str) -> i32 {
        if !self.mode.is("luminance") || !self.light_type.is(layer) {
            return 0;
        }
        self.minimum_light_level.get()
    }

    /// `Fullbright.getGamma()`.
    pub fn gamma(&self) -> bool {
        self.mode.is("gamma")
    }

    /// `Fullbright.Mode.Potion`'s bookkeeping: Java refreshes the effect every tick.
    pub fn potion_ticks(&self) -> u64 {
        self.potion_ticks
    }

    pub fn mode_name(&self) -> &'static str {
        self.mode.get()
    }

    /// `disableNightVision()` on deactivate; the port has no effect API.
    pub fn clear_potion(&mut self) {
        self.potion_ticks = 0;
    }
}

impl Default for Fullbright {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Fullbright {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        // PORT GAP: no gamma/light-map/night-vision effect access - `mode`,
        // `luminance()` and `gamma()` are the surface the host reads.
        self.potion_ticks = 0;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.clear_potion();
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {
        if self.mode.is("potion") {
            self.potion_ticks += 1;
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.mode.get().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Fullbright::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_table() {
        let m = Fullbright::new();
        for name in ["mode", "light-type", "minimum-light-level"] {
            assert!(
                m.base.settings.get(name).is_some(),
                "setting {name} missing"
            );
        }
        assert_eq!(m.base.settings.groups.len(), 1);
        assert!(m.mode.is("gamma"), "Java default is Mode.Gamma");
        assert_eq!(m.minimum_light_level.get(), 8);
    }

    #[test]
    fn the_mode_decides_what_the_host_gets() {
        let mut m = Fullbright::new();
        assert!(m.gamma());
        assert_eq!(m.luminance("block"), 0);

        m.mode.set_index(2); // Luminance
        assert!(!m.gamma());
        assert_eq!(m.luminance("block"), 8);
        assert_eq!(m.luminance("sky"), 0, "light-type is block by default");

        m.light_type.set_index(1); // Sky
        assert_eq!(m.luminance("sky"), 8);
        assert_eq!(m.info_string().as_deref(), Some("luminance"));
    }

    #[test]
    fn potion_mode_ticks_the_effect_counter() {
        let mut m = Fullbright::new();
        m.mode.set_index(1); // Potion
        let mut env = TestEnv::in_world();
        m.on_tick(&mut env);
        m.on_tick(&mut env);
        assert_eq!(m.potion_ticks(), 2);
        m.on_deactivate(&mut env);
        assert_eq!(m.potion_ticks(), 0);
    }
}

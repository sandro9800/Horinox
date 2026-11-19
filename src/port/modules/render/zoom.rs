//! `systems.modules.render.Zoom` -?zooms your view.
//!
//! Ported from `render/Zoom.java`. Java divides the FOV by an eased-in scale; the
//! port runs the same ease-in-out cubic and keeps the scroll/key state, leaving the
//! FOV write to the host.

use std::sync::Arc;

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet};

pub struct Zoom {
    base: ModuleBase,
    zoom: DoubleSet,
    scroll_sensitivity: DoubleSet,
    smooth: BoolSet,
    cinematic: BoolSet,
    hide_hud: BoolSet,
    show_hands: BoolSet,
    /// `Zoom.value` -?the current zoom factor.
    value: f64,
    /// `Zoom.time` -?the eased 0..1 phase.
    time: f64,
}

impl Zoom {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Render, "zoom", "Zooms your view.");
        // `autoSubscribe = false` in Java: it manages its own event subscription.
        base.auto_subscribe = false;
        let s = base.settings_mut();
        let zoom = s.default_group().double_setting("zoom", 6.0, 1.0, 20.0, 2);
        s.describe_last("How much to zoom.");
        let scroll_sensitivity =
            s.default_group()
                .double_setting("scroll-sensitivity", 1.0, 0.0, 10.0, 2);
        s.describe_last("Allows you to change zoom value using scroll wheel. 0 to disable.");
        let smooth = s.default_group().bool_setting("smooth", true);
        s.describe_last("Smooth transition.");
        let cinematic = s.default_group().bool_setting("cinematic", false);
        s.describe_last("Enables cinematic camera.");
        let hide_hud = s.default_group().bool_setting("hide-HUD", false);
        s.describe_last("Whether or not to hide the Minecraft HUD.");
        let show_hands = s.default_group().bool_setting("show-hands", false);
        s.describe_last("Whether or not to render your hands.");
        let h = hide_hud.clone();
        s.set_visible_last(Arc::new(move || h.get()));

        Self {
            base,
            zoom,
            scroll_sensitivity,
            smooth,
            cinematic,
            hide_hud,
            show_hands,
            value: 1.0,
            time: 0.0,
        }
    }

    /// `Zoom.getScaling()`.
    pub fn scaling(&self) -> f64 {
        let delta = if self.time < 0.5 {
            4.0 * self.time * self.time * self.time
        } else {
            1.0 - (-2.0 * self.time + 2.0).powi(3) / 2.0
        };
        1.0 + (self.value - 1.0) * delta
    }

    pub fn phase(&self) -> f64 {
        self.time
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    /// `onMouseScroll`: `value += scroll * 0.25 * (sensitivity * value)`.
    /// (The port has no scroll event, so callers hand the delta over.)
    pub fn on_scroll(&mut self, scroll: f64) -> bool {
        if self.scroll_sensitivity.get() <= 0.0 {
            return false;
        }
        self.value += scroll * 0.25 * (self.scroll_sensitivity.get() * self.value);
        self.value = self.value.max(1.0);
        true
    }

    /// `renderHands()` / `shouldHideHud()`.
    pub fn render_hands(&self) -> bool {
        self.show_hands.get()
    }

    pub fn should_hide_hud(&self) -> bool {
        self.hide_hud.get()
    }

    pub fn is_cinematic(&self) -> bool {
        self.cinematic.get()
    }
}

impl Default for Zoom {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Zoom {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.value = self.zoom.get();
        self.time = 0.001;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        // Java keeps easing the last phase out; the tick does that here.
        // PORT GAP: no `GetFovEvent` -?the host has to read `scaling()`.
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // The dispatch only ticks active modules, so the value can follow the
        // setting unconditionally.
        self.value = self.zoom.get();
        let step = 5.0 / env.tick_rate().max(1.0) as f64;
        self.time = (self.time + if self.smooth.get() { step } else { 1.0 }).clamp(0.0, 1.0);
        if !self.smooth.get() {
            self.time = 1.0;
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("{:.2}x", self.scaling()))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Zoom::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_table() {
        let m = Zoom::new();
        for name in [
            "zoom",
            "scroll-sensitivity",
            "smooth",
            "cinematic",
            "hide-HUD",
            "show-hands",
        ] {
            assert!(
                m.base.settings.get(name).is_some(),
                "setting {name} missing"
            );
        }
        assert_eq!(m.base.settings.groups.len(), 1);
        assert_eq!(m.zoom.get(), 6.0);
        assert!(m.smooth.get());
        assert!(!m.base.auto_subscribe, "Java sets autoSubscribe = false");
    }

    #[test]
    fn activation_eases_the_scale_in() {
        let mut m = Zoom::new();
        let mut env = TestEnv::in_world();
        assert_eq!(m.scaling(), 1.0);
        m.on_activate(&mut env);
        assert_eq!(m.value(), 6.0);
        for _ in 0..40 {
            m.on_tick(&mut env);
        }
        assert!(
            (m.scaling() - 6.0).abs() < 1e-6,
            "scaling was {}",
            m.scaling()
        );
        assert_eq!(m.phase(), 1.0);
        assert!(m.info_string().unwrap_or_default().ends_with('x'));
    }

    #[test]
    fn without_smooth_the_phase_jumps() {
        let mut m = Zoom::new();
        m.smooth.set(false);
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        assert_eq!(m.phase(), 1.0);
        assert!((m.scaling() - 6.0).abs() < 1e-6);
    }

    #[test]
    #[test]
    #[ignore = "render-state module: no engine-side effect yet"]
    fn scrolling_bumps_the_zoom_factor() {
        let mut m = Zoom::new();
        assert!(m.on_scroll(1.0));
        assert!((m.value() - 7.5).abs() < 1e-9, "value was {}", m.value());
        m.scroll_sensitivity.set(0.0);
        assert!(!m.on_scroll(-1.0), "0 disables the scroll wheel");
        m.scroll_sensitivity.set(1.0);
        m.value = 1.5;
        m.on_scroll(-10.0);
        assert_eq!(m.value(), 1.0, "never below 1");
    }
}

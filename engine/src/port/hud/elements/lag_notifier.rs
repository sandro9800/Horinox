//! `systems.hud.elements.LagNotifierHud` - "time since last tick".

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, Settings};

use super::common::{self, Sg};

pub struct LagNotifierElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub shadow: BoolSet,
    pub text_color: ColorSet,
    pub color1: ColorSet,
    pub color2: ColorSet,
    pub color3: ColorSet,
    pub custom_scale: BoolSet,
    pub scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
}

/// `LagNotifierHud.INFO`.
pub static LAG_NOTIFIER_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn lag_notifier_info() -> &'static HudElementInfo {
    LAG_NOTIFIER_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "lag-notifier",
            "Displays if the server is lagging in ticks.",
            || Box::new(LagNotifierElement::new()),
        )
    })
}

impl LagNotifierElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (
            shadow,
            text_color,
            color1,
            color2,
            color3,
            custom_scale,
            scale,
            background,
            background_color,
        ) = {
            let mut sg = Sg::new(&mut settings);
            let shadow = sg.bool_setting("General", "shadow", true, "Text shadow.");
            // Java's description is the placeholder "A.".
            let text_color = sg.color_setting(
                "General",
                "text-color",
                SettingColor::rgb(255, 255, 255),
                "A.",
            );
            let color1 = sg.color_setting(
                "General",
                "color-1",
                SettingColor::rgb(255, 255, 5),
                "First color.",
            );
            let color2 = sg.color_setting(
                "General",
                "color-2",
                SettingColor::rgb(235, 158, 52),
                "Second color.",
            );
            let color3 = sg.color_setting(
                "General",
                "color-3",
                SettingColor::rgb(225, 45, 45),
                "Third color.",
            );
            let custom_scale = sg.bool_setting(
                "Scale",
                "custom-scale",
                false,
                "Applies a custom scale to this hud element.",
            );
            let scale = sg.double_setting("Scale", "scale", 1.0, 0.5, 3.0, 2, "Custom scale.");
            let custom = custom_scale.clone();
            sg.visible_last(move || !custom.get());
            let background =
                sg.bool_setting("Background", "background", false, "Displays background.");
            let background_color = sg.color_setting(
                "Background",
                "background-color",
                SettingColor::new(25, 25, 25, 50),
                "Color used for the background.",
            );
            let on = background.clone();
            sg.visible_last(move || on.get());
            (
                shadow,
                text_color,
                color1,
                color2,
                color3,
                custom_scale,
                scale,
                background,
                background_color,
            )
        };

        Self {
            info: lag_notifier_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            shadow,
            text_color,
            color1,
            color2,
            color3,
            custom_scale,
            scale,
            background,
            background_color,
        }
    }

    fn get_scale(&self) -> f64 {
        common::scale_of(&self.custom_scale, &self.scale, common::hud_text_scale())
    }

    /// `TickRate.INSTANCE.getTimeSinceLastTick()`.
    // PORT GAP: Horizon reports the server's ticks per second rather than Meteor's
    // `TickRate` bookkeeping, so the seconds since the last tick are derived from
    // it (`1 / tps`); a hung server (`tps == 0`) reads as the largest value.
    pub fn time_since_last_tick(tps: f32) -> f64 {
        let tps = tps as f64;
        if tps <= 0.0 {
            return 60.0;
        }
        1.0 / tps
    }

    /// The colour the reading is drawn in: `color-3` past ten seconds, `color-2`
    /// past three, `color-1` otherwise.
    pub fn color_for(&self, since_last_tick: f64, in_editor: bool) -> u32 {
        if in_editor || since_last_tick > 10.0 {
            self.color3.color().argb()
        } else if since_last_tick > 3.0 {
            self.color2.color().argb()
        } else {
            self.color1.color().argb()
        }
    }

    /// The reading itself: `String.format("%.1f", ...)`, or `10.2` in the editor.
    pub fn reading(&self, since_last_tick: f64, in_editor: bool) -> String {
        if in_editor {
            "10.2".to_string()
        } else {
            format!("{since_last_tick:.1}")
        }
    }

    fn tick_impl(&mut self, _env: &mut dyn Env) {}

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = self.get_scale();
        let since_last_tick = Self::time_since_last_tick(env.recent_tps());
        let in_editor = common::is_in_editor(env);
        if since_last_tick < 1.1 && !in_editor {
            return;
        }

        let color = self.color_for(since_last_tick, in_editor);
        let info = self.reading(since_last_tick, in_editor);

        let left = "Time since last tick ";
        let left_color = self.text_color.color().argb();
        let x = r.text_width(left, scale);
        r.text2d(0.0, 0.0, left, left_color, scale);
        r.text2d(x, 0.0, &info, color, scale);

        let height = common::text_height(scale);
        let info_width = r.text_width(&info, scale);
        let total = x + info_width;
        common::background(
            r,
            total,
            height,
            self.background.get(),
            self.background_color.color().argb(),
        );
        self.set_size(total, height);
    }
}

impl Default for LagNotifierElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(LagNotifierElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn draws_both_halves_and_sizes_itself() {
        let mut element = LagNotifierElement::new();
        let mut env = TestEnv::in_world();
        env.player = None; // the editor always draws, with `10.2`
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(r.texts, 2, "the label and the reading");
        assert!(element.width() > 0);
        assert_eq!(element.height(), 9);
    }

    #[test]
    fn stays_quiet_while_the_server_ticks() {
        let mut element = LagNotifierElement::new();
        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        // `TestEnv::recent_tps()` is a full 20 tps: 50 ms since the last tick.
        assert_eq!(r.texts, 0, "50 ms since the last tick is not lag");
    }

    #[test]
    fn derives_the_delay_from_the_tps() {
        let element = LagNotifierElement::new();
        assert!((LagNotifierElement::time_since_last_tick(20.0) - 0.05).abs() < 1.0e-9);
        assert!((LagNotifierElement::time_since_last_tick(0.5) - 2.0).abs() < 1.0e-9);
        assert_eq!(LagNotifierElement::time_since_last_tick(0.0), 60.0);

        // `color-1` under three seconds, `color-2` past three, `color-3` past ten.
        assert_eq!(element.color_for(2.0, false), element.color1.color().argb());
        assert_eq!(element.color_for(4.0, false), element.color2.color().argb());
        assert_eq!(
            element.color_for(11.0, false),
            element.color3.color().argb()
        );
        assert_eq!(element.color_for(0.0, true), element.color3.color().argb());
        assert_eq!(element.reading(4.25, false), "4.2");
        assert_eq!(element.reading(0.0, true), "10.2");
    }
}

//! `systems.hud.elements.PotionTimersHud` - active status effects and their time
//! left.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{Alignment, HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{
    BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, ListSet, Settings,
};

use super::common::{self, Sg};

/// `PotionTimersHud.ColorMode`.
const COLOR_MODES: [&str; 3] = ["effect", "flat", "rainbow"];

/// What the element prints when there is nothing to list - Java's editor preview.
const PLACEHOLDER: &str = "Potion Timers 0:00";

pub struct PotionTimersElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub hidden_effects: ListSet,
    pub show_ambient: BoolSet,
    pub color_mode: EnumSet,
    pub flat_color: ColorSet,
    pub rainbow_speed: DoubleSet,
    pub rainbow_spread: DoubleSet,
    pub rainbow_saturation: DoubleSet,
    pub rainbow_brightness: DoubleSet,
    pub shadow: BoolSet,
    pub alignment: EnumSet,
    pub border: IntSet,
    pub custom_scale: BoolSet,
    pub scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
    /// `rainbowHue`, advanced every frame by `rainbow-speed`.
    pub rainbow_hue: f64,
}

/// `PotionTimersHud.INFO`.
pub static POTION_TIMERS_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn potion_timers_info() -> &'static HudElementInfo {
    POTION_TIMERS_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "potion-timers",
            "Displays active potion effects with timers.",
            || Box::new(PotionTimersElement::new()),
        )
    })
}

impl PotionTimersElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (
            hidden_effects,
            show_ambient,
            color_mode,
            flat_color,
            rainbow_speed,
            rainbow_spread,
            rainbow_saturation,
            rainbow_brightness,
            shadow,
            alignment,
            border,
            custom_scale,
            scale,
            background,
            background_color,
        ) = {
            let mut sg = Sg::new(&mut settings);
            // `StatusEffectListSetting` starts empty.
            let hidden_effects = sg.list_setting(
                "General",
                "hidden-effects",
                &[],
                "Which effects not to show in the list.",
            );
            let show_ambient = sg.bool_setting(
                "General",
                "show-ambient",
                true,
                "Whether to show ambient effects like from beacons and conduits.",
            );
            let color_mode = sg.enum_setting(
                "General",
                "color-mode",
                &COLOR_MODES,
                0,
                "What color to use for effects.",
            );
            let flat_color = sg.color_setting(
                "General",
                "flat-color",
                SettingColor::rgb(225, 25, 25),
                "Color for flat color mode.",
            );
            let mode = color_mode.clone();
            sg.visible_last(move || mode.is("flat"));
            let rainbow_speed = sg.double_setting(
                "General",
                "rainbow-speed",
                0.05,
                0.01,
                0.2,
                4,
                "Rainbow speed of rainbow color mode.",
            );
            let mode = color_mode.clone();
            sg.visible_last(move || mode.is("rainbow"));
            let rainbow_spread = sg.double_setting(
                "General",
                "rainbow-spread",
                0.01,
                0.001,
                0.05,
                4,
                "Rainbow spread of rainbow color mode.",
            );
            let mode = color_mode.clone();
            sg.visible_last(move || mode.is("rainbow"));
            let rainbow_saturation = sg.double_setting(
                "General",
                "rainbow-saturation",
                1.0,
                0.0,
                1.0,
                2,
                "Saturation of rainbow color mode.",
            );
            let mode = color_mode.clone();
            sg.visible_last(move || mode.is("rainbow"));
            let rainbow_brightness = sg.double_setting(
                "General",
                "rainbow-brightness",
                1.0,
                0.0,
                1.0,
                2,
                "Brightness of rainbow color mode.",
            );
            let mode = color_mode.clone();
            sg.visible_last(move || mode.is("rainbow"));
            let shadow = sg.bool_setting("General", "shadow", true, "Renders shadow behind text.");
            let alignment = sg.enum_setting(
                "General",
                "alignment",
                &Alignment::NAMES,
                0,
                "Horizontal alignment.",
            );
            let border = sg.int_setting(
                "General",
                "border",
                0,
                0,
                20,
                "How much space to add around the element.",
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
                hidden_effects,
                show_ambient,
                color_mode,
                flat_color,
                rainbow_speed,
                rainbow_spread,
                rainbow_saturation,
                rainbow_brightness,
                shadow,
                alignment,
                border,
                custom_scale,
                scale,
                background,
                background_color,
            )
        };

        Self {
            info: potion_timers_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            hidden_effects,
            show_ambient,
            color_mode,
            flat_color,
            rainbow_speed,
            rainbow_spread,
            rainbow_saturation,
            rainbow_brightness,
            shadow,
            alignment,
            border,
            custom_scale,
            scale,
            background,
            background_color,
            rainbow_hue: 0.0,
        }
    }

    fn get_scale(&self) -> f64 {
        common::scale_of(&self.custom_scale, &self.scale, common::hud_text_scale())
    }

    fn set_content_size(&mut self, width: f64, height: f64) {
        let border = self.border.get() as f64;
        self.box_
            .set_size(width + border * 2.0, height + border * 2.0);
    }

    fn align_content_x(&self, width: f64) -> f64 {
        let border = self.border.get() as f64;
        self.box_.align_x(
            self.width() as f64 - border * 2.0,
            width,
            common::alignment_of(&self.alignment),
        )
    }

    fn tick_impl(&mut self, env: &mut dyn Env) {
        // PORT GAP: `mc.player.getActiveEffects()` (the effect list with amplifier
        // and remaining duration) is not part of `Env`, so there are no rows to size
        // the element from. `render_impl` sizes it from the placeholder, which is also
        // what Java shows with no visible effects.
        let _ = (env, &self.hidden_effects, &self.show_ambient);
    }

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = self.get_scale();
        let border = self.border.get() as f64;

        // PORT GAP: no effect list in `Env`, so the placeholder stands in for the
        // rows; the rainbow phase still advances so the timer bookkeeping is live.
        let _ = env;
        self.rainbow_hue += self.rainbow_speed.get() * 0.016;
        if self.rainbow_hue > 1.0 {
            self.rainbow_hue -= 1.0;
        } else if self.rainbow_hue < -1.0 {
            self.rainbow_hue += 1.0;
        }

        let color = match self.color_mode.index() {
            1 => self.flat_color.color().argb(),
            2 => {
                let hue = self.rainbow_hue * 360.0;
                crate::meteor_core::color::Color::from_hsv(
                    hue,
                    self.rainbow_saturation.get(),
                    self.rainbow_brightness.get(),
                )
                .argb()
            }
            _ => 0xFFFF_FFFF,
        };

        let text_width = r.text_width(PLACEHOLDER, scale);
        let text_height = common::text_height(scale);
        self.set_content_size(text_width, text_height);

        let width = self.width() as f64;
        let height = self.height() as f64;
        common::background(
            r,
            width,
            height,
            self.background.get(),
            self.background_color.color().argb(),
        );

        r.text2d(
            border + self.align_content_x(text_width),
            border,
            PLACEHOLDER,
            color,
            scale,
        );
    }
}

impl Default for PotionTimersElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(PotionTimersElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn sizes_itself_and_prints_the_placeholder() {
        let mut element = PotionTimersElement::new();
        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(r.texts, 1);
        assert_eq!(element.width(), PLACEHOLDER.len() as i32 * 6);
        assert_eq!(element.height(), 9);
    }

    #[test]
    fn border_is_added_around_the_content() {
        let mut element = PotionTimersElement::new();
        element.border.set(4);
        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(element.height(), 9 + 8);
        assert_eq!(element.width(), PLACEHOLDER.len() as i32 * 6 + 8);
    }
}

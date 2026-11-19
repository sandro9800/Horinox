//! `systems.hud.elements.CompassHud` - the floating compass.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, Settings};

use super::common::{self, Sg};

/// `CompassHud.Mode`.
const MODES: [&str; 2] = ["direction", "axis"];

/// `CompassHud.Direction`: the four cardinal points in `Direction.values()` order,
/// each with the axis text `Mode.Axis` prints.
const DIRECTIONS: [(&str, &str); 4] = [("N", "Z-"), ("W", "X-"), ("S", "Z+"), ("E", "X+")];

pub struct CompassElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub mode: EnumSet,
    pub color_north: ColorSet,
    pub color_other: ColorSet,
    pub shadow: BoolSet,
    pub custom_scale: BoolSet,
    pub text_scale: DoubleSet,
    pub compass_scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
}

/// `CompassHud.INFO`.
pub static COMPASS_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn compass_info() -> &'static HudElementInfo {
    COMPASS_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "compass",
            "Displays a compass.",
            || Box::new(CompassElement::new()),
        )
    })
}

impl CompassElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (
            mode,
            color_north,
            color_other,
            shadow,
            custom_scale,
            text_scale,
            compass_scale,
            background,
            background_color,
        ) = {
            let mut sg = Sg::new(&mut settings);
            let mode = sg.enum_setting(
                "General",
                "type",
                &MODES,
                1,
                "Which type of direction information to show.",
            );
            let color_north = sg.color_setting(
                "General",
                "color-north",
                SettingColor::rgb(225, 45, 45),
                "Color of north.",
            );
            let color_other = sg.color_setting(
                "General",
                "color-other",
                SettingColor::rgb(255, 255, 255),
                "Color of other directions.",
            );
            let shadow = sg.bool_setting("General", "shadow", false, "Text shadow.");
            let custom_scale = sg.bool_setting(
                "Scale",
                "custom-scale",
                false,
                "Apply custom scales to this hud element.",
            );
            let text_scale = sg.double_setting(
                "Scale",
                "text-scale",
                1.0,
                0.5,
                3.0,
                2,
                "Scale to use for the letters.",
            );
            let custom = custom_scale.clone();
            sg.visible_last(move || custom.get());
            let compass_scale = sg.double_setting(
                "Scale",
                "compass-scale",
                1.0,
                0.5,
                3.0,
                2,
                "Scale of the whole HUD element.",
            );
            let custom = custom_scale.clone();
            sg.visible_last(move || custom.get());
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
                mode,
                color_north,
                color_other,
                shadow,
                custom_scale,
                text_scale,
                compass_scale,
                background,
                background_color,
            )
        };

        let mut box_ = HudBox::default();
        let scale = common::scale_of(&custom_scale, &compass_scale, common::hud_text_scale());
        common::size_box(&mut box_, 100.0 * scale, 100.0 * scale);

        Self {
            info: compass_info(),
            settings,
            box_,
            active: true,
            mode,
            color_north,
            color_other,
            shadow,
            custom_scale,
            text_scale,
            compass_scale,
            background,
            background_color,
        }
    }

    fn get_text_scale(&self) -> f64 {
        common::scale_of(
            &self.custom_scale,
            &self.text_scale,
            common::hud_text_scale(),
        )
    }

    fn get_compass_scale(&self) -> f64 {
        common::scale_of(
            &self.custom_scale,
            &self.compass_scale,
            common::hud_text_scale(),
        )
    }

    /// `CompassHud.getPos`: the yaw of one compass point, in radians.
    fn position(index: usize, yaw: f64) -> f64 {
        yaw + index as f64 * std::f64::consts::PI / 2.0
    }

    fn tick_impl(&mut self, _env: &mut dyn Env) {}

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = self.get_compass_scale();
        let text_scale = self.get_text_scale();
        let width = self.width() as f64;
        let height = self.height() as f64;

        common::background(
            r,
            width,
            height,
            self.background.get(),
            self.background_color.color().argb(),
        );

        let in_editor = common::is_in_editor(env);
        let player = env.player();

        let pitch = match &player {
            Some(_) if !in_editor => {
                let p = player.as_ref().map(|p| p.pitch as f64).unwrap_or(0.0);
                (p + 30.0).clamp(-90.0, 90.0)
            }
            _ => 120.0,
        };
        let pitch = pitch.to_radians();
        let yaw = match &player {
            Some(p) if !in_editor => common::wrap_degrees(p.yaw as f64),
            _ => 180.0,
        }
        .to_radians();

        let center_x = width / 2.0;
        let center_y = height / 2.0;
        let text_height = common::text_height(text_scale);
        let axis_mode = self.mode.is("axis");

        for (index, (name, axis)) in DIRECTIONS.iter().enumerate() {
            let text = if axis_mode { *axis } else { *name };
            let pos = Self::position(index, yaw);
            let x = center_x + pos.sin() * scale * 40.0;
            let y = center_y + pos.cos() * pitch.sin() * scale * 40.0;
            let color = if index == 0 {
                self.color_north.color().argb()
            } else {
                self.color_other.color().argb()
            };
            let w = r.text_width(text, text_scale);
            r.text2d(x - w / 2.0, y - text_height / 2.0, text, color, text_scale);
        }

        self.set_size(100.0 * scale, 100.0 * scale);
    }
}

impl Default for CompassElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(CompassElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn draws_the_four_cardinal_points() {
        let mut element = CompassElement::new();
        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(r.texts, 4, "N, W, S and E");
        assert_eq!(element.width(), 100);
        assert_eq!(element.height(), 100);
    }

    #[test]
    fn axis_mode_and_scales_follow_the_settings() {
        let mut element = CompassElement::new();
        assert!(element.mode.is("axis"));
        element.mode.set_index(0);
        assert!(element.mode.is("direction"));
        element.custom_scale.set(true);
        element.compass_scale.set(2.0);
        element.text_scale.set(3.0);
        assert_eq!(element.get_compass_scale(), 2.0);
        assert_eq!(element.get_text_scale(), 3.0);

        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(element.width(), 200);
    }
}

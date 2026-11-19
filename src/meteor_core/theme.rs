//! `gui.GuiTheme` + `gui.themes.meteor.MeteorGuiTheme` -?the theme's colours and
//! metrics, with the exact defaults the Meteor theme ships.
//!
//! The GUI draws with these, so reproducing them is what makes the port's ClickGUI
//! look like the original: the same accent purple, the same three-state
//! background/outline ladder, the same 6 px padding and 1.25x title scale.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::setting::{ColorSet, DoubleSet, EnumSet, Settings};

/// `GuiTheme.TITLE_TEXT_SCALE`.
pub const TITLE_TEXT_SCALE: f64 = 1.25;

/// A colour that changes with the widget's hover/press state
/// (`MeteorGuiTheme.ThreeStateColorSetting`).
#[derive(Clone)]
pub struct ThreeStateColor {
    pub normal: ColorSet,
    pub hovered: ColorSet,
    pub pressed: ColorSet,
}

impl ThreeStateColor {
    pub fn get(&self) -> crate::meteor_core::color::Color {
        self.normal.color()
    }

    pub fn get_state(&self, pressed: bool, hovered: bool) -> crate::meteor_core::color::Color {
        if pressed {
            self.pressed.color()
        } else if hovered {
            self.hovered.color()
        } else {
            self.normal.color()
        }
    }
}

/// The HORIXON theme and its settings, adapted from MeteorGuiTheme.
pub struct MeteorTheme {
    pub settings: Settings,

    // General
    pub scale: DoubleSet,
    pub module_alignment: EnumSet,
    pub category_icons: crate::meteor_core::setting::BoolSet,
    pub modules_help_text: crate::meteor_core::setting::BoolSet,
    pub hide_hud: crate::meteor_core::setting::BoolSet,

    // Colours
    pub accent_color: ColorSet,
    pub checkbox_color: ColorSet,
    pub plus_color: ColorSet,
    pub minus_color: ColorSet,
    pub favorite_color: ColorSet,

    // Text
    pub text_color: ColorSet,
    pub text_secondary_color: ColorSet,
    pub text_highlight_color: ColorSet,
    pub title_text_color: ColorSet,
    pub logged_in_color: ColorSet,
    pub placeholder_color: ColorSet,

    // Background / outline / scrollbar / slider
    pub background: ThreeStateColor,
    pub module_background: ColorSet,
    pub outline: ThreeStateColor,
    pub separator_text: ColorSet,
    pub separator_center: ColorSet,
    pub separator_edges: ColorSet,
    pub scrollbar: ThreeStateColor,
    pub slider_handle: ThreeStateColor,
    pub slider_left: ColorSet,
    pub slider_right: ColorSet,
}

impl Default for MeteorTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl MeteorTheme {
    pub fn new() -> Self {
        let mut settings = Settings::new();

        let (scale, module_alignment, category_icons, modules_help_text, hide_hud) = {
            let g = settings.default_group();
            let scale = g.double_setting("scale", 1.0, 0.75, 4.0, 2);
            let alignment =
                g.enum_setting("module-alignment", &["auto", "left", "center", "right"], 2);
            let icons = g.bool_setting("category-icons", false);
            let help = g.bool_setting("modules-help-text", true);
            let hide = g.bool_setting("hide-HUD", false);
            (scale, alignment, icons, help, hide)
        };

        let color_set = |settings: &mut Settings, group: &str, name: &str, c: SettingColor| {
            let g = if group == "General" {
                settings.default_group()
            } else {
                settings.create_group(group)
            };
            g.color_setting(name, c)
        };

        // `Colors`
        let accent = color_set(
            &mut settings,
            "Colors",
            "accent-color",
            SettingColor::rgb(34, 211, 238),
        );
        let checkbox = color_set(
            &mut settings,
            "Colors",
            "checkbox-color",
            SettingColor::rgb(34, 211, 238),
        );
        let plus = color_set(
            &mut settings,
            "Colors",
            "plus-color",
            SettingColor::rgb(50, 255, 50),
        );
        let minus = color_set(
            &mut settings,
            "Colors",
            "minus-color",
            SettingColor::rgb(255, 50, 50),
        );
        let favorite = color_set(
            &mut settings,
            "Colors",
            "favorite-color",
            SettingColor::rgb(250, 215, 0),
        );

        // `Text`
        let text = color_set(
            &mut settings,
            "Text",
            "text-color",
            SettingColor::rgb(255, 255, 255),
        );
        let text_secondary = color_set(
            &mut settings,
            "Text",
            "text-secondary-text-color",
            SettingColor::rgb(150, 150, 150),
        );
        let text_highlight = color_set(
            &mut settings,
            "Text",
            "text-highlight-color",
            SettingColor::new(34, 211, 238, 100),
        );
        let title_text = color_set(
            &mut settings,
            "Text",
            "title-text-color",
            SettingColor::rgb(255, 255, 255),
        );
        let logged_in = color_set(
            &mut settings,
            "Text",
            "logged-in-text-color",
            SettingColor::rgb(45, 225, 45),
        );
        let placeholder = color_set(
            &mut settings,
            "Text",
            "placeholder-color",
            SettingColor::new(255, 255, 255, 20),
        );

        // `Background`
        let background = {
            let g = settings.create_group("Background");
            ThreeStateColor {
                normal: g.color_setting("background-color", SettingColor::new(9, 20, 34, 238)),
                hovered: g.color_setting(
                    "hovered-background-color",
                    SettingColor::new(15, 34, 52, 242),
                ),
                pressed: g.color_setting(
                    "pressed-background-color",
                    SettingColor::new(22, 48, 68, 246),
                ),
            }
        };
        let module_background = color_set(
            &mut settings,
            "Background",
            "module-background-color",
            SettingColor::rgb(18, 34, 50),
        );

        // `Outline`
        let outline = {
            let g = settings.create_group("Outline");
            ThreeStateColor {
                normal: g.color_setting("outline-color", SettingColor::rgb(25, 42, 60)),
                hovered: g.color_setting("hovered-outline-color", SettingColor::rgb(34, 211, 238)),
                pressed: g.color_setting("pressed-outline-color", SettingColor::rgb(8, 145, 178)),
            }
        };

        // `Separator`
        let separator_text = color_set(
            &mut settings,
            "Separator",
            "separator-text-color",
            SettingColor::rgb(255, 255, 255),
        );
        let separator_center = color_set(
            &mut settings,
            "Separator",
            "separator-center-color",
            SettingColor::rgb(255, 255, 255),
        );
        let separator_edges = color_set(
            &mut settings,
            "Separator",
            "separator-edges-color",
            SettingColor::new(225, 225, 225, 150),
        );

        // `Scrollbar`
        let scrollbar = {
            let g = settings.create_group("Scrollbar");
            ThreeStateColor {
                normal: g.color_setting("scrollbar-color", SettingColor::new(15, 34, 52, 242)),
                hovered: g.color_setting(
                    "hovered-scrollbar-color",
                    SettingColor::new(22, 48, 68, 246),
                ),
                pressed: g.color_setting(
                    "pressed-scrollbar-color",
                    SettingColor::new(8, 145, 178, 246),
                ),
            }
        };

        // `Slider`
        let slider_handle = {
            let g = settings.create_group("Slider");
            ThreeStateColor {
                normal: g.color_setting("slider-handle-color", SettingColor::rgb(34, 211, 238)),
                hovered: g.color_setting(
                    "hovered-slider-handle-color",
                    SettingColor::rgb(103, 232, 249),
                ),
                pressed: g.color_setting(
                    "pressed-slider-handle-color",
                    SettingColor::rgb(8, 145, 178),
                ),
            }
        };
        let slider_left = color_set(
            &mut settings,
            "Slider",
            "slider-left-color",
            SettingColor::rgb(34, 211, 238),
        );
        let slider_right = color_set(
            &mut settings,
            "Slider",
            "slider-right-color",
            SettingColor::rgb(18, 34, 50),
        );

        Self {
            settings,
            scale,
            module_alignment,
            category_icons,
            modules_help_text,
            hide_hud,
            accent_color: accent,
            checkbox_color: checkbox,
            plus_color: plus,
            minus_color: minus,
            favorite_color: favorite,
            text_color: text,
            text_secondary_color: text_secondary,
            text_highlight_color: text_highlight,
            title_text_color: title_text,
            logged_in_color: logged_in,
            placeholder_color: placeholder,
            background,
            module_background,
            outline,
            separator_text,
            separator_center,
            separator_edges,
            scrollbar,
            slider_handle,
            slider_left,
            slider_right,
        }
    }

    /// `GuiTheme.scale(double)`.
    pub fn scale(&self, value: f64) -> f64 {
        value * self.scale.get()
    }

    /// `GuiTheme.pad()`.
    pub fn pad(&self) -> f64 {
        self.scale(6.0)
    }

    /// `GuiTheme.textHeight(bool title)`.
    pub fn text_height(&self) -> f64 {
        self.scale(9.0)
    }

    pub fn title_text_height(&self) -> f64 {
        self.scale(9.0 * TITLE_TEXT_SCALE)
    }

    /// `GuiTheme.textWidth`. The port's font is 6 px per character at 1x, the
    /// same advance Minecraft's default font gives a lowercase glyph (`WFont`
    /// widens it for capitals, which the GUI's layout does too).
    pub fn text_width(&self, text: &str) -> f64 {
        self.scale(text.chars().map(char_advance).sum::<f64>())
    }

    pub fn text_color(&self) -> crate::meteor_core::color::Color {
        self.text_color.color()
    }

    pub fn text_secondary(&self) -> crate::meteor_core::color::Color {
        self.text_secondary_color.color()
    }

    pub fn accent(&self) -> crate::meteor_core::color::Color {
        self.accent_color.color()
    }

    /// The background a widget draws, honouring hover/press.
    pub fn background_color(
        &self,
        pressed: bool,
        hovered: bool,
    ) -> crate::meteor_core::color::Color {
        self.background.get_state(pressed, hovered)
    }

    pub fn outline_color(&self, pressed: bool, hovered: bool) -> crate::meteor_core::color::Color {
        self.outline.get_state(pressed, hovered)
    }
}

/// The advance of one character in the default font, in GUI pixels at 1x.
fn char_advance(c: char) -> f64 {
    match c {
        'i' | 'l' | '.' | ',' | '!' | '\'' | '|' => 3.0,
        'I' | '[' | ']' | '(' | ')' | ' ' => 4.0,
        'f' | 'k' | 't' | 'r' | '*' => 5.0,
        'm' | 'w' | 'M' | 'W' => 7.0,
        c if c.is_ascii_uppercase() => 6.0,
        _ => 6.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_meteor_theme() {
        let t = MeteorTheme::new();
        assert_eq!(
            t.accent().argb(),
            crate::meteor_core::color::Color::rgb(145, 61, 226).argb()
        );
        assert_eq!(
            t.background_color(false, false).a,
            200,
            "background is semi-transparent"
        );
        assert_eq!(t.scale.get(), 1.0);
        assert_eq!(t.pad(), 6.0);
    }

    #[test]
    fn three_state_colours() {
        let t = MeteorTheme::new();
        let normal = t.background_color(false, false);
        let hovered = t.background_color(false, true);
        let pressed = t.background_color(true, false);
        assert_ne!(normal, hovered);
        assert_eq!(pressed.r, 40);
    }

    #[test]
    fn text_metrics_scale() {
        let mut t = MeteorTheme::new();
        let w1 = t.text_width("Kill Aura");
        t.scale.set(2.0);
        let w2 = t.text_width("Kill Aura");
        assert!((w2 - w1 * 2.0).abs() < 1.0e-9);
        assert_eq!(t.text_height(), 18.0);
    }
}

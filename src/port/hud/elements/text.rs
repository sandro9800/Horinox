//! `systems.hud.elements.TextHud` - arbitrary text, refreshed on a delay.
//!
//! `MeteorTextHud.INFO` (the `text` element, with its FPS/TPS/watermark presets)
//! is this class in Java.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{
    BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, Settings, StringSet,
};

use super::common::{self, Sg};

/// `TextHud.Shown`.
const SHOWN: [&str; 3] = ["always", "when-true", "when-false"];

/// `MeteorClient.NAME`, the default text.
const DEFAULT_TEXT: &str = "Meteor Client";

pub struct TextHudElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub text: StringSet,
    pub update_delay: IntSet,
    pub shadow: BoolSet,
    pub border: IntSet,
    pub shown: EnumSet,
    pub condition: StringSet,
    pub custom_scale: BoolSet,
    pub scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
    /// The text the last `run_tick` produced.
    pub section: String,
    /// `originalWidth`/`originalHeight`, before the border is added.
    original_width: f64,
    original_height: f64,
    timer: i32,
    visible: bool,
    first_tick: bool,
}

/// `MeteorTextHud.INFO` / `TextHud.INFO`.
pub static TEXT_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn text_info() -> &'static HudElementInfo {
    TEXT_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "text",
            "Displays arbitrary text with Starscript.",
            || Box::new(TextHudElement::new()),
        )
    })
}

impl TextHudElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (
            text,
            update_delay,
            shadow,
            border,
            shown,
            condition,
            custom_scale,
            scale,
            background,
            background_color,
        ) = {
            let mut sg = Sg::new(&mut settings);
            sg.group("Shown");
            let text = sg.string_setting(
                "General",
                "text",
                DEFAULT_TEXT,
                "Text to display with Starscript.",
            );
            let update_delay = sg.int_setting(
                "General",
                "update-delay",
                4,
                0,
                100,
                "Update delay in ticks",
            );
            let shadow = sg.bool_setting("General", "shadow", true, "Renders shadow behind text.");
            let border = sg.int_setting(
                "General",
                "border",
                0,
                0,
                20,
                "How much space to add around the text.",
            );
            let shown = sg.enum_setting(
                "Shown",
                "shown",
                &SHOWN,
                0,
                "When this text element is shown.",
            );
            let condition = sg.string_setting(
                "Shown",
                "condition",
                "",
                "Condition to check when shown is not Always.",
            );
            let s = shown.clone();
            sg.visible_last(move || !s.is("always"));
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
                text,
                update_delay,
                shadow,
                border,
                shown,
                condition,
                custom_scale,
                scale,
                background,
                background_color,
            )
        };

        Self {
            info: text_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            text,
            update_delay,
            shadow,
            border,
            shown,
            condition,
            custom_scale,
            scale,
            background,
            background_color,
            section: String::new(),
            original_width: 0.0,
            original_height: 0.0,
            timer: 0,
            visible: true,
            first_tick: true,
        }
    }

    fn get_scale(&self) -> f64 {
        common::scale_of(&self.custom_scale, &self.scale, common::hud_text_scale())
    }

    /// `TextHud.setSize`: the border is added around the measured text.
    fn set_content_size(&mut self, width: f64, height: f64) {
        self.original_width = width;
        self.original_height = height;
        let border = self.border.get() as f64;
        self.box_
            .set_size(width + border * 2.0, height + border * 2.0);
    }

    /// `TextHud.calculateSize`: the measured section, or 100px when it came out
    /// empty (so the element stays grabbable in the editor).
    fn calculate_size(&mut self, r: &dyn Renderer) {
        let scale = self.get_scale();
        let width = if self.section.trim().is_empty() {
            0.0
        } else {
            r.text_width(&self.section, scale)
        };
        let height = common::text_height(scale);
        if width > 0.0 {
            self.set_content_size(width, height);
        } else {
            self.set_content_size(100.0, height);
        }
    }

    /// `TextHud.runTick`: evaluate `text` (and `condition`, when not `always`).
    fn run_tick(&mut self) {
        // PORT GAP: Meteor runs the text through Starscript (`{player.pos.x}` and
        // the `meteor.*` namespace). Horizon has no script engine, so the setting is
        // printed as written; `condition` is read as the literal `true`/`false`.
        self.section = self.text.get();

        if !self.shown.is("always") {
            let evaluated = self.condition.get();
            let truth = evaluated.trim().eq_ignore_ascii_case("true");
            self.visible = if self.shown.is("when-true") {
                truth
            } else {
                !truth
            };
        } else {
            self.visible = true;
        }

        self.first_tick = false;
    }

    fn tick_impl(&mut self, _env: &mut dyn Env) {
        if self.timer <= 0 {
            self.run_tick();
            self.timer = self.update_delay.get();
        } else {
            self.timer -= 1;
        }
    }

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if self.first_tick {
            self.run_tick();
        }
        self.calculate_size(r);

        let visible = self.shown.is("always") || self.visible;
        let empty = self.section.trim().is_empty();
        let width = self.width() as f64;
        let height = self.height() as f64;
        let border = self.border.get() as f64;
        let scale = self.get_scale();

        if (empty || !visible) && common::is_in_editor(env) {
            common::placeholder(r, width, height);
        }
        if empty || !visible {
            return;
        }

        // Java's custom-font text is flushed at `HudRenderer.end`, so the background
        // quad it draws afterwards still lands underneath the glyphs.
        if self.background.get() {
            r.rect(
                0.0,
                0.0,
                width,
                height,
                self.background_color.color().argb(),
            );
        }

        // PORT GAP: Java draws each Starscript section (`#1`, `#2`, ...) in the
        // matching `Hud.text-colors` entry; the port's `Hud` has no such list, so the
        // text is one white run.
        r.text2d(border, border, &self.section, 0xFFFF_FFFF, scale);
    }
}

impl Default for TextHudElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(TextHudElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn prints_the_setting_text_and_sizes_itself() {
        let mut element = TextHudElement::new();
        element.text.set("Hello");
        let mut env = TestEnv::in_world();
        element.tick(&mut env);
        assert_eq!(element.section, "Hello");

        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(r.texts, 1);
        assert_eq!(element.width(), 30, "5 characters at 6px");
        assert_eq!(element.height(), 9);
    }

    #[test]
    fn border_is_added_around_the_text() {
        let mut element = TextHudElement::new();
        element.text.set("abc");
        element.border.set(4);
        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(element.width(), 18 + 8);
        assert_eq!(element.height(), 9 + 8);
    }

    #[test]
    fn the_condition_gate_hides_the_text() {
        let mut element = TextHudElement::new();
        element.shown.set_index(1); // when-true
        element.condition.set("false");
        element.update_delay.set(0); // re-evaluate every tick
        let mut env = TestEnv::in_world();
        element.tick(&mut env);
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(r.texts, 0);

        element.condition.set("true");
        element.tick(&mut env);
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(r.texts, 1);
    }

    #[test]
    fn update_delay_throttles_the_evaluation() {
        let mut element = TextHudElement::new();
        element.update_delay.set(2);
        let mut env = TestEnv::in_world();
        element.tick(&mut env); // runs, timer = 2
        element.text.set("changed");
        element.tick(&mut env); // timer 2 -> 1, no run
        assert_eq!(element.section, DEFAULT_TEXT);
        element.tick(&mut env); // timer 1 -> 0
        element.tick(&mut env); // runs
        assert_eq!(element.section, "changed");
    }
}

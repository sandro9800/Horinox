//! `systems.hud.elements.PlayerModelHud` - the little player model.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, Settings};

use super::common::{self, Sg};

/// `PlayerModelHud.CenterOrientation`.
const CENTER_ORIENTATION: [&str; 2] = ["north", "south"];

pub struct PlayerModelElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub copy_yaw: BoolSet,
    pub custom_yaw: IntSet,
    pub copy_pitch: BoolSet,
    pub custom_pitch: IntSet,
    pub center_orientation: EnumSet,
    pub custom_scale: BoolSet,
    pub scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
}

/// `PlayerModelHud.INFO`.
pub static PLAYER_MODEL_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn player_model_info() -> &'static HudElementInfo {
    PLAYER_MODEL_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "player-model",
            "Displays a model of your player.",
            || Box::new(PlayerModelElement::new()),
        )
    })
}

impl PlayerModelElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (
            copy_yaw,
            custom_yaw,
            copy_pitch,
            custom_pitch,
            center_orientation,
            custom_scale,
            scale,
            background,
            background_color,
        ) = {
            let mut sg = Sg::new(&mut settings);
            let copy_yaw = sg.bool_setting(
                "General",
                "copy-yaw",
                true,
                "Makes the player model's yaw equal to yours.",
            );
            let custom_yaw = sg.int_setting(
                "General",
                "custom-yaw",
                0,
                -180,
                180,
                "Custom yaw for when copy yaw is off.",
            );
            let copy = copy_yaw.clone();
            sg.visible_last(move || !copy.get());
            let copy_pitch = sg.bool_setting(
                "General",
                "copy-pitch",
                true,
                "Makes the player model's pitch equal to yours.",
            );
            let custom_pitch = sg.int_setting(
                "General",
                "custom-pitch",
                0,
                -90,
                90,
                "Custom pitch for when copy pitch is off.",
            );
            let copy = copy_pitch.clone();
            sg.visible_last(move || !copy.get());
            let center_orientation = sg.enum_setting(
                "General",
                "center-orientation",
                &CENTER_ORIENTATION,
                1,
                "Which direction the player faces when the HUD model faces directly forward.",
            );
            let custom_scale = sg.bool_setting(
                "Scale",
                "custom-scale",
                false,
                "Applies a custom scale to this hud element.",
            );
            let scale = sg.double_setting("Scale", "scale", 2.0, 0.5, 3.0, 2, "Custom scale.");
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
                copy_yaw,
                custom_yaw,
                copy_pitch,
                custom_pitch,
                center_orientation,
                custom_scale,
                scale,
                background,
                background_color,
            )
        };

        let mut box_ = HudBox::default();
        let initial_scale = common::scale_of(&custom_scale, &scale, 2.0);
        common::size_box(&mut box_, 50.0 * initial_scale, 75.0 * initial_scale);

        Self {
            info: player_model_info(),
            settings,
            box_,
            active: true,
            copy_yaw,
            custom_yaw,
            copy_pitch,
            custom_pitch,
            center_orientation,
            custom_scale,
            scale,
            background,
            background_color,
        }
    }

    fn get_scale(&self) -> f64 {
        common::scale_of(&self.custom_scale, &self.scale, 2.0)
    }

    /// The yaw/pitch `Minecraft.getEntityRenderDispatcher()` is asked to draw with
    /// (Java negates both before handing them to `renderer.entity`).
    fn model_angles(&self, env: &dyn Env) -> (f64, f64) {
        let offset = if self.center_orientation.is("north") {
            180.0
        } else {
            0.0
        };
        let (yaw, pitch) = match env.player() {
            Some(p) => (p.yaw as f64, p.pitch as f64),
            None => (0.0, 0.0),
        };
        let yaw = if self.copy_yaw.get() {
            common::wrap_degrees(yaw + offset)
        } else {
            self.custom_yaw.get() as f64
        };
        let pitch = if self.copy_pitch.get() {
            pitch
        } else {
            self.custom_pitch.get() as f64
        };
        (-yaw, -pitch)
    }

    fn tick_impl(&mut self, _env: &mut dyn Env) {}

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = self.get_scale();
        let width = self.width() as f64;
        let height = self.height() as f64;

        // PORT GAP: `HudRenderer.entity` needs Minecraft's living-entity render
        // pipeline (a rendered `Player` state); `Renderer` has no entity draw, so the
        // model itself is not drawn. The element keeps Meteor's size and its
        // background/placeholder so the layout, the anchoring and the editor preview
        // still match.
        let _angles = self.model_angles(env);

        if self.background.get() {
            r.rect(
                0.0,
                0.0,
                width,
                height,
                self.background_color.color().argb(),
            );
        } else if env.player().is_none() {
            // `mc.player == null`: Java fills the box and crosses it out.
            r.rect(
                0.0,
                0.0,
                width,
                height,
                self.background_color.color().argb(),
            );
            common::placeholder(r, width, height);
        }

        self.set_size(50.0 * scale, 75.0 * scale);
    }
}

impl Default for PlayerModelElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(PlayerModelElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn keeps_meteor_size_and_follows_the_player_angles() {
        let mut element = PlayerModelElement::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().yaw = 90.0;
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);

        assert_eq!(element.width(), 100, "50 * scale 2");
        assert_eq!(element.height(), 150, "75 * scale 2");
        // `center-orientation` defaults to south, so yaw 90 comes back negated.
        assert_eq!(element.model_angles(&env), (-90.0, -0.0));

        element.copy_yaw.set(false);
        element.custom_yaw.set(45);
        element.center_orientation.set_index(0); // north
        assert_eq!(element.model_angles(&env).0, -45.0);
    }

    #[test]
    fn draws_the_placeholder_without_a_player() {
        let mut element = PlayerModelElement::new();
        let mut env = TestEnv::in_world();
        env.player = None;
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert!(element.width() > 0 && element.height() > 0);
    }
}

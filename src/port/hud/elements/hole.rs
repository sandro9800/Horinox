//! `systems.hud.elements.HoleHud` - the blocks that make the hole you stand in.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::math::Dir;
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, ListSet, Settings};

use super::common::{self, Sg};

/// `HoleHud.Facing`: the four sides around the player, as yaw offsets.
const FACINGS: [(&str, i32); 4] = [("left", -90), ("front", 0), ("right", 90), ("back", 180)];

pub struct HoleElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub safe: ListSet,
    pub custom_scale: BoolSet,
    pub scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
}

/// `HoleHud.INFO`.
pub static HOLE_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn hole_info() -> &'static HudElementInfo {
    HOLE_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "hole",
            "Displays information about the hole you are standing in.",
            || Box::new(HoleElement::new()),
        )
    })
}

impl HoleElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (safe, custom_scale, scale, background, background_color) = {
            let mut sg = Sg::new(&mut settings);
            let safe = sg.list_setting(
                "General",
                "safe-blocks",
                &[
                    "minecraft:obsidian",
                    "minecraft:bedrock",
                    "minecraft:crying_obsidian",
                    "minecraft:netherite_block",
                ],
                "Which blocks to consider safe.",
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
            (safe, custom_scale, scale, background, background_color)
        };

        let mut element = Self {
            info: hole_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            safe,
            custom_scale,
            scale,
            background,
            background_color,
        };
        element.calculate_size();
        element
    }

    fn get_scale(&self) -> f64 {
        common::scale_of(&self.custom_scale, &self.scale, 2.0)
    }

    fn calculate_size(&mut self) {
        let scale = self.get_scale();
        self.box_.set_size(16.0 * 3.0 * scale, 16.0 * 3.0 * scale);
    }

    /// `safe.get().contains(block)`.
    fn is_safe(&self, id: &str) -> bool {
        self.safe
            .get()
            .iter()
            .any(|safe| common::same_id(safe, id) || safe.eq_ignore_ascii_case(id))
    }

    /// `HoleHud.get(Facing)`: `Direction.fromYRot(wrapDegrees(yaw + offset))`, or
    /// `Direction.DOWN` in the editor (which always shows obsidian).
    fn facing(&self, yaw: f64, offset: i32) -> Dir {
        common::direction_from_yaw(common::wrap_degrees(yaw + offset as f64))
    }

    fn tick_impl(&mut self, _env: &mut dyn Env) {}

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = self.get_scale();
        self.calculate_size();
        let width = self.width() as f64;
        let height = self.height() as f64;
        let in_editor = common::is_in_editor(env);
        let yaw = env.player().map(|p| p.yaw as f64).unwrap_or(0.0);
        let feet = env
            .player()
            .map(|p| p.pos.to_block_pos())
            .unwrap_or(crate::meteor_core::math::BlockPos::ORIGIN);
        let unit = 16.0 * scale;

        common::background(
            r,
            width,
            height,
            self.background.get(),
            self.background_color.color().argb(),
        );

        // Left, Front, Right, Back - the cross Meteor draws around the player.
        let places = [
            (0.0, unit),
            (unit, 0.0),
            (2.0 * unit, unit),
            (unit, 2.0 * unit),
        ];

        for ((_, offset), (x, y)) in FACINGS.iter().zip(places.iter()) {
            let dir = if in_editor {
                Dir::Down
            } else {
                self.facing(yaw, *offset)
            };

            let id = if dir == Dir::Down {
                // The editor previews the floor as obsidian.
                "minecraft:obsidian".to_string()
            } else {
                let pos = feet.offset_dir(dir);
                match env.block(pos) {
                    Some(block) => block.id,
                    None => continue,
                }
            };

            if !self.is_safe(&id) {
                continue;
            }
            let stack = common::preview(&id);
            r.item2d(*x, *y, &stack, unit);

            // PORT GAP: `mc.level.destructionProgress()` (the block-breaking
            // overlay Meteor paints over the mined block) is not readable through
            // `Env`, so the mining progress squares are not drawn.
        }
    }
}

impl Default for HoleElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(HoleElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;
    use crate::meteor_core::math::BlockPos;

    #[test]
    fn facing_follows_the_yaw() {
        let element = HoleElement::new();
        // Yaw 0 looks south; front is south, left is east, right is west.
        assert_eq!(element.facing(0.0, 0), Dir::South);
        assert_eq!(element.facing(0.0, -90), Dir::East);
        assert_eq!(element.facing(0.0, 90), Dir::West);
        assert_eq!(element.facing(0.0, 180), Dir::North);
    }

    #[test]
    fn draws_the_safe_blocks_around_the_player() {
        let mut element = HoleElement::new();
        let mut env = TestEnv::in_world();
        // The player stands on (0, 64, 0); ring every side with obsidian.
        for (dx, dz) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
            env.push_block(BlockPos::new(dx, 64, dz), "minecraft:obsidian");
        }
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(element.width(), 96, "16 * 3 * scale 2");
        assert!(element.is_safe("minecraft:obsidian"));
        assert!(!element.is_safe("minecraft:stone"));
    }
}

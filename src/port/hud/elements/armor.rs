//! `systems.hud.elements.ArmorHud` - the four armour slots and their durability.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, ItemStackInfo, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, Settings};

use super::common::{self, Sg};

/// `ArmorHud.Orientation`.
const ORIENTATION: [&str; 2] = ["horizontal", "vertical"];
/// `ArmorHud.Durability`.
const DURABILITY: [&str; 4] = ["none", "bar", "total", "percentage"];

pub struct ArmorElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub orientation: EnumSet,
    pub flip_order: BoolSet,
    pub show_empty: BoolSet,
    pub durability: EnumSet,
    pub durability_color: ColorSet,
    pub durability_shadow: BoolSet,
    pub custom_scale: BoolSet,
    pub scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
}

/// `ArmorHud.INFO`.
pub static ARMOR_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn armor_info() -> &'static HudElementInfo {
    ARMOR_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "armor",
            "Displays your armor.",
            || Box::new(ArmorElement::new()),
        )
    })
}

impl ArmorElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (
            orientation,
            flip_order,
            show_empty,
            durability,
            durability_color,
            durability_shadow,
            custom_scale,
            scale,
            background,
            background_color,
        ) = {
            let mut sg = Sg::new(&mut settings);
            let orientation = sg.enum_setting(
                "General",
                "orientation",
                &ORIENTATION,
                0,
                "How to display armor.",
            );
            let flip_order = sg.bool_setting(
                "General",
                "flip-order",
                true,
                "Flips the order of armor items.",
            );
            let show_empty = sg.bool_setting(
                "General",
                "show-empty",
                false,
                "Renders barrier icons for empty slots.",
            );
            let durability = sg.enum_setting(
                "Durability",
                "durability",
                &DURABILITY,
                1,
                "How to display armor durability.",
            );
            let durability_color = sg.color_setting(
                "Durability",
                "durability-color",
                SettingColor::rgb(255, 255, 255),
                "Color of the text.",
            );
            let uses_text = durability.clone();
            sg.visible_last(move || {
                let d = uses_text.get();
                d == "total" || d == "percentage"
            });
            let durability_shadow =
                sg.bool_setting("Durability", "durability-shadow", true, "Text shadow.");
            let uses_text = durability.clone();
            sg.visible_last(move || {
                let d = uses_text.get();
                d == "total" || d == "percentage"
            });
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
                orientation,
                flip_order,
                show_empty,
                durability,
                durability_color,
                durability_shadow,
                custom_scale,
                scale,
                background,
                background_color,
            )
        };

        let mut element = Self {
            info: armor_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            orientation,
            flip_order,
            show_empty,
            durability,
            durability_color,
            durability_shadow,
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

    /// `ArmorHud.calculateSize`: four stacks plus the two-pixel overlaps.
    fn calculate_size(&mut self) {
        let scale = self.get_scale();
        if self.orientation.is("vertical") {
            self.box_
                .set_size(16.0 * scale, (16.0 * 4.0 + 2.0 * 4.0) * scale);
        } else {
            self.box_
                .set_size((16.0 * 4.0 + 2.0 * 4.0) * scale, 16.0 * scale);
        }
    }

    /// `ArmorHud.getItem`: the editor shows netherite, an empty slot optionally
    /// shows a barrier.
    fn stack_for(&self, env: &dyn Env, index: usize) -> Option<ItemStackInfo> {
        if common::is_in_editor(env) {
            return Some(common::preview(match index {
                3 => "minecraft:netherite_helmet",
                2 => "minecraft:netherite_chestplate",
                1 => "minecraft:netherite_leggings",
                _ => "minecraft:netherite_boots",
            }));
        }
        // `armor_stacks` is boots-first, which is `EquipmentSlot.getIndex()` order.
        let stack = common::armor_stacks(env)[index].clone();
        match stack {
            Some(s) => Some(s),
            None if self.show_empty.get() => Some(common::preview("minecraft:barrier")),
            None => None,
        }
    }

    fn tick_impl(&mut self, _env: &mut dyn Env) {}

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = self.get_scale();
        self.calculate_size();
        let width = self.width() as f64;
        let height = self.height() as f64;

        // `flip-order` on: helmet first, boots last.
        let order: [usize; 4] = if self.flip_order.get() {
            [3, 2, 1, 0]
        } else {
            [0, 1, 2, 3]
        };
        let stacks: Vec<Option<ItemStackInfo>> = order
            .iter()
            .map(|index| self.stack_for(env, *index))
            .collect();
        let empty_slots = stacks.iter().filter(|s| s.is_none()).count();

        if self.background.get() && empty_slots < 4 {
            r.rect(
                0.0,
                0.0,
                width,
                height,
                self.background_color.color().argb(),
            );
        }

        let vertical = self.orientation.is("vertical");
        let durability = self.durability.get();
        let text_height = common::text_height(1.0);
        let color = self.durability_color.color().argb();

        for (position, stack) in stacks.iter().enumerate() {
            let (x, y) = if vertical {
                (0.0, position as f64 * common::item_pitch(scale))
            } else {
                (position as f64 * common::item_pitch(scale), 0.0)
            };
            let Some(stack) = stack else { continue };

            r.item2d(x, y, stack, common::item_size(scale));

            let max = common::max_damage(&stack.id);
            let damage = stack.damage.unwrap_or(0);
            match (durability, max) {
                // `renderer.item(..., isDamageableItem() && durability == Bar)`: the
                // vanilla bar, drawn by hand because `item2d` has no overlay.
                ("bar", Some(max)) => common::durability_bar(r, x, y, scale, damage, max),
                ("total", Some(max)) => {
                    let message = (max - damage).to_string();
                    let message_width = r.text_width(&message, 1.0);
                    let (tx, ty) = if vertical {
                        (
                            8.0 * scale - message_width / 2.0,
                            position as f64 * common::item_pitch(scale) + common::item_pitch(scale)
                                - text_height,
                        )
                    } else {
                        (
                            position as f64 * common::item_pitch(scale) + 8.0 * scale
                                - message_width / 2.0,
                            height - text_height,
                        )
                    };
                    r.text2d(tx, ty, &message, color, 1.0);
                }
                ("percentage", Some(max)) => {
                    let message = common::durability_percent(damage, max).to_string();
                    let message_width = r.text_width(&message, 1.0);
                    let (tx, ty) = if vertical {
                        (
                            8.0 * scale - message_width / 2.0,
                            position as f64 * common::item_pitch(scale) + common::item_pitch(scale)
                                - text_height,
                        )
                    } else {
                        (
                            position as f64 * common::item_pitch(scale) + 8.0 * scale
                                - message_width / 2.0,
                            height - text_height,
                        )
                    };
                    r.text2d(tx, ty, &message, color, 1.0);
                }
                // `Durability.None`, or an item the max-damage table does not name.
                _ => {}
            }
        }
    }
}

impl Default for ArmorElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(ArmorElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, NullRenderer};

    fn armor_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        for (index, id, damage) in [
            (common::SLOT_HELMET, "minecraft:netherite_helmet", 20),
            (
                common::SLOT_CHESTPLATE,
                "minecraft:netherite_chestplate",
                100,
            ),
            (common::SLOT_LEGGINGS, "minecraft:netherite_leggings", 40),
            (common::SLOT_BOOTS, "minecraft:netherite_boots", 10),
        ] {
            env.inventory.push(InvSlot {
                index,
                stack: Some(ItemStackInfo {
                    id: id.into(),
                    count: 1,
                    damage: Some(damage),
                    ..Default::default()
                }),
            });
        }
        env
    }

    #[test]
    fn draws_four_armour_slots_and_sizes_itself() {
        let mut element = ArmorElement::new();
        let mut env = armor_env();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(element.width(), 144, "(16 * 4 + 2 * 4) * 2");
        assert_eq!(element.height(), 32);

        // `total` durability puts a number on every slot.
        element.durability.set_index(2);
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(r.texts, 4, "one durability readout per armour piece");
    }

    #[test]
    fn vertical_orientation_flips_the_size() {
        let mut element = ArmorElement::new();
        element.orientation.set_index(1);
        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(element.width(), 32);
        assert_eq!(element.height(), 144);
    }

    #[test]
    fn show_empty_draws_barriers_in_the_editor() {
        let mut element = ArmorElement::new();
        element.show_empty.set(true);
        let mut env = TestEnv::in_world();
        env.player = None; // the editor
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert!(element.width() > 0);
    }
}

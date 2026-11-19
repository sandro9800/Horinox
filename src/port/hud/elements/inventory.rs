//! `systems.hud.elements.InventoryHud` - the 3x9 inventory viewer.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, ItemStackInfo, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, Settings};

use super::common::{self, Sg};

/// `InventoryHud.Background`: the three draws plus the size each one needs.
const BACKGROUND: [&str; 4] = ["none", "texture", "outline", "flat"];

fn background_size(index: usize) -> (f64, f64) {
    match index {
        // Texture
        1 => (176.0, 67.0),
        // None, Outline, Flat
        _ => (162.0, 54.0),
    }
}

pub struct InventoryElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub containers: BoolSet,
    pub custom_scale: BoolSet,
    pub scale: DoubleSet,
    pub background: EnumSet,
    pub background_color: ColorSet,
}

/// `InventoryHud.INFO`.
pub static INVENTORY_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn inventory_info() -> &'static HudElementInfo {
    INVENTORY_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "inventory",
            "Displays your inventory.",
            || Box::new(InventoryElement::new()),
        )
    })
}

impl InventoryElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (containers, custom_scale, scale, background, background_color) = {
            let mut sg = Sg::new(&mut settings);
            let containers = sg.bool_setting(
                "General",
                "containers",
                false,
                "Shows the contents of a container when holding them.",
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
            let background = sg.enum_setting(
                "Background",
                "background",
                &BACKGROUND,
                1,
                "Background of inventory viewer.",
            );
            let background_color = sg.color_setting(
                "Background",
                "background-color",
                SettingColor::rgb(255, 255, 255),
                "Color used for the background.",
            );
            let bg = background.clone();
            sg.visible_last(move || !bg.is("none"));
            (
                containers,
                custom_scale,
                scale,
                background,
                background_color,
            )
        };

        let mut box_ = HudBox::default();
        let initial_scale = common::scale_of(&custom_scale, &scale, 2.0);
        let (w, h) = background_size(background.index());
        common::size_box(&mut box_, w * initial_scale, h * initial_scale);

        Self {
            info: inventory_info(),
            settings,
            box_,
            active: true,
            containers,
            custom_scale,
            scale,
            background,
            background_color,
        }
    }

    fn get_scale(&self) -> f64 {
        common::scale_of(&self.custom_scale, &self.scale, 2.0)
    }

    fn calculate_size(&mut self) {
        let scale = self.get_scale();
        let (w, h) = background_size(self.background.index());
        self.box_.set_size(w * scale, h * scale);
    }

    fn tick_impl(&mut self, _env: &mut dyn Env) {}

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = self.get_scale();
        self.calculate_size();
        let width = self.width() as f64;
        let height = self.height() as f64;
        let color = self.background_color.color().argb();
        let texture = self.background.is("texture");
        let in_editor = common::is_in_editor(env);

        // ---- background ----
        match self.background.index() {
            0 => {}
            // PORT GAP: the `container.png` / `container-transparent.png` textures
            // are not reachable through `Renderer`, so the textured background is a
            // flat fill in the same colour, and `Outline` strokes instead.
            1 => r.rect(0.0, 0.0, width, height, color),
            2 => r.rect_outline(0.0, 0.0, width, height, color, 1.0),
            _ => r.rect(0.0, 0.0, width, height, color),
        }

        // PORT GAP: `Utils.getItemsInContainerItem` (a shulker's/ender chest's
        // contents) is not in `Env`, so `containers` cannot substitute the grid's
        // stacks; the element keeps showing the player's own main inventory.
        let _ = self.containers.get();

        // ---- 3 rows of 9, `getInventory().getItem(index + 9)` ----
        for row in 0..3 {
            for i in 0..9 {
                let index = row * 9 + i;
                let stack = if in_editor {
                    preview_stack(index)
                } else {
                    common::slot_stack(env, index + 9)
                };
                let Some(stack) = stack else { continue };

                let (item_x, item_y) = if texture {
                    (
                        (8.0 + i as f64 * 18.0) * scale,
                        (7.0 + row as f64 * 18.0) * scale,
                    )
                } else {
                    (
                        (1.0 + i as f64 * 18.0) * scale,
                        (1.0 + row as f64 * 18.0) * scale,
                    )
                };
                r.item2d(item_x, item_y, &stack, common::item_size(scale));
            }
        }
    }
}

/// `InventoryHud.PREVIEW_ITEMS`, shown in the editor: an obsidian, an end crystal
/// and a golden apple, all at 64.
fn preview_stack(index: usize) -> Option<ItemStackInfo> {
    match index {
        0 => Some(common::stack_of("minecraft:obsidian", 64)),
        1 => Some(common::stack_of("minecraft:end_crystal", 64)),
        2 => Some(common::stack_of("minecraft:golden_apple", 64)),
        _ => None,
    }
}

impl Default for InventoryElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(InventoryElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, NullRenderer};

    #[test]
    fn sizes_itself_from_the_background_enum() {
        let mut element = InventoryElement::new();
        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        // `texture` (the default) is 176x67 at scale 2.
        assert_eq!(element.width(), 352);
        assert_eq!(element.height(), 134);

        element.background.set_index(0); // none -> 162x54
        element.render(&mut env, &mut r);
        assert_eq!(element.width(), 324);
        assert_eq!(element.height(), 108);
    }

    #[test]
    fn draws_the_main_inventory_rows() {
        let mut element = InventoryElement::new();
        let mut env = TestEnv::in_world();
        for i in 9..36 {
            env.inventory.push(InvSlot {
                index: i,
                stack: Some(common::stack_of("minecraft:stone", 64)),
            });
        }
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert!(element.width() > 0 && element.height() > 0);
    }
}

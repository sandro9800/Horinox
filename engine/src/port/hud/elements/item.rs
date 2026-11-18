//! `systems.hud.elements.ItemHud` - the item counter.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, Settings, StringSet};

use super::common::{self, Sg};

pub struct ItemElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub item: StringSet,
    pub none_mode: EnumSet,
    pub custom_scale: BoolSet,
    pub scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
}

/// `ItemHud.INFO`.
pub static ITEM_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn item_info() -> &'static HudElementInfo {
    ITEM_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "item",
            "Displays the item count.",
            || Box::new(ItemElement::new()),
        )
    })
}

impl ItemElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (item, none_mode, custom_scale, scale, background, background_color) = {
            let mut sg = Sg::new(&mut settings);
            // Java: `ItemSetting` with `Items.TOTEM_OF_UNDYING`; the port keeps the
            // id as text (the contract's item-setting mapping).
            let item = sg.string_setting(
                "General",
                "item",
                "minecraft:totem_of_undying",
                "Item to display",
            );
            let none_mode = sg.enum_setting(
                "General",
                "none-mode",
                &["hide-item", "hide-count", "show-count"],
                1,
                "How to render the item when you don't have the specified item in your inventory.",
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
                item,
                none_mode,
                custom_scale,
                scale,
                background,
                background_color,
            )
        };

        let mut box_ = HudBox::default();
        // `calculateSize()` in the constructor.
        let initial_scale = common::scale_of(&custom_scale, &scale, 2.0);
        common::size_box(&mut box_, 17.0 * initial_scale, 17.0 * initial_scale);

        Self {
            info: item_info(),
            settings,
            box_,
            active: true,
            item,
            none_mode,
            custom_scale,
            scale,
            background,
            background_color,
        }
    }

    fn get_scale(&self) -> f64 {
        common::scale_of(&self.custom_scale, &self.scale, 2.0)
    }

    fn tick_impl(&mut self, _env: &mut dyn Env) {}

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = self.get_scale();
        let width = self.width() as f64;
        let height = self.height() as f64;
        let id = self.item.get();
        let count = common::count_of(env, &id);
        let hide_item = self.none_mode.is("hide-item");

        // Java posts the item draw, so the background lands underneath it.
        common::background(
            r,
            width,
            height,
            self.background.get(),
            self.background_color.color().argb(),
        );

        if hide_item && count <= 0 {
            if common::is_in_editor(env) {
                common::placeholder(r, width, height);
            }
        } else {
            // `NoneMode.ShowCount` renders a ghost stack (`setCount(1)`) so the icon
            // is still visible with nothing in the inventory.
            let shown = common::stack_of(&id, count.max(1));
            r.item2d(0.0, 0.0, &shown, common::item_size(scale));

            // PORT GAP: `HudRenderer.item(..., overlay)` draws the vanilla count
            // overlay; `Renderer::item2d` has no overlay flag, so the count is text.
            // `hide-count` still shows a count above one, as Java's overlay does.
            if self.none_mode.is("show-count") || (!hide_item && count > 1) {
                let text = count.to_string();
                let w = r.text_width(&text, scale);
                r.text2d(
                    width - w - 1.0,
                    height - common::text_height(scale),
                    &text,
                    0xFFFF_FFFF,
                    scale,
                );
            }
        }

        self.set_size(17.0 * scale, 17.0 * scale);
    }
}

impl Default for ItemElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(ItemElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn draws_the_item_and_sizes_itself() {
        let mut element = ItemElement::new();
        let mut env = TestEnv::in_world();
        // A totem in the hotbar: two of them, so the count text is drawn too.
        env.inventory.push(crate::meteor_core::env::InvSlot {
            index: 0,
            stack: Some(common::stack_of("minecraft:totem_of_undying", 2)),
        });

        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);

        assert!(element.width() > 0 && element.height() > 0);
        assert_eq!(element.width(), 34, "17 * the default scale of 2");
        assert!(r.texts > 0, "the count overlay is text in the port");
    }

    #[test]
    fn hide_item_draws_only_the_editor_placeholder() {
        let mut element = ItemElement::new();
        element.none_mode.set_index(0);
        let mut env = TestEnv::in_world();
        env.player = None; // `Utils.canUpdate() == false` - the editor
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert!(element.width() > 0);
        assert_eq!(r.texts, 0);
    }
}

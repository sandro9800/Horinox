//! `systems.hud.elements.MapHud` - a held map, drawn on the HUD.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, Settings};

use super::common::{self, Sg};

/// `MapHud.Mode`.
const MODES: [&str; 3] = ["slot-index", "map-id", "simple"];

pub struct MapElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub mode: EnumSet,
    pub slot_index: IntSet,
    pub map_id: IntSet,
    pub scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
    /// The inventory slot the map was found in this tick, as `tick` discovers it.
    pub map_slot: Option<usize>,
}

/// `MapHud.INFO`.
pub static MAP_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn map_info() -> &'static HudElementInfo {
    MAP_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "map",
            "Displays the contents of a map on your Hud.",
            || Box::new(MapElement::new()),
        )
    })
}

impl MapElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (mode, slot_index, map_id, scale, background, background_color) = {
            let mut sg = Sg::new(&mut settings);
            let mode = sg.enum_setting(
                "General",
                "mode",
                &MODES,
                2,
                "How to determine which map to render.",
            );
            let slot_index = sg.int_setting(
                "General",
                "slot-index",
                0,
                0,
                40,
                "Which slot to grab the map from.",
            );
            let m = mode.clone();
            sg.visible_last(move || m.is("slot-index"));
            // `noSlider()`: the id is typed, not slid.
            let map_id = sg.int_setting(
                "General",
                "map-id",
                0,
                0,
                i32::MAX,
                "Which map id to render from. Must be in your inventory!",
            );
            let m = mode.clone();
            sg.visible_last(move || m.is("map-id"));
            let scale = sg.double_setting(
                "Visual",
                "scale",
                1.0,
                0.5,
                3.0,
                2,
                "How big to render the map.",
            );
            let background = sg.bool_setting("Visual", "background", false, "Displays background.");
            let background_color = sg.color_setting(
                "Visual",
                "background-color",
                SettingColor::new(25, 25, 25, 50),
                "Color used for the background.",
            );
            let on = background.clone();
            sg.visible_last(move || on.get());
            (
                mode,
                slot_index,
                map_id,
                scale,
                background,
                background_color,
            )
        };

        let mut box_ = HudBox::default();
        common::size_box(&mut box_, 128.0 * scale.get(), 128.0 * scale.get());

        Self {
            info: map_info(),
            settings,
            box_,
            active: true,
            mode,
            slot_index,
            map_id,
            scale,
            background,
            background_color,
            map_slot: None,
        }
    }

    fn tick_impl(&mut self, env: &mut dyn Env) {
        let scale = self.scale.get();
        self.set_size(128.0 * scale, 128.0 * scale);

        self.map_slot = None;
        if !env.can_update() {
            return;
        }
        let _ = self.map_id.get();

        match self.mode.index() {
            0 => {
                // `SlotIndex`
                let index = self.slot_index.get() as usize;
                if common::slot_stack(env, index).is_some() {
                    self.map_slot = Some(index);
                }
            }
            // `MapId` and `Simple` both look the map up by its `MAP_ID` component.
            // PORT GAP: `ItemStackInfo` carries no data components, so the map id
            // cannot be matched; both modes take the first filled map in the
            // inventory.
            _ => {
                self.map_slot = env
                    .inventory()
                    .into_iter()
                    .find(|slot| {
                        slot.stack
                            .as_ref()
                            .map(|s| common::strip_ns(&s.id).ends_with("filled_map"))
                            .unwrap_or(false)
                    })
                    .map(|slot| slot.index);
            }
        }
    }

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let width = self.width() as f64;
        let height = self.height() as f64;
        let background_color = self.background_color.color().argb();

        let Some(slot) = self.map_slot else {
            // No map: the editor shows an empty frame, the game shows nothing.
            if common::is_in_editor(env) {
                r.rect(0.0, 0.0, width, height, background_color);
                common::placeholder(r, width, height);
            }
            return;
        };

        if self.background.get() {
            r.rect(0.0, 0.0, width, height, background_color);
        }

        // PORT GAP: drawing the map itself needs Minecraft's `MapRenderState` /
        // `MapItemSavedData` (`mc.getMapRenderer().extractRenderState(...)`), which
        // `Env` does not expose; the map's own item icon stands in for the texture so
        // the element still shows which map is being rendered.
        if let Some(stack) = common::slot_stack(env, slot) {
            let scale = self.scale.get();
            r.item2d(0.0, 0.0, &stack, common::item_size(scale));
        }
    }
}

impl Default for MapElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(MapElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, NullRenderer};

    #[test]
    fn finds_the_held_map_and_sizes_itself() {
        let mut element = MapElement::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 3,
            stack: Some(common::stack_of("minecraft:filled_map", 1)),
        });

        element.tick(&mut env);
        assert_eq!(element.map_slot, Some(3));
        assert_eq!(element.width(), 128);
        assert_eq!(element.height(), 128);

        element.scale.set(2.0);
        element.tick(&mut env);
        assert_eq!(element.width(), 256);
    }

    #[test]
    fn slot_index_mode_reads_the_named_slot() {
        let mut element = MapElement::new();
        element.mode.set_index(0);
        element.slot_index.set(9);
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 9,
            stack: Some(common::stack_of("minecraft:filled_map", 1)),
        });
        element.tick(&mut env);
        assert_eq!(element.map_slot, Some(9));

        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert!(element.width() > 0);
    }

    #[test]
    fn empty_when_no_map_is_carried() {
        let mut element = MapElement::new();
        let mut env = TestEnv::in_world();
        element.tick(&mut env);
        assert_eq!(element.map_slot, None);
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert!(element.width() > 0);
    }
}

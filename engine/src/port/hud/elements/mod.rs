//! The HUD element registry (`Hud.init`'s `register(...)` calls), one file per
//! element.
//!
//! `register` registers the elements in the order `Hud.init()` does, and then adds
//! the default layout `Hud.resetToDefaultElements()` builds: Meteor's watermark in
//! the top-left corner, the active-module list in the top-right, and the port's
//! coordinates readout in the bottom-right.

use crate::meteor_core::hud::Hud;

pub mod armor;
pub mod combat;
pub mod common;
pub mod compass;
pub mod hole;
pub mod inventory;
pub mod item;
pub mod keyboard;
pub mod lag_notifier;
pub mod map;
pub mod module_infos;
pub mod player_model;
pub mod player_radar;
pub mod potion_timers;
pub mod text;

/// The trait boilerplate every element shares: `info`/`settings`/`box` accessors,
/// the active flag, and `tick`/`render` forwarded to the element's own
/// `tick_impl`/`render_impl` (which is where the ported Java lives).
macro_rules! hud_element {
    ($ty:ty) => {
        impl crate::meteor_core::hud::HudElement for $ty {
            fn info(&self) -> &'static crate::meteor_core::hud::HudElementInfo {
                self.info
            }

            fn settings(&self) -> &crate::meteor_core::setting::Settings {
                &self.settings
            }

            fn settings_mut(&mut self) -> &mut crate::meteor_core::setting::Settings {
                &mut self.settings
            }

            fn box_ref(&self) -> &crate::meteor_core::hud::HudBox {
                &self.box_
            }

            fn box_mut(&mut self) -> &mut crate::meteor_core::hud::HudBox {
                &mut self.box_
            }

            fn is_active(&self) -> bool {
                self.active
            }

            fn set_active(&mut self, active: bool) {
                self.active = active;
            }

            fn tick(&mut self, env: &mut dyn crate::meteor_core::env::Env) {
                self.tick_impl(env)
            }

            fn render(
                &mut self,
                env: &mut dyn crate::meteor_core::env::Env,
                r: &mut dyn crate::meteor_core::env::Renderer,
            ) {
                self.render_impl(env, r)
            }
        }
    };
}
pub(crate) use hud_element;

/// `Hud.init()`'s element list, in the order Java registers them, plus the
/// `TextHud` behind `MeteorTextHud.INFO` (this Meteor revision registers that info
/// through `MeteorTextHud`, so it is kept next to it).
pub fn register(hud: &mut Hud) {
    use super::{
        active_modules_info, meteor_text_info, ActiveModulesElement, CoordsElement, TextElement,
    };

    hud.register(meteor_text_info());
    hud.register(item::item_info());
    hud.register(inventory::inventory_info());
    hud.register(compass::compass_info());
    hud.register(armor::armor_info());
    hud.register(hole::hole_info());
    hud.register(player_model::player_model_info());
    hud.register(active_modules_info());
    hud.register(lag_notifier::lag_notifier_info());
    hud.register(player_radar::player_radar_info());
    hud.register(module_infos::module_infos_info());
    hud.register(potion_timers::potion_timers_info());
    hud.register(combat::combat_info());
    hud.register(map::map_info());
    hud.register(keyboard::keyboard_info());
    hud.register(text::text_info());
    // The port's own coordinate readout; `Hud.init` has no such element, so it comes
    // after every entry Java registers.
    hud.register(super::coords_info());

    // `Hud.resetToDefaultElements()`: the elements a fresh config starts with.
    let mut watermark = TextElement::meteor_text();
    watermark.box_.set_pos(4, 4);
    hud.add(Box::new(watermark));

    let mut active_modules = ActiveModulesElement::new();
    active_modules.box_.set_pos(-4, 4);
    active_modules.box_.x_anchor = crate::meteor_core::hud::XAnchor::Right;
    active_modules.box_.y_anchor = crate::meteor_core::hud::YAnchor::Top;
    hud.add(Box::new(active_modules));

    let mut coords = CoordsElement::new();
    coords.box_.set_pos(-4, -4);
    coords.box_.x_anchor = crate::meteor_core::hud::XAnchor::Right;
    coords.box_.y_anchor = crate::meteor_core::hud::YAnchor::Bottom;
    hud.add(Box::new(coords));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_every_element_in_hud_init_order() {
        let mut hud = Hud::new();
        register(&mut hud);
        let names: Vec<&str> = hud.registry.iter().map(|i| i.name).collect();
        assert_eq!(
            names,
            vec![
                "horixon-text",
                "item",
                "inventory",
                "compass",
                "armor",
                "hole",
                "player-model",
                "active-modules",
                "lag-notifier",
                "player-radar",
                "module-infos",
                "potion-timers",
                "combat",
                "map",
                "keyboard",
                "text",
                "coords",
            ]
        );
        // Every registered element instantiates and keeps its own settings bag.
        for info in hud.registry.clone() {
            let element = info.create();
            assert_eq!(element.info().name, info.name);
            assert!(
                !element.settings().is_empty(),
                "{} has no settings",
                info.name
            );
        }
        assert_eq!(hud.elements.len(), 3, "the default layout");
    }
}

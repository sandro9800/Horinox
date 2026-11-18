//! Render modules (`systems.modules.render`).
//!
//! One module per file, plus the two Java sub-packages that carry helper
//! classes: [`blockesp`] (`render/blockesp/BlockESP` and its chunks/groups) and
//! [`marker`] (`render/marker/Marker` and the shape classes it renders).
//!
//! [`register`] adds them in `Modules.initRender()`'s order, which is also the
//! order `Modules.java` lists in the `add(new ...)` block.

pub mod better_tab;
pub mod better_tooltips;
pub mod block_selection;
pub mod blockesp;
pub mod blur;
pub mod boss_stack;
pub(crate) mod box_draw;
pub mod breadcrumbs;
pub mod break_indicators;
pub mod camera_tweaks;
pub mod chams;
pub mod city_esp;
pub mod entity_owner;
pub mod esp;
pub mod free_look;
pub mod freecam;
pub mod fullbright;
pub mod hand_view;
pub mod hole_esp;
pub mod item_highlight;
pub mod item_physics;
pub mod light_overlay;
pub mod logout_spots;
pub mod marker;
pub mod nametags;
pub mod no_render;
pub mod pop_chams;
pub mod storage_esp;
pub mod time_changer;
pub mod tracers;
pub mod trail;
pub mod trajectories;
pub mod tunnel_esp;
pub mod void_esp;
pub mod wall_hack;
pub mod waypoints;
pub mod weather_changer;
pub mod xray;
pub mod zoom;

use crate::meteor_core::manager::Modules;

/// `Modules.initRender()`.
pub fn register(modules: &mut Modules) {
    better_tab::register(modules);
    better_tooltips::register(modules);
    blockesp::register(modules);
    block_selection::register(modules);
    blur::register(modules);
    boss_stack::register(modules);
    breadcrumbs::register(modules);
    break_indicators::register(modules);
    camera_tweaks::register(modules);
    chams::register(modules);
    city_esp::register(modules);
    entity_owner::register(modules);
    esp::register(modules);
    freecam::register(modules);
    free_look::register(modules);
    fullbright::register(modules);
    hand_view::register(modules);
    hole_esp::register(modules);
    item_physics::register(modules);
    item_highlight::register(modules);
    light_overlay::register(modules);
    logout_spots::register(modules);
    marker::register(modules);
    nametags::register(modules);
    no_render::register(modules);
    pop_chams::register(modules);
    storage_esp::register(modules);
    time_changer::register(modules);
    tracers::register(modules);
    trail::register(modules);
    trajectories::register(modules);
    tunnel_esp::register(modules);
    void_esp::register(modules);
    wall_hack::register(modules);
    waypoints::register(modules);
    weather_changer::register(modules);
    xray::register(modules);
    zoom::register(modules);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every render module `Modules.java` lists, in registration order.
    const ORDER: [&str; 38] = [
        "better-tab",
        "better-tooltips",
        "block-esp",
        "block-selection",
        "blur",
        "boss-stack",
        "breadcrumbs",
        "break-indicators",
        "camera-tweaks",
        "chams",
        "city-esp",
        "entity-owner",
        "esp",
        "freecam",
        "free-look",
        "fullbright",
        "hand-view",
        "hole-esp",
        "item-physics",
        "item-highlight",
        "light-overlay",
        "logout-spots",
        "marker",
        "nametags",
        "no-render",
        "pop-chams",
        "storage-esp",
        "time-changer",
        "tracers",
        "trail",
        "trajectories",
        "tunnel-esp",
        "void-esp",
        "wall-hack",
        "waypoints",
        "weather-changer",
        "xray",
        "zoom",
    ];

    #[test]
    fn registers_every_render_module_in_modules_java_order() {
        let mut m = Modules::new();
        register(&mut m);
        let names: Vec<&str> = m
            .group(crate::meteor_core::category::Category::Render)
            .iter()
            .map(|m| m.base().name)
            .collect::<Vec<_>>();
        assert_eq!(names.len(), ORDER.len(), "registered {names:?}");
        // `Modules::add` appends, and `register` never sorts, so the category's
        // index list is the registration order.
        assert_eq!(names, ORDER.to_vec());
    }

    #[test]
    fn every_module_keeps_its_java_description() {
        let mut m = Modules::new();
        register(&mut m);
        for name in ORDER {
            let base = m.base(name).unwrap_or_else(|| panic!("{name} missing"));
            assert!(!base.description.is_empty(), "{name} has no description");
            assert_eq!(base.title, crate::meteor_core::util::name_to_title(name));
        }
        assert_eq!(
            m.base("esp").map(|b| b.description),
            Some("Renders entities through walls.")
        );
        assert_eq!(
            m.base("weather-changer").map(|b| b.description),
            Some("Allows you to override the world's current weather.")
        );
    }
}

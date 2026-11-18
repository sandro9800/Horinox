use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub const ID: &str = "horixon";
pub const NAME: &str = "HORIXON";
pub const VERSION: &str = "0.3.0";
pub const DESCRIPTION: &str = "HORIXON native client and configurable render modules.";

pub const PANEL_BACKGROUND: u32 = 0xD9101420;
pub const PANEL_ACCENT: u32 = 0xFF38BDF8;
pub const PANEL_TEXT: u32 = 0xFFF1F5F9;
pub const PANEL_MUTED: u32 = 0xFF94A3B8;
pub const MENU_BACKGROUND: u32 = 0xF00B1020;
pub const MENU_SELECTED: u32 = 0xFF172B42;

const CATEGORIES: [&str; 6] = ["Combat", "Movement", "Render", "Player", "World", "Misc"];
const RENDER_MODULES: [(&str, &str); 3] = [
    ("array_list", "ArrayList"),
    ("coordinates", "Coordinates"),
    ("fps", "FPS"),
];
static MENU_OPEN: AtomicBool = AtomicBool::new(false);
static SELECTED_CATEGORY: AtomicUsize = AtomicUsize::new(0);
static SELECTED_MODULE: AtomicUsize = AtomicUsize::new(0);
static SETTINGS_LOADED: AtomicBool = AtomicBool::new(false);
static ARRAY_LIST_ENABLED: AtomicBool = AtomicBool::new(true);
static COORDINATES_ENABLED: AtomicBool = AtomicBool::new(true);
static FPS_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn position_line(x: f64, y: f64, z: f64, dimension: &str) -> String {
    format!("XYZ  {:.1}  {:.1}  {:.1}  |  {}", x, y, z, dimension)
}

pub fn status_line(fps: u32) -> String {
    format!("NATIVE CLIENT  •  {} FPS", fps)
}

pub fn load_settings(array_list: Option<String>, coordinates: Option<String>, fps: Option<String>) {
    if SETTINGS_LOADED.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return;
    }
    ARRAY_LIST_ENABLED.store(read_bool(array_list, true), Ordering::Relaxed);
    COORDINATES_ENABLED.store(read_bool(coordinates, true), Ordering::Relaxed);
    FPS_ENABLED.store(read_bool(fps, true), Ordering::Relaxed);
}

fn read_bool(value: Option<String>, default: bool) -> bool {
    value.map_or(default, |v| v.trim() != "false")
}

pub fn menu_is_open() -> bool {
    MENU_OPEN.load(Ordering::Relaxed)
}

pub fn selected_category() -> usize {
    SELECTED_CATEGORY.load(Ordering::Relaxed) % CATEGORIES.len()
}

pub fn selected_module() -> usize {
    SELECTED_MODULE.load(Ordering::Relaxed)
}

pub fn category_name(index: usize) -> &'static str {
    CATEGORIES[index % CATEGORIES.len()]
}

pub fn category_count() -> usize {
    CATEGORIES.len()
}

pub fn module_count(category: usize) -> usize {
    if category == 2 { RENDER_MODULES.len() } else { 0 }
}

pub fn module_name(category: usize, index: usize) -> &'static str {
    if category == 2 { RENDER_MODULES[index % RENDER_MODULES.len()].1 } else { "" }
}

pub fn module_id(category: usize, index: usize) -> &'static str {
    if category == 2 { RENDER_MODULES[index % RENDER_MODULES.len()].0 } else { "" }
}

pub fn module_enabled(category: usize, index: usize) -> bool {
    if category != 2 { return false; }
    match index {
        0 => ARRAY_LIST_ENABLED.load(Ordering::Relaxed),
        1 => COORDINATES_ENABLED.load(Ordering::Relaxed),
        2 => FPS_ENABLED.load(Ordering::Relaxed),
        _ => false,
    }
}

fn set_module_enabled(index: usize, enabled: bool) {
    match index {
        0 => ARRAY_LIST_ENABLED.store(enabled, Ordering::Relaxed),
        1 => COORDINATES_ENABLED.store(enabled, Ordering::Relaxed),
        2 => FPS_ENABLED.store(enabled, Ordering::Relaxed),
        _ => {}
    }
}

/// Toggle a registered module and return its new state.
pub fn toggle_module(category: usize, index: usize) -> Option<bool> {
    if index >= module_count(category) { return None; }
    let enabled = !module_enabled(category, index);
    set_module_enabled(index, enabled);
    Some(enabled)
}

pub fn coordinates_enabled() -> bool { COORDINATES_ENABLED.load(Ordering::Relaxed) }
pub fn fps_enabled() -> bool { FPS_ENABLED.load(Ordering::Relaxed) }
pub fn array_list_enabled() -> bool { ARRAY_LIST_ENABLED.load(Ordering::Relaxed) }

/// Returns the changed module id and new state so the caller can persist it.
pub fn handle_key(playing: bool, enabled: bool, key: &str, pressed: bool) -> Option<(&'static str, bool)> {
    if !enabled || !pressed { return None; }
    if key == "key.keyboard.escape" {
        MENU_OPEN.store(false, Ordering::Relaxed);
        return None;
    }
    if !playing { return None; }
    match key {
        "key.keyboard.right.shift" => {
            MENU_OPEN.fetch_xor(true, Ordering::Relaxed);
        }
        "key.keyboard.left" if menu_is_open() => {
            SELECTED_CATEGORY.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |i|
                Some((i + CATEGORIES.len() - 1) % CATEGORIES.len())).ok();
            SELECTED_MODULE.store(0, Ordering::Relaxed);
        }
        "key.keyboard.right" if menu_is_open() => {
            SELECTED_CATEGORY.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |i|
                Some((i + 1) % CATEGORIES.len())).ok();
            SELECTED_MODULE.store(0, Ordering::Relaxed);
        }
        "key.keyboard.up" if menu_is_open() => {
            let count = module_count(selected_category());
            if count > 0 {
                SELECTED_MODULE.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |i|
                    Some((i + count - 1) % count)).ok();
            }
        }
        "key.keyboard.down" if menu_is_open() => {
            let count = module_count(selected_category());
            if count > 0 {
                SELECTED_MODULE.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |i|
                    Some((i + 1) % count)).ok();
            }
        }
        "key.keyboard.space" | "key.keyboard.enter" if menu_is_open() => {
            if let Some(enabled) = toggle_module(selected_category(), selected_module()) {
                return Some((module_id(selected_category(), selected_module()), enabled));
            }
        }
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_line_keeps_negative_coordinates_and_dimension() {
        assert_eq!(position_line(-12.25, 64.0, 3.5, "minecraft:overworld"),
            "XYZ  -12.2  64.0  3.5  |  minecraft:overworld");
    }
    #[test]
    fn status_line_reports_frame_rate() {
        assert_eq!(status_line(144), "NATIVE CLIENT  •  144 FPS");
    }
    #[test]
    fn module_identity_and_categories_are_stable() {
        assert_eq!(ID, "horixon");
        assert_eq!(NAME, "HORIXON");
        assert_eq!(VERSION, "0.3.0");
        assert_eq!(category_count(), 6);
        assert_eq!(category_name(6), "Combat");
        assert_eq!(module_count(2), 3);
        assert_eq!(module_name(2, 0), "ArrayList");
        assert_eq!(module_id(2, 1), "coordinates");
        assert_eq!(module_count(0), 0);
    }
    #[test]
    fn settings_default_on_unless_explicitly_false() {
        assert!(read_bool(None, true));
        assert!(read_bool(Some("true".into()), false));
        assert!(!read_bool(Some("false\n".into()), true));
    }
    #[test]
    fn module_toggle_changes_only_a_valid_render_module() {
        let before = module_enabled(2, 0);
        assert_eq!(toggle_module(2, 0), Some(!before));
        assert_eq!(module_enabled(2, 0), !before);
        assert_eq!(toggle_module(0, 0), None);
        assert_eq!(module_enabled(2, 0), !before);
        set_module_enabled(0, before);
    }
}

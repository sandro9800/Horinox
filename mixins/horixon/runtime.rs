use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub const ID: &str = "horixon";
pub const NAME: &str = "HORIXON";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "HORIXON native client foundation and HUD.";

pub const PANEL_BACKGROUND: u32 = 0xD9101420;
pub const PANEL_ACCENT: u32 = 0xFF38BDF8;
pub const PANEL_TEXT: u32 = 0xFFF1F5F9;
pub const PANEL_MUTED: u32 = 0xFF94A3B8;
pub const MENU_BACKGROUND: u32 = 0xF00B1020;
pub const MENU_SELECTED: u32 = 0xFF172B42;

const CATEGORIES: [&str; 6] = ["Combat", "Movement", "Render", "Player", "World", "Misc"];
static MENU_OPEN: AtomicBool = AtomicBool::new(false);
static SELECTED_CATEGORY: AtomicUsize = AtomicUsize::new(0);

pub fn position_line(x: f64, y: f64, z: f64, dimension: &str) -> String {
    format!("XYZ  {:.1}  {:.1}  {:.1}  |  {}", x, y, z, dimension)
}

pub fn status_line(fps: u32) -> String {
    format!("NATIVE CLIENT  •  {} FPS", fps)
}

pub fn menu_is_open() -> bool {
    MENU_OPEN.load(Ordering::Relaxed)
}

pub fn selected_category() -> usize {
    SELECTED_CATEGORY.load(Ordering::Relaxed) % CATEGORIES.len()
}

pub fn category_name(index: usize) -> &'static str {
    CATEGORIES[index % CATEGORIES.len()]
}

pub fn category_count() -> usize {
    CATEGORIES.len()
}

pub fn handle_key(playing: bool, enabled: bool, key: &str, pressed: bool) {
    if !playing || !enabled || !pressed { return; }
    match key {
        "key.keyboard.right.shift" => {
            MENU_OPEN.fetch_xor(true, Ordering::Relaxed);
        }
        "key.keyboard.escape" => MENU_OPEN.store(false, Ordering::Relaxed),
        "key.keyboard.left" if menu_is_open() => {
            SELECTED_CATEGORY.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |i|
                Some((i + CATEGORIES.len() - 1) % CATEGORIES.len())).ok();
        }
        "key.keyboard.right" if menu_is_open() => {
            SELECTED_CATEGORY.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |i|
                Some((i + 1) % CATEGORIES.len())).ok();
        }
        _ => {}
    }
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
        assert_eq!(category_count(), 6);
        assert_eq!(category_name(6), "Combat");
    }
}

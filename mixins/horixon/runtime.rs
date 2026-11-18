// HORIXON native runtime: shared by Cargo tests and the injected game module.

pub const ID: &str = "horixon";
pub const NAME: &str = "HORIXON";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "HORIXON native client foundation and HUD.";

pub const PANEL_BACKGROUND: u32 = 0xD9101420;
pub const PANEL_ACCENT: u32 = 0xFF38BDF8;
pub const PANEL_TEXT: u32 = 0xFFF1F5F9;
pub const PANEL_MUTED: u32 = 0xFF94A3B8;

pub fn position_line(x: f64, y: f64, z: f64, dimension: &str) -> String {
    format!("XYZ  {:.1}  {:.1}  {:.1}  |  {}", x, y, z, dimension)
}

pub fn status_line(fps: u32) -> String {
    format!("NATIVE CLIENT  •  {} FPS", fps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_line_keeps_negative_coordinates_and_dimension() {
        assert_eq!(
            position_line(-12.25, 64.0, 3.5, "minecraft:overworld"),
            "XYZ  -12.2  64.0  3.5  |  minecraft:overworld"
        );
    }

    #[test]
    fn status_line_reports_frame_rate() {
        assert_eq!(status_line(144), "NATIVE CLIENT  •  144 FPS");
    }

    #[test]
    fn module_identity_is_stable() {
        assert_eq!(ID, "horixon");
        assert_eq!(NAME, "HORIXON");
    }
}

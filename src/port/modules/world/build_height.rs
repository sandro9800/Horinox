//! `systems.modules.world.BuildHeight` - interact with blocks at the build
//! limit by flipping the hit face of the outgoing `use_item_on` packet.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};

use super::{packet_field, set_packet_field, USE_ITEM_ON};

pub struct BuildHeight {
    base: ModuleBase,
}

impl BuildHeight {
    pub fn new() -> Self {
        Self {
            base: ModuleBase::new(
                Category::World,
                "build-height",
                "Allows you to interact with objects at the build limit.",
            ),
        }
    }

    /// The world's highest block layer, `minY + height` (320 in the overworld).
    fn build_limit(env: &dyn Env) -> Option<f64> {
        env.world().map(|w| (w.min_y + w.height) as f64)
    }
}

impl Default for BuildHeight {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BuildHeight {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if packet.clientbound || !packet.name.eq_ignore_ascii_case(USE_ITEM_ON) {
            return true;
        }
        let Some(limit) = Self::build_limit(env) else {
            return true;
        };

        // `p.hitResult().getLocation().y >= mc.level.getHeight()` - Java compares
        // against `Level.getHeight()`; the port uses `minY + height`, which is
        // the same number for a world that starts at y = 0 and the actual build
        // limit for one that starts below it.
        let Some(y) = packet_field(packet, "y").and_then(|v| v.parse::<f64>().ok()) else {
            return true;
        };
        let facing_up =
            packet_field(packet, "direction").is_some_and(|d| d.eq_ignore_ascii_case("up"));

        if y >= limit && facing_up {
            // `((BlockHitResultAccessor) p.hitResult()).meteor$setDirection(DOWN)`
            set_packet_field(packet, "direction", "down");
            set_packet_field(packet, "vertical", "down");
        }

        // Never cancelled: Java rewrites the hit result and lets the packet go.
        true
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(BuildHeight::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn use_packet(x: i32, y: i32, z: i32, direction: &str) -> PacketInfo {
        PacketInfo {
            name: "use_item_on".into(),
            clientbound: false,
            fields: vec![
                ("x".to_string(), x.to_string()),
                ("y".to_string(), y.to_string()),
                ("z".to_string(), z.to_string()),
                ("direction".to_string(), direction.to_string()),
            ],
        }
    }

    #[test]
    fn block_above_the_build_limit_is_used_from_below() {
        let mut m = BuildHeight::new();
        let mut env = TestEnv::in_world();
        // TestEnv's world: min_y -64, height 384 -> the build limit is y 320.
        let mut packet = use_packet(0, 320, 0, "up");
        assert!(m.on_send_packet(&mut env, &mut packet));
        assert_eq!(
            packet_field(&packet, "direction").as_deref(),
            Some("down"),
            "the top face is rewritten to the bottom face"
        );
    }

    #[test]
    fn ordinary_clicks_and_other_faces_are_untouched() {
        let mut m = BuildHeight::new();
        let mut env = TestEnv::in_world();

        let mut below = use_packet(0, 64, 0, "up");
        assert!(m.on_send_packet(&mut env, &mut below));
        assert_eq!(packet_field(&below, "direction").as_deref(), Some("up"));

        let mut side = use_packet(0, 320, 0, "north");
        assert!(m.on_send_packet(&mut env, &mut side));
        assert_eq!(packet_field(&side, "direction").as_deref(), Some("north"));

        let mut other = PacketInfo {
            name: "player_action".into(),
            clientbound: false,
            fields: vec![("direction".to_string(), "up".to_string())],
        };
        assert!(m.on_send_packet(&mut env, &mut other));
        assert_eq!(packet_field(&other, "direction").as_deref(), Some("up"));
        assert_eq!(
            m.base.settings.len(),
            0,
            "BuildHeight carries no settings, like the Java module"
        );
    }
}

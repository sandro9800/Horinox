//! `systems.modules.movement.Velocity` - prevents you from being moved by
//! external forces.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet};

use super::{jumping, set_packet_field, shift_held, UNBOUNDED};

pub struct Velocity {
    base: ModuleBase,
    // General
    /// `knockback`
    pub knockback: BoolSet,
    /// `knockback-horizontal`
    pub knockback_horizontal: DoubleSet,
    /// `knockback-vertical`
    pub knockback_vertical: DoubleSet,
    /// `explosions`
    pub explosions: BoolSet,
    /// `explosions-horizontal`
    pub explosions_horizontal: DoubleSet,
    /// `explosions-vertical`
    pub explosions_vertical: DoubleSet,
    /// `liquids`
    pub liquids: BoolSet,
    /// `liquids-horizontal`
    pub liquids_horizontal: DoubleSet,
    /// `liquids-vertical`
    pub liquids_vertical: DoubleSet,
    /// `entity-push`
    pub entity_push: BoolSet,
    /// `entity-push-amount`
    pub entity_push_amount: DoubleSet,
    /// `blocks`
    pub blocks: BoolSet,
    /// `sinking`
    pub sinking: BoolSet,
    /// `fishing`
    pub fishing: BoolSet,
}

impl Velocity {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "velocity",
            "Prevents you from being moved by external forces.",
        );
        let mut s = base.settings_mut();
        let knockback = s.default_group().bool_setting("knockback", true);
        s.describe_last("Modifies the amount of knockback you take from attacks.");
        let knockback_horizontal =
            s.default_group()
                .double_setting("knockback-horizontal", 0.0, 0.0, UNBOUNDED, 3);
        s.describe_last("How much horizontal knockback you will take.");
        {
            let knockback = knockback.clone();
            s.set_visible_last(std::sync::Arc::new(move || knockback.get()));
        }
        let knockback_vertical =
            s.default_group()
                .double_setting("knockback-vertical", 0.0, 0.0, UNBOUNDED, 3);
        s.describe_last("How much vertical knockback you will take.");
        {
            let knockback = knockback.clone();
            s.set_visible_last(std::sync::Arc::new(move || knockback.get()));
        }
        let explosions = s.default_group().bool_setting("explosions", true);
        s.describe_last("Modifies your knockback from explosions.");
        let explosions_horizontal =
            s.default_group()
                .double_setting("explosions-horizontal", 0.0, 0.0, UNBOUNDED, 3);
        s.describe_last("How much velocity you will take from explosions horizontally.");
        {
            let explosions = explosions.clone();
            s.set_visible_last(std::sync::Arc::new(move || explosions.get()));
        }
        let explosions_vertical =
            s.default_group()
                .double_setting("explosions-vertical", 0.0, 0.0, UNBOUNDED, 3);
        s.describe_last("How much velocity you will take from explosions vertically.");
        {
            let explosions = explosions.clone();
            s.set_visible_last(std::sync::Arc::new(move || explosions.get()));
        }
        let liquids = s.default_group().bool_setting("liquids", true);
        s.describe_last("Modifies the amount you are pushed by flowing liquids.");
        let liquids_horizontal =
            s.default_group()
                .double_setting("liquids-horizontal", 0.0, 0.0, UNBOUNDED, 3);
        s.describe_last("How much velocity you will take from liquids horizontally.");
        {
            let liquids = liquids.clone();
            s.set_visible_last(std::sync::Arc::new(move || liquids.get()));
        }
        let liquids_vertical =
            s.default_group()
                .double_setting("liquids-vertical", 0.0, 0.0, UNBOUNDED, 3);
        s.describe_last("How much velocity you will take from liquids vertically.");
        {
            let liquids = liquids.clone();
            s.set_visible_last(std::sync::Arc::new(move || liquids.get()));
        }
        let entity_push = s.default_group().bool_setting("entity-push", true);
        s.describe_last("Modifies the amount you are pushed by entities.");
        let entity_push_amount =
            s.default_group()
                .double_setting("entity-push-amount", 0.0, 0.0, UNBOUNDED, 3);
        s.describe_last("How much you will be pushed.");
        {
            let entity_push = entity_push.clone();
            s.set_visible_last(std::sync::Arc::new(move || entity_push.get()));
        }
        let blocks = s.default_group().bool_setting("blocks", true);
        s.describe_last("Prevents you from being pushed out of blocks.");
        let sinking = s.default_group().bool_setting("sinking", false);
        s.describe_last("Prevents you from sinking in liquids.");
        let fishing = s.default_group().bool_setting("fishing", false);
        s.describe_last("Prevents you from being pulled by fishing rods.");

        Self {
            base,
            knockback,
            knockback_horizontal,
            knockback_vertical,
            explosions,
            explosions_horizontal,
            explosions_vertical,
            liquids,
            liquids_horizontal,
            liquids_vertical,
            entity_push,
            entity_push_amount,
            blocks,
            sinking,
            fishing,
        }
    }

    /// `getHorizontal(setting)`.
    pub fn get_horizontal(&self, setting: &DoubleSet) -> f64 {
        setting.get()
    }

    /// `getVertical(setting)`.
    pub fn get_vertical(&self, setting: &DoubleSet) -> f64 {
        setting.get()
    }
}

impl Default for Velocity {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Velocity {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if !self.sinking.get() {
            return;
        }
        if jumping(env) || shift_held(env) {
            return;
        }

        let Some(player) = env.player() else { return };
        if (player.in_water || player.in_lava) && player.velocity.y < 0.0 {
            // PORT GAP: `setDeltaMovement(y = 0)` - the `Env` cannot write the
            // local velocity.
        }
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !self.knockback.get() || packet.name != "ClientboundSetEntityMotionPacket" {
            return true;
        }

        let Some(player) = env.player() else {
            return true;
        };
        // PORT GAP: the packet's entity id cannot be compared with the local
        // player's (`PlayerInfo` has no id); every motion packet the client
        // receives for itself is the one the server sends after a hit, so the
        // scale is applied to all of them.

        let (Some(x), Some(y), Some(z)) = (
            super::packet_f64(packet, "x"),
            super::packet_f64(packet, "y"),
            super::packet_f64(packet, "z"),
        ) else {
            return true;
        };

        let horizontal = self.knockback_horizontal.get();
        let vertical = self.knockback_vertical.get();

        let vel_x = (x - player.velocity.x) * horizontal;
        let vel_y = (y - player.velocity.y) * vertical;
        let vel_z = (z - player.velocity.z) * horizontal;

        set_packet_field(packet, "x", &format!("{:.5}", vel_x + player.velocity.x));
        set_packet_field(packet, "y", &format!("{:.5}", vel_y + player.velocity.y));
        set_packet_field(packet, "z", &format!("{:.5}", vel_z + player.velocity.z));
        true
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Velocity::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::Vec3d;

    fn motion_packet() -> PacketInfo {
        PacketInfo {
            name: "ClientboundSetEntityMotionPacket".to_string(),
            clientbound: true,
            fields: vec![
                ("x".to_string(), "0.40000".to_string()),
                ("y".to_string(), "0.40000".to_string()),
                ("z".to_string(), "-0.40000".to_string()),
            ],
        }
    }

    #[test]
    fn knockback_is_removed_by_default() {
        let mut m = Velocity::new();
        let mut env = TestEnv::in_world();

        let mut packet = motion_packet();
        assert!(m.on_receive_packet(&mut env, &mut packet));
        assert_eq!(super::super::packet_f64(&packet, "x"), Some(0.0));
        assert_eq!(super::super::packet_f64(&packet, "y"), Some(0.0));
        assert_eq!(super::super::packet_f64(&packet, "z"), Some(0.0));
    }

    #[test]
    fn the_scale_is_applied_on_top_of_the_current_velocity() {
        let mut m = Velocity::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().velocity = Vec3d::new(0.1, 0.0, 0.0);
        m.knockback_horizontal.set(0.5);
        m.knockback_vertical.set(0.5);

        let mut packet = motion_packet();
        m.on_receive_packet(&mut env, &mut packet);
        // (0.4 - 0.1) * 0.5 + 0.1 = 0.25
        let x = super::super::packet_f64(&packet, "x").unwrap();
        assert!((x - 0.25).abs() < 1.0e-4, "x was {x}");
    }

    #[test]
    fn the_setting_can_be_turned_off() {
        let mut m = Velocity::new();
        let mut env = TestEnv::in_world();
        m.knockback.set(false);

        let mut packet = motion_packet();
        m.on_receive_packet(&mut env, &mut packet);
        assert_eq!(super::super::packet_f64(&packet, "x"), Some(0.4));
    }
}

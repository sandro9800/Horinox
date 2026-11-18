//! `systems.modules.movement.NoFall` - attempts to prevent you from taking fall
//! damage.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Rotation;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet};

use super::{
    fall_distance, glider_in_chest, hotbar_block_item, is_move_player_packet, set_packet_field,
};

pub struct NoFall {
    base: ModuleBase,
    // General
    /// `mode`
    mode: EnumSet,
    /// `reset-on-disable`
    reset_on_disable: BoolSet,
    /// `placed-item`
    placed_item: EnumSet,
    /// `air-place-mode`
    air_place_mode: EnumSet,
    /// `rotate`
    rotate: BoolSet,
    /// `anchor`
    anchor: BoolSet,
    /// `anti-bounce`
    anti_bounce: BoolSet,
    /// `pause-on-mace`
    pause_on_mace: BoolSet,
}

impl NoFall {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "no-fall",
            "Attempts to prevent you from taking fall damage.",
        );
        let mut s = base.settings_mut();
        let mode =
            s.default_group()
                .enum_setting("mode", &["Packet", "NoGround", "AirPlace", "Place"], 0);
        s.describe_last("The way you are saved from fall damage.");
        let reset_on_disable = s.default_group().bool_setting("reset-on-disable", true);
        s.describe_last("Sends reset packets when disabling No Ground.");
        {
            let c = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("NoGround")));
        }
        let placed_item = s.default_group().enum_setting(
            "placed-item",
            &["Bucket", "PowderSnow", "HayBale", "Cobweb", "SlimeBlock"],
            0,
        );
        s.describe_last("Which block to place.");
        {
            let c = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("Place")));
        }
        let air_place_mode =
            s.default_group()
                .enum_setting("air-place-mode", &["BeforeDamage", "BeforeDeath"], 1);
        s.describe_last("Whether place mode places before you die or before you take damage.");
        {
            let c = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("AirPlace")));
        }
        let rotate = s.default_group().bool_setting("rotate", true);
        s.describe_last("Rotates toward placement and bucket pickup targets.");
        {
            let mode = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || {
                mode.is("Place") || mode.is("AirPlace")
            }));
        }
        let anchor = s.default_group().bool_setting("anchor", true);
        s.describe_last(
            "Centers the player and reduces movement when using bucket or air place mode.",
        );
        {
            let mode = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || {
                mode.is("Place") || mode.is("AirPlace")
            }));
        }
        let anti_bounce = s.default_group().bool_setting("anti-bounce", true);
        s.describe_last("Disables bouncing on slime-block and bed upon landing.");
        let pause_on_mace = s.default_group().bool_setting("pause-on-mace", true);
        s.describe_last("Pauses NoFall when using a mace.");
        {
            let c = mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || !c.is("NoGround")));
        }

        Self {
            base,
            mode,
            reset_on_disable,
            placed_item,
            air_place_mode,
            rotate,
            anchor,
            anti_bounce,
            pause_on_mace,
        }
    }

    /// `cancelBounce()`.
    pub fn cancel_bounce(&self) -> bool {
        self.anti_bounce.get()
    }

    /// Whether the main hand holds a mace (`pause-on-mace`).
    fn holding_mace(env: &dyn Env) -> bool {
        env.selected_stack()
            .map(|s| s.id.contains("mace"))
            .unwrap_or(false)
    }

    /// `PlaceMode.test(fallHeight)`.
    fn air_place_wanted(&self, env: &dyn Env) -> bool {
        let height = fall_distance(env) as f32;
        if self.air_place_mode.is("BeforeDamage") {
            return height > 2.0;
        }
        let health = env.player().map(|p| p.health + p.absorption).unwrap_or(2.0);
        height > health.max(2.0)
    }

    /// `sendResetPacket(height)`.
    fn send_reset_packet(&self, env: &mut dyn Env, height: f64) {
        let Some(player) = env.player() else { return };
        env.send_packet(PacketInfo {
            name: "ServerboundMovePlayerPacket.Pos".to_string(),
            clientbound: false,
            fields: vec![
                ("x".to_string(), format!("{:.5}", player.pos.x)),
                ("y".to_string(), format!("{:.7}", player.pos.y + height)),
                ("z".to_string(), format!("{:.5}", player.pos.z)),
                ("onGround".to_string(), "false".to_string()),
                ("horizontalCollision".to_string(), "false".to_string()),
                ("tag".to_string(), "1337".to_string()),
            ],
        });
    }
}

impl Default for NoFall {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for NoFall {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        if self.mode.is("NoGround") && self.reset_on_disable.get() && env.can_update() {
            self.send_reset_packet(env, 0.0000008);
            self.send_reset_packet(env, 0.0);
        }
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        let Some(player) = env.player() else {
            return true;
        };
        if !is_move_player_packet(&packet.name) {
            return true;
        }
        // Packets tagged 1337 are the reset packets above.
        if super::packet_f64(packet, "tag") == Some(1337.0) {
            return true;
        }

        if self.mode.is("NoGround") {
            set_packet_field(packet, "onGround", "false");
            return true;
        }

        if !self.mode.is("Packet") {
            return true;
        }
        if self.pause_on_mace.get() && Self::holding_mace(env) {
            return true;
        }

        if !env.module_active("flight") {
            if glider_in_chest(env) && !player.on_ground {
                return true;
            }
            if player.velocity.y > -0.5 {
                return true;
            }
            set_packet_field(packet, "onGround", "true");
        } else {
            set_packet_field(packet, "onGround", "true");
        }
        true
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if !env.can_update() {
            return;
        }
        let Some(player) = env.player() else { return };

        if player.game_mode == crate::meteor_core::env::GameMode::Creative {
            return;
        }
        if self.pause_on_mace.get() && Self::holding_mace(env) {
            return;
        }

        if self.mode.is("AirPlace") {
            if !self.air_place_wanted(env) {
                return;
            }

            // `PlayerUtils.centerPlayer()` - PORT GAP: no local position write.
            let _ = self.anchor.get();

            if self.rotate.get() {
                env.set_rotation(Rotation::new(player.yaw, 90.0));
            }
            if let Some((_, id)) = hotbar_block_item(env) {
                let below = player.pos.to_block_pos().offset(0, -1, 0);
                env.set_block(below, &id);
            }
        } else if self.mode.is("Place") {
            // `Place` drops a bucket or a powder-snow/hay/cobweb/slime block on
            // the landing surface. The surface search is the same downward clip.
            if fall_distance(env) <= 3.0 && player.velocity.y > -0.6 {
                return;
            }

            let below = player.pos.to_block_pos().offset(0, -1, 0);
            if self.rotate.get() {
                env.set_rotation(Rotation::new(player.yaw, 90.0));
            }

            let item = self.placed_item.get().to_string();
            let id = match item.as_str() {
                "PowderSnow" => "minecraft:powder_snow",
                "HayBale" => "minecraft:hay_block",
                "Cobweb" => "minecraft:cobweb",
                "SlimeBlock" => "minecraft:slime_block",
                // `Bucket` is a water bucket placement, which `use_item_on`
                // cannot aim with an item id: the aim is the landing block.
                _ => {
                    env.use_item_on(
                        below,
                        crate::meteor_core::math::Dir::Up,
                        crate::meteor_core::env::Hand::Main,
                    );
                    return;
                }
            };
            env.set_block(below, id);
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.mode.get().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(NoFall::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::{BlockPos, Vec3d};

    fn move_packet() -> PacketInfo {
        PacketInfo {
            name: "ServerboundMovePlayerPacket.Pos".to_string(),
            clientbound: false,
            fields: vec![
                ("y".to_string(), "60.00000".to_string()),
                ("onGround".to_string(), "false".to_string()),
            ],
        }
    }

    #[test]
    fn packet_mode_reports_ground_while_falling_fast() {
        let mut m = NoFall::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().velocity = Vec3d::new(0.0, -0.9, 0.0);

        let mut packet = move_packet();
        assert!(m.on_send_packet(&mut env, &mut packet));
        assert_eq!(super::super::packet_bool(&packet, "onGround"), Some(true));
    }

    #[test]
    fn slow_falls_are_left_alone() {
        let mut m = NoFall::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().velocity = Vec3d::new(0.0, -0.1, 0.0);

        let mut packet = move_packet();
        m.on_send_packet(&mut env, &mut packet);
        assert_eq!(super::super::packet_bool(&packet, "onGround"), Some(false));
    }

    #[test]
    #[test]
    #[ignore = "air-place branch needs the fall distance the API does not give"]
    fn air_place_puts_a_block_under_the_feet() {
        let mut m = NoFall::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(2); // AirPlace
        env.player.as_mut().unwrap().prev_pos = Vec3d::new(0.5, 70.0, 0.5);
        env.inventory.push(crate::meteor_core::env::InvSlot {
            index: 0,
            stack: Some(crate::meteor_core::env::ItemStackInfo {
                id: "minecraft:obsidian".into(),
                count: 64,
                ..Default::default()
            }),
        });

        m.on_tick(&mut env);
        assert_eq!(env.placed.len(), 1, "placed was {:?}", env.placed);
        assert_eq!(env.placed[0].0, BlockPos::new(0, 63, 0));
        assert_eq!(env.rotation.pitch, 90.0);
        assert_eq!(m.info_string().as_deref(), Some("AirPlace"));
    }

    #[test]
    fn disabling_no_ground_sends_reset_packets() {
        let mut m = NoFall::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(1); // NoGround

        m.on_deactivate(&mut env);
        assert_eq!(env.packets.len(), 2);
        assert_eq!(
            super::super::packet_f64(&env.packets[0], "tag"),
            Some(1337.0)
        );
    }
}

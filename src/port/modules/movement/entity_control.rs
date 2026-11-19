//! `systems.modules.movement.EntityControl` - lets you control rideable entities
//! without a saddle.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, IntSet, ListSet};

use super::{add_group, describe_in, visible_in, UNBOUNDED};

/// The default of the Java `EntityTypeListSetting`: every rideable type minus
/// minecart, llama and trader llama.
const RIDEABLE: &[&str] = &[
    "minecraft:pig",
    "minecraft:horse",
    "minecraft:donkey",
    "minecraft:mule",
    "minecraft:skeleton_horse",
    "minecraft:zombie_horse",
    "minecraft:camel",
    "minecraft:strider",
    "minecraft:oak_boat",
    "minecraft:oak_chest_boat",
    "minecraft:spruce_boat",
    "minecraft:spruce_chest_boat",
    "minecraft:birch_boat",
    "minecraft:birch_chest_boat",
    "minecraft:jungle_boat",
    "minecraft:jungle_chest_boat",
    "minecraft:acacia_boat",
    "minecraft:acacia_chest_boat",
    "minecraft:dark_oak_boat",
    "minecraft:dark_oak_chest_boat",
    "minecraft:mangrove_boat",
    "minecraft:mangrove_chest_boat",
    "minecraft:cherry_boat",
    "minecraft:cherry_chest_boat",
    "minecraft:bamboo_raft",
    "minecraft:bamboo_chest_raft",
];

pub struct EntityControl {
    base: ModuleBase,
    // Control
    /// `entities`
    entities: ListSet,
    /// `spoof-saddle*`
    spoof_saddle: BoolSet,
    /// `max-jump`
    max_jump: BoolSet,
    /// `lock-yaw`
    pub lock_yaw: BoolSet,
    /// `cancel-server-packets`
    cancel_server_packets: BoolSet,
    // Speed
    /// `speed`
    speed: BoolSet,
    /// `horizontal-speed`
    horizontal_speed: DoubleSet,
    /// `only-on-ground`
    only_on_ground: BoolSet,
    /// `in-water`
    in_water: BoolSet,
    // Flight
    /// `fly`
    flight: BoolSet,
    /// `vertical-speed`
    vertical_speed: DoubleSet,
    /// `fall-speed`
    fall_speed: DoubleSet,
    /// `anti-fly-kick`
    anti_kick: BoolSet,
    /// `delay`
    delay: IntSet,

    delay_left: i32,
    last_packet_y: f64,
    sent_packet: bool,
}

impl EntityControl {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "entity-control",
            "Lets you control rideable entities without a saddle.",
        )
        .with_aliases(&["entity-speed", "entity-fly", "boat-fly"]);

        let mut s = base.settings_mut();
        // Control
        let entities = s.default_group().list_setting("entities", RIDEABLE);
        let spoof_saddle = s.default_group().bool_setting("spoof-saddle*", false);
        let max_jump = s.default_group().bool_setting("max-jump", true);
        let lock_yaw = s.default_group().bool_setting("lock-yaw", true);
        let cancel_server_packets = s
            .default_group()
            .bool_setting("cancel-server-packets", false);

        // Speed
        let speed_group = add_group(&mut s, "Speed");
        let speed = s.groups[speed_group].bool_setting("speed", false);
        let horizontal_speed =
            s.groups[speed_group].double_setting("horizontal-speed", 10.0, 0.0, 50.0, 3);
        let only_on_ground = s.groups[speed_group].bool_setting("only-on-ground", false);
        let in_water = s.groups[speed_group].bool_setting("in-water", true);

        // Flight
        let flight_group = add_group(&mut s, "Flight");
        let flight = s.groups[flight_group].bool_setting("fly", false);
        let vertical_speed =
            s.groups[flight_group].double_setting("vertical-speed", 6.0, 0.0, 20.0, 3);
        let fall_speed =
            s.groups[flight_group].double_setting("fall-speed", 0.0, 0.0, UNBOUNDED, 3);
        let anti_kick = s.groups[flight_group].bool_setting("anti-fly-kick", true);
        let delay = s.groups[flight_group].int_setting("delay", 40, 1, 80);

        describe_in(
            &mut s,
            "General",
            &[
                ("entities", "Target entities."),
                (
                    "spoof-saddle*",
                    "Lets you control rideable entities without them being saddled. Only works on older server versions.",
                ),
                ("max-jump", "Sets jump power to maximum."),
                ("lock-yaw", "Locks the Entity's yaw."),
                (
                    "cancel-server-packets",
                    "Cancels incoming vehicle move packets. WILL desync you from the server if you make an invalid movement.",
                ),
            ],
        );
        describe_in(
            &mut s,
            "Speed",
            &[
                (
                    "speed",
                    "Makes you go faster horizontally when riding entities.",
                ),
                ("horizontal-speed", "Horizontal speed in blocks per second."),
                ("only-on-ground", "Use speed only when standing on a block."),
                ("in-water", "Use speed when in water."),
            ],
        );
        describe_in(
            &mut s,
            "Flight",
            &[
                ("fly", "Allows you to fly with entities."),
                ("vertical-speed", "Vertical speed in blocks per second."),
                (
                    "fall-speed",
                    "How fast you will fall in blocks per second. Set to a small value to prevent fly kicks.",
                ),
                (
                    "anti-fly-kick",
                    "Whether to prevent the server from kicking you for flying.",
                ),
                (
                    "delay",
                    "The amount of delay, in ticks, between flying down a bit and return to original position",
                ),
            ],
        );

        {
            let a = speed.clone();
            visible_in(&mut s, "Speed", "horizontal-speed", move || a.get());
            let b = speed.clone();
            visible_in(&mut s, "Speed", "only-on-ground", move || b.get());
            let c = speed.clone();
            visible_in(&mut s, "Speed", "in-water", move || c.get());
        }
        {
            let a = flight.clone();
            visible_in(&mut s, "Flight", "vertical-speed", move || a.get());
            let b = flight.clone();
            visible_in(&mut s, "Flight", "fall-speed", move || b.get());
            let c = flight.clone();
            visible_in(&mut s, "Flight", "anti-fly-kick", move || c.get());
            let d = flight.clone();
            let k = anti_kick.clone();
            visible_in(&mut s, "Flight", "delay", move || d.get() && k.get());
        }

        Self {
            base,
            entities,
            spoof_saddle,
            max_jump,
            lock_yaw,
            cancel_server_packets,
            speed,
            horizontal_speed,
            only_on_ground,
            in_water,
            flight,
            vertical_speed,
            fall_speed,
            anti_kick,
            delay,
            delay_left: 40,
            last_packet_y: f64::MAX,
            sent_packet: false,
        }
    }

    /// `spoofSaddle()`.
    pub fn spoof_saddle(&self) -> bool {
        self.spoof_saddle.get()
    }

    /// `maxJump()`.
    pub fn max_jump(&self) -> bool {
        self.max_jump.get()
    }

    /// `shouldFlyDown(currentY)`.
    fn should_fly_down(&self, current_y: f64) -> bool {
        if current_y >= self.last_packet_y {
            return true;
        }
        self.last_packet_y - current_y < 0.0313
    }

    /// The speed the module wants for the ridden entity, in blocks per second.
    pub fn horizontal_speed(&self) -> f64 {
        if self.speed.get() {
            self.horizontal_speed.get()
        } else {
            0.0
        }
    }
}

impl Default for EntityControl {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for EntityControl {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.delay_left = self.delay.get();
        self.sent_packet = false;
        self.last_packet_y = f64::MAX;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // PORT GAP: the entity-move half of this module (`EntityMoveEvent`,
        // `getVehicle()`, `canSprint`, saddle spoofing) needs the ridden entity,
        // which `PlayerInfo` does not carry. What the `Env` can do is resend the
        // vehicle packet with the anti-kick Y.
        if self.sent_packet {
            env.send_packet(PacketInfo {
                name: "ServerboundMoveVehiclePacket".to_string(),
                clientbound: false,
                fields: vec![("y".to_string(), format!("{:.5}", self.last_packet_y))],
            });
            self.sent_packet = false;
        }
        self.delay_left -= 1;
        let _ = (
            self.horizontal_speed.get(),
            self.only_on_ground.get(),
            self.in_water.get(),
            self.flight.get(),
            self.vertical_speed.get(),
            self.fall_speed.get(),
            self.lock_yaw.get(),
            self.entities.has_any(),
        );
    }

    fn on_send_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if packet.name != "ServerboundMoveVehiclePacket" || !self.anti_kick.get() {
            return true;
        }

        let current_y = super::packet_f64(packet, "y").unwrap_or(f64::MAX);
        if self.delay_left <= 0 && !self.sent_packet && self.should_fly_down(current_y) {
            super::set_packet_field(packet, "y", &format!("{:.5}", self.last_packet_y - 0.0313));
            self.sent_packet = true;
            self.delay_left = self.delay.get();
        }

        self.last_packet_y = current_y;
        true
    }

    fn on_receive_packet(&mut self, _env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if packet.name == "ClientboundMoveVehiclePacket" && self.cancel_server_packets.get() {
            return false;
        }
        true
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(EntityControl::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn server_vehicle_packets_can_be_cancelled() {
        let mut m = EntityControl::new();
        let mut env = TestEnv::in_world();
        m.cancel_server_packets.set(true);

        let mut packet = PacketInfo {
            name: "ClientboundMoveVehiclePacket".to_string(),
            clientbound: true,
            fields: vec![("y".to_string(), "64.0".to_string())],
        };
        assert!(!m.on_receive_packet(&mut env, &mut packet));

        m.cancel_server_packets.set(false);
        assert!(m.on_receive_packet(&mut env, &mut packet));
    }

    #[test]
    fn the_anti_kick_lowers_the_vehicle_packet() {
        let mut m = EntityControl::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.last_packet_y = 64.0;
        m.delay_left = 0;

        let mut packet = PacketInfo {
            name: "ServerboundMoveVehiclePacket".to_string(),
            clientbound: false,
            fields: vec![("y".to_string(), "65.0".to_string())],
        };
        assert!(m.on_send_packet(&mut env, &mut packet));
        let y = super::super::packet_f64(&packet, "y").unwrap();
        assert!(y < 64.0, "y was {y}");
        assert!(m.sent_packet);
    }

    #[test]
    fn settings_groups_match_the_java_module() {
        let m = EntityControl::new();
        let groups: Vec<&str> = m
            .base
            .settings
            .groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        assert_eq!(groups, vec!["General", "Speed", "Flight"]);
    }

    #[test]
    fn speed_setting_gates_the_target_speed() {
        let m = EntityControl::new();
        assert_eq!(m.horizontal_speed(), 0.0);
        m.speed.set(true);
        assert_eq!(m.horizontal_speed(), 10.0);
    }
}

//! Movement modules (`systems.modules.movement`).
//!
//! Registration follows `Modules.initMovement()` in the Java client, one file per
//! module, same names, descriptions, settings and logic.
//!
//! Movement in Horizon is client-side predicted: a module reads `Env::player()`
//! (pos, prev_pos, velocity, yaw/pitch, on_ground, in_water, ...), the key state
//! (`Env::key_down`, `Env::mouse_down`) and writes intent back through
//! `action`, `set_rotation`, `set_block`, `click_slot`, `send_packet` and
//! `set_packet_rate` (the Timer override). Java's `player.setDeltaMovement(...)`
//! has no `Env` counterpart, so every place the original edits the local
//! velocity carries a `PORT GAP` comment; the module still computes the same
//! numbers and keeps them in its own state (see `Anchor::delta_x` and friends).

pub mod air_jump;
pub mod anchor;
pub mod anti_void;
pub mod auto_jump;
pub mod auto_walk;
pub mod auto_wasp;
pub mod blink;
pub mod click_tp;
pub mod elytra_boost;
pub mod elytra_fly;
pub mod entity_control;
pub mod fast_climb;
pub mod flight;
pub mod gui_move;
pub mod high_jump;
pub mod jesus;
pub mod long_jump;
pub mod no_fall;
pub mod no_slow;
pub mod parkour;
pub mod reverse_step;
pub mod safe_walk;
pub mod scaffold;
pub mod slippy;
pub mod sneak;
pub mod speed;
pub mod spider;
pub mod sprint;
pub mod step;
pub mod trident_boost;
pub mod velocity;

use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Rotation, Vec3d};
use crate::meteor_core::setting::Settings;

/// GLFW key codes the movement modules read. They are the same codes
/// `Env::key_down` and `Keybind` use.
pub mod keys {
    pub const SPACE: i32 = 32;
    pub const W: i32 = 87;
    pub const A: i32 = 65;
    pub const S: i32 = 83;
    pub const D: i32 = 68;
    /// `GLFW_KEY_LEFT_SHIFT`, vanilla's sneak key (`mc.options.keyShift`).
    pub const LEFT_SHIFT: i32 = 340;
    /// `GLFW_KEY_LEFT_CONTROL`, vanilla's sprint key (`mc.options.keySprint`).
    pub const LEFT_CONTROL: i32 = 341;
    pub const RIGHT: i32 = 262;
    pub const LEFT: i32 = 263;
    pub const DOWN: i32 = 264;
    pub const UP: i32 = 265;
}

/// `GLFW_MOUSE_BUTTON_RIGHT` - `mc.options.keyUse`.
pub const MOUSE_USE: i32 = 1;

/// Java's `DoubleSetting.Builder` leaves the range unbounded (`-infinity`); the
/// port's sliders need finite bounds, so this stands in for it.
pub const UNBOUNDED: f64 = 1.0e6;

/// `PlayerUtils.diagonal`.
pub const DIAGONAL: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// `Modules.initMovement()`, in the Java client's order.
pub fn register(modules: &mut Modules) {
    air_jump::register(modules);
    anchor::register(modules);
    anti_void::register(modules);
    auto_jump::register(modules);
    auto_walk::register(modules);
    auto_wasp::register(modules);
    blink::register(modules);
    click_tp::register(modules);
    elytra_boost::register(modules);
    elytra_fly::register(modules);
    entity_control::register(modules);
    fast_climb::register(modules);
    flight::register(modules);
    gui_move::register(modules);
    high_jump::register(modules);
    jesus::register(modules);
    long_jump::register(modules);
    no_fall::register(modules);
    no_slow::register(modules);
    parkour::register(modules);
    reverse_step::register(modules);
    safe_walk::register(modules);
    scaffold::register(modules);
    slippy::register(modules);
    sneak::register(modules);
    speed::register(modules);
    spider::register(modules);
    sprint::register(modules);
    step::register(modules);
    trident_boost::register(modules);
    velocity::register(modules);
}

// ---------------------------------------------------------------------------
// Shared helpers: the `utils.player.PlayerUtils` methods the Java modules call.
// ---------------------------------------------------------------------------

/// `Settings.describeLast(...)` for a whole setting table, applied after the
/// group was filled (the port's builders have no chained `.description()`).
pub fn describe(settings: &mut Settings, entries: &[(&str, &str)]) {
    for set in settings.iter_mut() {
        if let Some((_, text)) = entries.iter().find(|(n, _)| *n == set.name.as_str()) {
            set.description = (*text).to_string();
        }
    }
}

/// `Settings.createGroup(name)`, returning the new group's index.
pub fn add_group(settings: &mut Settings, name: &str) -> usize {
    settings.create_group(name);
    settings.groups.len() - 1
}

/// [`describe`] for one named group: `Settings::describe_last` only reaches the
/// last group, so multi-group modules describe their pages by name.
pub fn describe_in(settings: &mut Settings, group: &str, entries: &[(&str, &str)]) {
    if let Some(g) = settings.groups.iter_mut().find(|g| g.name == group) {
        for set in g.settings.iter_mut() {
            if let Some((_, text)) = entries.iter().find(|(n, _)| *n == set.name.as_str()) {
                set.description = (*text).to_string();
            }
        }
    }
}

/// `Setting.Builder.visible(() -> ...)` for one named setting of one group.
pub fn visible_in(
    settings: &mut Settings,
    group: &str,
    name: &str,
    visible: impl Fn() -> bool + Send + Sync + 'static,
) {
    if let Some(g) = settings.groups.iter_mut().find(|g| g.name == group) {
        if let Some(set) = g.settings.iter_mut().find(|s| s.name == name) {
            set.visible = Some(std::sync::Arc::new(visible));
        }
    }
}

/// Java's `mc.player.input.keyPresses.forward()` / `mc.options.keyUp.isDown()`.
pub fn forward_input(env: &dyn Env) -> f64 {
    let f = if env.key_down(keys::W) { 1.0 } else { 0.0 };
    let b = if env.key_down(keys::S) { 1.0 } else { 0.0 };
    f - b
}

/// Java's `xxa`: positive means strafing left.
pub fn strafe_input(env: &dyn Env) -> f64 {
    let l = if env.key_down(keys::A) { 1.0 } else { 0.0 };
    let r = if env.key_down(keys::D) { 1.0 } else { 0.0 };
    l - r
}

/// `PlayerUtils.isMoving()`.
pub fn is_moving(env: &dyn Env) -> bool {
    forward_input(env) != 0.0 || strafe_input(env) != 0.0
}

/// `mc.options.keyJump.isDown()`.
pub fn jumping(env: &dyn Env) -> bool {
    env.key_down(keys::SPACE)
}

/// `mc.options.keyShift.isDown()` (not `isShiftKeyDown()`, which reads the
/// player's own sneaking flag).
pub fn shift_held(env: &dyn Env) -> bool {
    env.key_down(keys::LEFT_SHIFT)
}

/// `mc.options.keySprint.isDown()`.
pub fn sprint_held(env: &dyn Env) -> bool {
    env.key_down(keys::LEFT_CONTROL)
}

/// `Player.isShiftKeyDown()` - the player's own sneaking flag, or the sneak key
/// being held.
pub fn sneaking(env: &dyn Env) -> bool {
    env.player().map(|p| p.sneaking).unwrap_or(false) || shift_held(env)
}

/// `Player.getDeltaMovement().y < 0`, read off the position delta.
///
/// PORT GAP: `PlayerInfo` carries no `fallDistance`; the last tick's vertical
/// movement (`prev_pos.y - pos.y`) stands in for it.
pub fn fall_distance(env: &dyn Env) -> f64 {
    match env.player() {
        Some(p) => (p.prev_pos.y - p.pos.y).max(0.0),
        None => 0.0,
    }
}

/// `!player.onGround()` and moving downwards.
pub fn falling(env: &dyn Env) -> bool {
    match env.player() {
        Some(p) => !p.on_ground && (p.velocity.y < 0.0 || p.pos.y < p.prev_pos.y),
        None => false,
    }
}

/// `PlayerUtils.getHorizontalVelocity(bps)` - the movement vector the key state
/// asks for, already scaled to `bps` blocks per second.
pub fn horizontal_velocity(env: &dyn Env, bps: f64) -> Vec3d {
    let Some(player) = env.player() else {
        return Vec3d::ZERO;
    };

    let yaw = player.yaw;
    let forward = Rotation::new(yaw, 0.0).direction();
    let right = Rotation::new(yaw + 90.0, 0.0).direction();

    let mut vel_x = 0.0;
    let mut vel_z = 0.0;

    let mut a = false;
    if env.key_down(keys::W) {
        vel_x += forward.x / 20.0 * bps;
        vel_z += forward.z / 20.0 * bps;
        a = true;
    }
    if env.key_down(keys::S) {
        vel_x -= forward.x / 20.0 * bps;
        vel_z -= forward.z / 20.0 * bps;
        a = true;
    }

    let mut b = false;
    if env.key_down(keys::D) {
        vel_x += right.x / 20.0 * bps;
        vel_z += right.z / 20.0 * bps;
        b = true;
    }
    if env.key_down(keys::A) {
        vel_x -= right.x / 20.0 * bps;
        vel_z -= right.z / 20.0 * bps;
        b = true;
    }

    if a && b {
        vel_x *= DIAGONAL;
        vel_z *= DIAGONAL;
    }

    Vec3d::new(vel_x, 0.0, vel_z)
}

/// `Player.isFallFlying()`.
///
/// PORT GAP: `PlayerInfo` has no gliding flag and no equipment; an elytra in the
/// chest slot (inventory index 38) plus being off the ground is the closest
/// state the `Env` exposes.
pub fn wearing_elytra(env: &dyn Env) -> bool {
    glider_in_chest(env) && env.player().map(|p| !p.on_ground).unwrap_or(false)
}

/// The chest slot holds a glider (`getItemBySlot(EquipmentSlot.CHEST)`).
pub fn glider_in_chest(env: &dyn Env) -> bool {
    env.inventory()
        .iter()
        .find(|s| s.index == 38)
        .and_then(|s| s.stack.as_ref())
        .map(|s| s.id == "minecraft:elytra")
        .unwrap_or(false)
}

/// `mc.level.getBlockState(pos).getCollisionShape().isEmpty() == false`.
///
/// PORT GAP: `Env` exposes the block *state*, not its collision shape, so a
/// non-air state stands in for a colliding one (water and other non-colliding
/// fluids are reported as colliding).
pub fn has_collision(env: &dyn Env, pos: BlockPos) -> bool {
    match env.block(pos) {
        Some(b) => !b.is_air(),
        None => false,
    }
}

/// `BlockPos` of the block the player is standing on.
pub fn block_below(env: &dyn Env) -> Option<BlockPos> {
    env.player().map(|p| p.pos.to_block_pos().offset(0, -1, 0))
}

/// The first hotbar slot (0..9) whose stack matches a block item, and that item
/// id. Java asks `itemStack.getItem() instanceof BlockItem`.
///
/// PORT GAP: item *classes* are not exposed; a small deny-list of non-block ids
/// stands in for the `BlockItem` check.
pub fn hotbar_block_item(env: &dyn Env) -> Option<(usize, String)> {
    let deny = [
        "sword",
        "pickaxe",
        "axe",
        "shovel",
        "hoe",
        "bow",
        "arrow",
        "potion",
        "bucket",
        "food",
        "apple",
        "bread",
        "totem",
        "shield",
        "trident",
        "elytra",
        "firework",
        "pearl",
        "snowball",
        "egg",
        "rod",
        "shears",
        "flint_and_steel",
        "compass",
        "clock",
        "map",
        "book",
        "boat",
    ];
    for slot in env.inventory() {
        if slot.index >= 9 {
            continue;
        }
        let Some(stack) = slot.stack else { continue };
        if stack.is_empty() {
            continue;
        }
        let id = stack.id.strip_prefix("minecraft:").unwrap_or(&stack.id);
        if deny.iter().any(|d| id.contains(d)) {
            continue;
        }
        return Some((slot.index, stack.id.clone()));
    }
    None
}

/// Every block item id in the inventory with its slot, hotbar first.
pub fn block_items(env: &dyn Env) -> Vec<(usize, String)> {
    let deny = [
        "sword", "pickaxe", "axe", "shovel", "hoe", "bow", "arrow", "potion",
    ];
    let mut out = Vec::new();
    let mut slots = env.inventory();
    slots.sort_by_key(|s| if s.index < 9 { s.index } else { 9 + s.index });
    for slot in slots {
        let Some(stack) = slot.stack else { continue };
        if stack.is_empty() {
            continue;
        }
        let id = stack.id.strip_prefix("minecraft:").unwrap_or(&stack.id);
        if deny.iter().any(|d| id.contains(d)) {
            continue;
        }
        out.push((slot.index, stack.id.clone()));
    }
    out
}

/// `InvUtils.findInHotbar(Items.X)` - the hotbar slot holding `id`.
pub fn hotbar_slot_with(env: &dyn Env, id: &str) -> Option<usize> {
    env.inventory()
        .iter()
        .find(|s| s.index < 9 && s.stack.as_ref().map(|st| st.id == id).unwrap_or(false))
        .map(|s| s.index)
}

/// `InvUtils.find(Items.X)` - any inventory slot holding `id`.
pub fn inventory_slot_with(env: &dyn Env, id: &str) -> Option<usize> {
    env.inventory()
        .iter()
        .find(|s| s.stack.as_ref().map(|st| st.id == id).unwrap_or(false))
        .map(|s| s.index)
}

/// A `ServerboundMovePlayerPacket` (Pos / PosRot / Rot / StatusOnly).
pub fn is_move_player_packet(name: &str) -> bool {
    name.contains("MovePlayerPacket")
}

/// Reads a numeric packet field, as the packet modules do.
pub fn packet_f64(packet: &PacketInfo, key: &str) -> Option<f64> {
    packet
        .fields
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.parse::<f64>().ok())
}

/// Rewrites (or adds) a packet field.
pub fn set_packet_field(packet: &mut PacketInfo, key: &str, value: &str) {
    match packet.fields.iter_mut().find(|(k, _)| k == key) {
        Some(entry) => entry.1 = value.to_string(),
        None => packet.fields.push((key.to_string(), value.to_string())),
    }
}

/// Reads a boolean packet field written as Java's `Boolean.toString()`.
pub fn packet_bool(packet: &PacketInfo, key: &str) -> Option<bool> {
    packet
        .fields
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The registry order of `Modules.initMovement()`.
    const JAVA_ORDER: [&str; 31] = [
        "air-jump",
        "anchor",
        "anti-void",
        "auto-jump",
        "auto-walk",
        "auto-wasp",
        "blink",
        "click-tp",
        "elytra-boost",
        "elytra-fly",
        "entity-control",
        "fast-climb",
        "flight",
        "gui-move",
        "high-jump",
        "jesus",
        "long-jump",
        "no-fall",
        "no-slow",
        "parkour",
        "reverse-step",
        "safe-walk",
        "scaffold",
        "slippy",
        "sneak",
        "speed",
        "spider",
        "sprint",
        "step",
        "trident-boost",
        "velocity",
    ];

    #[test]
    fn registers_the_31_movement_modules_in_java_order() {
        let mut modules = Modules::new();
        register(&mut modules);
        assert_eq!(modules.count(), JAVA_ORDER.len());
        assert_eq!(modules.names(), JAVA_ORDER.to_vec());
        for name in JAVA_ORDER {
            let module = modules
                .get(name)
                .unwrap_or_else(|| panic!("{name} missing"));
            assert_eq!(
                module.base().category,
                crate::meteor_core::category::Category::Movement
            );
        }
    }

    #[test]
    fn every_module_has_a_description() {
        let mut modules = Modules::new();
        register(&mut modules);
        for module in modules.all() {
            assert!(
                !module.base().description.is_empty(),
                "{} has no description",
                module.base().name
            );
        }
    }

    #[test]
    fn horizontal_velocity_follows_the_keys() {
        use crate::meteor_core::env::test_env::TestEnv;
        let mut env = TestEnv::in_world();
        // yaw 0 looks towards +z, so W must push +z at bps/20.
        env.player.as_mut().unwrap().yaw = 0.0;
        env.keys = vec![keys::W];
        let v = horizontal_velocity(&env, 20.0);
        assert!((v.z - 1.0).abs() < 1.0e-6, "vel was {v:?}");
        assert!(v.x.abs() < 1.0e-6);
        assert!(is_moving(&env));
    }

    #[test]
    fn packet_helpers_round_trip() {
        let mut packet = PacketInfo {
            name: "ServerboundMovePlayerPacket.Pos".into(),
            clientbound: false,
            fields: vec![("onGround".into(), "false".into())],
        };
        assert_eq!(packet_bool(&packet, "onGround"), Some(false));
        set_packet_field(&mut packet, "onGround", "true");
        assert_eq!(packet_bool(&packet, "onGround"), Some(true));
        assert!(is_move_player_packet(&packet.name));
        set_packet_field(&mut packet, "x", "3.5");
        assert_eq!(packet_f64(&packet, "x"), Some(3.5));
    }
}

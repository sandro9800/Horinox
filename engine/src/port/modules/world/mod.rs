//! World modules (`systems.modules.world`).
//!
//! The registration order is `Modules.initWorld()`'s: Ambience ... VeinMiner, and
//! then the two Baritone-only modules, which `Modules.java` adds only when
//! Baritone is loaded. The port always registers them and lets them report that
//! Baritone is absent, so the registry stays identical to Meteor's.
//!
//! The helpers at the bottom are the shared parts of `BlockUtils`, `InvUtils`
//! and the `BlockState` predicates the world modules call. They live here
//! instead of in `meteor-core` because they are all reads of the `Env` surface:
//! no module invents an `Env` method, it just reuses the same maths.

use std::sync::Arc;

use crate::meteor_core::env::{Action, BlockInfo, Env, Hand, ItemStackInfo, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Dir, Rotation, Vec3d};
use crate::meteor_core::setting::{ItemListSet, ListSet, SettingGroup};

pub mod ambience;
pub mod auto_breed;
pub mod auto_brewer;
pub mod auto_mount;
pub mod auto_nametag;
pub mod auto_shearer;
pub mod auto_sign;
pub mod auto_smelter;
pub mod build_height;
pub mod collisions;
pub mod echest_farmer;
pub mod enderman_look;
pub mod excavator;
pub mod flamethrower;
pub mod highway_builder;
pub mod infinity_miner;
pub mod liquid_filler;
pub mod no_ghost_blocks;
pub mod nuker;
pub mod packet_mine;
pub mod spawn_proofer;
pub mod stash_finder;
pub mod timer;
pub mod vein_miner;

/// `BaritoneUtils.IS_AVAILABLE` for this build.
///
/// The Java client compiles Excavator and InfinityMiner against Baritone and
/// registers them only when the mod is present. Horizon has no pathfinding layer
/// to drive, so the check is a runtime function that always reports absent: the
/// two modules stay in the registry, still carry their settings, and say so when
/// they are switched on.
pub fn baritone_available() -> bool {
    // PORT GAP: BaritoneAPI (`IBaritone`, `IMineProcess`, `ICustomGoalProcess`,
    // `SelectionManager`). Keeping this a runtime predicate -- rather than a
    // cargo feature -- matches the Java client, which decides at load time.
    false
}

/// `Modules.initWorld()`.
pub fn register(modules: &mut Modules) {
    modules.add(Box::new(ambience::Ambience::new()));
    modules.add(Box::new(auto_breed::AutoBreed::new()));
    modules.add(Box::new(auto_brewer::AutoBrewer::new()));
    modules.add(Box::new(auto_mount::AutoMount::new()));
    modules.add(Box::new(auto_nametag::AutoNametag::new()));
    modules.add(Box::new(auto_shearer::AutoShearer::new()));
    modules.add(Box::new(auto_sign::AutoSign::new()));
    modules.add(Box::new(auto_smelter::AutoSmelter::new()));
    modules.add(Box::new(build_height::BuildHeight::new()));
    modules.add(Box::new(collisions::Collisions::new()));
    modules.add(Box::new(echest_farmer::EChestFarmer::new()));
    modules.add(Box::new(enderman_look::EndermanLook::new()));
    modules.add(Box::new(flamethrower::Flamethrower::new()));
    modules.add(Box::new(highway_builder::HighwayBuilder::new()));
    modules.add(Box::new(liquid_filler::LiquidFiller::new()));
    modules.add(Box::new(no_ghost_blocks::NoGhostBlocks::new()));
    modules.add(Box::new(nuker::Nuker::new()));
    modules.add(Box::new(packet_mine::PacketMine::new()));
    modules.add(Box::new(stash_finder::StashFinder::new()));
    modules.add(Box::new(spawn_proofer::SpawnProofer::new()));
    modules.add(Box::new(timer::Timer::new()));
    modules.add(Box::new(vein_miner::VeinMiner::new()));
    modules.add(Box::new(excavator::Excavator::new()));
    modules.add(Box::new(infinity_miner::InfinityMiner::new()));
}

// ---- settings helpers -----------------------------------------------------

/// The `.describe("...")` builder call, applied to the group's last setting.
///
/// `Settings::describe_last`/`set_visible_last` attach to the *last group*, which
/// is only right while a module builds one group at a time; taking the group
/// explicitly lets a module leave a group and still describe its settings.
pub fn describe(group: &mut SettingGroup, text: &str) {
    if let Some(setting) = group.settings.last_mut() {
        setting.description = text.to_string();
    }
}

/// The `.visible(() -> ...)` builder call, applied to the group's last setting.
pub fn visible(group: &mut SettingGroup, f: impl Fn() -> bool + Send + Sync + 'static) {
    if let Some(setting) = group.settings.last_mut() {
        setting.visible = Some(Arc::new(f));
    }
}

// ---- shared reads ---------------------------------------------------------

/// `ListMode.allows(boolean)`; `mode_is_whitelist` is the enum's index 0.
pub fn allows(mode_is_whitelist: bool, contains: bool) -> bool {
    if mode_is_whitelist {
        contains
    } else {
        !contains
    }
}

/// `BlockListSetting` membership, tolerant of an id typed without a namespace.
pub fn listed(list: &ListSet, id: &str) -> bool {
    list.contains(id) || list.contains(id.strip_prefix("minecraft:").unwrap_or(id))
}

/// `ItemListSetting` membership.
pub fn listed_item(list: &ItemListSet, id: &str) -> bool {
    list.contains(id) || list.contains(id.strip_prefix("minecraft:").unwrap_or(id))
}

/// `ItemStack.is(Item)` for one or more ids.
pub fn is_item(stack: &ItemStackInfo, ids: &[&str]) -> bool {
    ids.iter().any(|id| {
        if stack.id.eq_ignore_ascii_case(id) {
            return true;
        }
        match (
            stack.id.strip_prefix("minecraft:"),
            id.strip_prefix("minecraft:"),
        ) {
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            (Some(a), None) => a.eq_ignore_ascii_case(id),
            (None, Some(b)) => stack.id.eq_ignore_ascii_case(b),
            (None, None) => false,
        }
    })
}

/// `BlockState.canBeReplaced()`: air, fluids and the passable decorations the
/// world modules actually meet.
pub fn replaceable(block: &BlockInfo) -> bool {
    const REPLACEABLE: [&str; 16] = [
        "minecraft:water",
        "minecraft:lava",
        "minecraft:short_grass",
        "minecraft:grass",
        "minecraft:fern",
        "minecraft:large_fern",
        "minecraft:dead_bush",
        "minecraft:seagrass",
        "minecraft:tall_seagrass",
        "minecraft:snow",
        "minecraft:fire",
        "minecraft:soul_fire",
        "minecraft:vine",
        "minecraft:structure_void",
        "minecraft:crimson_roots",
        "minecraft:warped_roots",
    ];
    block.is_air() || REPLACEABLE.iter().any(|id| block.is(id))
}

/// `BlockState.getFluidState().getType()` for a *source* block.
pub fn liquid_source(block: &BlockInfo) -> Option<&'static str> {
    let kind = if block.is("minecraft:water") {
        "water"
    } else if block.is("minecraft:lava") {
        "lava"
    } else {
        return None;
    };
    // `level=0` is the source; running water/lava carries 1..7.
    match block.prop("level") {
        None | Some("0") => Some(kind),
        _ => None,
    }
}

/// `BlockUtils.canBreak`: not air, not a fluid, and not one of the blocks the
/// server never lets go.
pub fn can_break(block: &BlockInfo) -> bool {
    const UNBREAKABLE: [&str; 12] = [
        "minecraft:bedrock",
        "minecraft:barrier",
        "minecraft:light",
        "minecraft:command_block",
        "minecraft:chain_command_block",
        "minecraft:repeating_command_block",
        "minecraft:structure_block",
        "minecraft:structure_void",
        "minecraft:jigsaw",
        "minecraft:end_portal_frame",
        "minecraft:reinforced_deepslate",
        "minecraft:moving_piston",
    ];
    !block.is_air()
        && !block.is("minecraft:water")
        && !block.is("minecraft:lava")
        && !UNBREAKABLE.iter().any(|id| block.is(id))
}

/// `BlockUtils.canPlace`: the position is replaceable and has a solid neighbour
/// to place against.
pub fn can_place(env: &dyn Env, pos: BlockPos) -> bool {
    let Some(here) = env.block(pos) else {
        return false;
    };
    if !replaceable(&here) {
        return false;
    }
    BlockPos::NEIGHBORS
        .into_iter()
        .any(|dir| match env.block(pos.offset_dir(dir)) {
            Some(b) => !replaceable(&b) && !b.is_air(),
            None => false,
        })
}

/// `InvUtils.findInHotbar(itemStack -> ...)`: the hotbar slot holding one of
/// `ids`, if any.
pub fn hotbar_slot(env: &dyn Env, ids: &[&str]) -> Option<usize> {
    env.inventory()
        .into_iter()
        .find(|slot| {
            slot.index < 9
                && slot
                    .stack
                    .as_ref()
                    .is_some_and(|stack| !stack.is_empty() && is_item(stack, ids))
        })
        .map(|slot| slot.index)
}

/// `InvUtils.findInHotbar(itemStack -> blocks.contains(Block.byItem(itemStack)))`.
pub fn hotbar_block_slot(env: &dyn Env, list: &ListSet) -> Option<usize> {
    env.inventory()
        .into_iter()
        .filter(|slot| slot.index < 9)
        .find(|slot| {
            slot.stack
                .as_ref()
                .is_some_and(|stack| listed(list, &stack.id))
        })
        .map(|slot| slot.index)
}

/// The stack in one slot, if any.
pub fn stack_in(env: &dyn Env, slot: usize) -> Option<ItemStackInfo> {
    env.inventory()
        .into_iter()
        .find(|s| s.index == slot)
        .and_then(|s| s.stack)
}

/// `BlockUtils.breakBlock(pos, swing)`.
pub fn break_block(env: &mut dyn Env, pos: BlockPos) {
    rotate_to(env, pos);
    // PORT GAP: block breaking as a player. `BlockUtils.breakBlock` sends the
    // START/STOP destroy packets with the player's sequence id; the Env only has
    // the generic attack action, which acts on the crosshair target.
    env.action(Action::Attack);
}

/// `BlockUtils.place(pos, item, rotate, ...)`.
pub fn place_block(env: &mut dyn Env, pos: BlockPos, state: &str) -> bool {
    rotate_to(env, pos);
    env.set_block(pos, state)
}

/// `Rotations.rotate(Rotations.getYaw(pos), Rotations.getPitch(pos), ...)`.
pub fn rotate_to(env: &mut dyn Env, pos: BlockPos) {
    rotate_to_pos(env, pos.center());
}

/// `Rotations.rotate(yaw, pitch, priority, null)` towards an arbitrary point.
pub fn rotate_to_pos(env: &mut dyn Env, at: Vec3d) {
    if let Some(player) = env.player() {
        env.set_rotation(Rotation::between(player.eye_pos(), at));
    }
}

/// `entity.getBoundingBox().getCenter()`, the point Meteor aims at.
pub fn entity_center(entity: &crate::meteor_core::env::EntityInfo) -> Vec3d {
    let (min, max) = entity.bounding_box();
    Vec3d::new(
        (min.x + max.x) / 2.0,
        (min.y + max.y) / 2.0,
        (min.z + max.z) / 2.0,
    )
}

/// `PlayerUtils.isWithin(entity, range)`.
pub fn entity_within(
    env: &dyn Env,
    entity: &crate::meteor_core::env::EntityInfo,
    range: f64,
) -> bool {
    match env.player() {
        Some(player) => player.pos.distance_to(entity.pos) <= range,
        None => false,
    }
}

/// `mc.gameMode.interact(mc.player, entity, hitResult, hand)`.
pub fn interact_entity(env: &mut dyn Env, at: Vec3d, rotate: bool) {
    if rotate {
        rotate_to_pos(env, at);
    }
    // PORT GAP: `MultiPlayerGameMode.interact(player, entity, hitResult, hand)`.
    // The Env exposes `action(Action::Use)`, which uses the main hand on whatever
    // the crosshair is on: there is no per-entity interact call with an explicit
    // hit result, and no `InvUtils.swap` slot-swap helper either.
    env.action(Action::Use);
}

/// `PlayerUtils.isWithin(pos, range)` for a block position's centre.
pub fn is_within(env: &dyn Env, pos: BlockPos, range: f64) -> bool {
    match env.player() {
        Some(player) => player.pos.distance_to(pos.center()) <= range,
        None => false,
    }
}

// ---- packet helpers -------------------------------------------------------

/// One field of a decoded packet, as text.
pub fn packet_field(packet: &PacketInfo, key: &str) -> Option<String> {
    packet
        .fields
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.clone())
}

/// Rewrites one field of a decoded packet (`BlockHitResultAccessor`-style edits).
pub fn set_packet_field(packet: &mut PacketInfo, key: &str, value: &str) {
    if let Some(field) = packet
        .fields
        .iter_mut()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
    {
        field.1 = value.to_string();
    } else {
        packet.fields.push((key.to_string(), value.to_string()));
    }
}

/// The block position a packet carries as `x`, `y`, `z`.
pub fn packet_pos(packet: &PacketInfo) -> Option<BlockPos> {
    let x = packet_field(packet, "x")?.parse::<i32>().ok()?;
    let y = packet_field(packet, "y")?.parse::<i32>().ok()?;
    let z = packet_field(packet, "z")?.parse::<i32>().ok()?;
    Some(BlockPos::new(x, y, z))
}

/// A `PacketInfo` carrying a block position, for the modules that answer the
/// server (`env.send_packet`).
pub fn pos_packet(name: &str, pos: BlockPos) -> PacketInfo {
    PacketInfo {
        name: name.to_string(),
        clientbound: false,
        fields: vec![
            ("x".to_string(), pos.x.to_string()),
            ("y".to_string(), pos.y.to_string()),
            ("z".to_string(), pos.z.to_string()),
        ],
    }
}

/// `ServerboundPlayerActionPacket`'s name in Horizon's packet stream.
pub const PLAYER_ACTION: &str = "player_action";
/// `ServerboundUseItemOnPacket`'s name in Horizon's packet stream.
pub const USE_ITEM_ON: &str = "use_item_on";

/// The `player_action` packet's action field, lower-cased
/// (`start_destroy_block` and friends).
pub fn player_action_kind(packet: &PacketInfo) -> Option<String> {
    packet_field(packet, "action").map(|a| a.to_ascii_lowercase())
}

/// A face name from a packet field (`Direction.getName()`).
pub fn face_from_name(name: &str) -> Dir {
    match name.to_ascii_lowercase().as_str() {
        "down" => Dir::Down,
        "north" => Dir::North,
        "south" => Dir::South,
        "west" => Dir::West,
        "east" => Dir::East,
        _ => Dir::Up,
    }
}

/// `BlockUtils.interact(hitResult, hand, swing)`.
pub fn interact_block(env: &mut dyn Env, pos: BlockPos, face: Dir) {
    rotate_to(env, pos);
    env.use_item_on(pos, face, Hand::Main);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::Dir as Face;

    #[test]
    fn every_world_module_is_registered_in_modules_java_order() {
        let mut modules = Modules::new();
        register(&mut modules);
        let names = modules.names();
        assert_eq!(names.len(), 24, "24 world modules, Baritone ones included");
        assert_eq!(names[0], "ambience");
        assert_eq!(names[1], "auto-breed");
        assert_eq!(names[7], "auto-smelter");
        assert_eq!(names[10], "echest-farmer");
        assert_eq!(names[13], "highway-builder");
        assert_eq!(names[21], "vein-miner");
        assert_eq!(names[22], "excavator");
        assert_eq!(names[23], "infinity-miner");
        assert!(modules.get("vein-miner").is_some());
        assert_eq!(modules.all().next().unwrap().base().name, "ambience");
    }

    #[test]
    fn baritone_is_absent() {
        assert!(!baritone_available());
    }

    #[test]
    fn block_predicates_match_the_java_utilities() {
        assert!(replaceable(&BlockInfo::new("minecraft:air")));
        assert!(replaceable(&BlockInfo::new("minecraft:water")));
        assert!(!replaceable(&BlockInfo::new("minecraft:stone")));

        assert_eq!(
            liquid_source(&BlockInfo::new("minecraft:water[level=0]")),
            Some("water")
        );
        assert_eq!(
            liquid_source(&BlockInfo::new("minecraft:water[level=3]")),
            None
        );
        assert_eq!(
            liquid_source(&BlockInfo::new("minecraft:lava")),
            Some("lava")
        );
        assert_eq!(liquid_source(&BlockInfo::new("minecraft:stone")), None);

        assert!(can_break(&BlockInfo::new("minecraft:stone")));
        assert!(!can_break(&BlockInfo::new("minecraft:bedrock")));
        assert!(!can_break(&BlockInfo::new("minecraft:air")));
        assert!(!can_break(&BlockInfo::new("minecraft:water")));

        assert!(is_item(
            &ItemStackInfo {
                id: "minecraft:diamond_pickaxe".into(),
                count: 1,
                ..Default::default()
            },
            &["diamond_pickaxe"]
        ));
        assert!(
            allows(false, false),
            "blacklist allows what it does not list"
        );
        assert!(!allows(false, true));
        assert!(allows(true, true), "whitelist allows only what it lists");
    }

    #[test]
    fn can_place_needs_a_solid_neighbour() {
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(1, 64, 0);
        env.push_block(pos, "minecraft:air");
        assert!(!can_place(&env, pos), "no neighbour to place against");
        env.push_block(BlockPos::new(1, 63, 0), "minecraft:stone");
        assert!(can_place(&env, pos));
    }

    #[test]
    fn packets_round_trip_positions_and_fields() {
        let mut packet = pos_packet(PLAYER_ACTION, BlockPos::new(1, 2, 3));
        assert_eq!(packet_pos(&packet), Some(BlockPos::new(1, 2, 3)));
        set_packet_field(&mut packet, "action", "START_DESTROY_BLOCK");
        assert_eq!(
            player_action_kind(&packet).as_deref(),
            Some("start_destroy_block")
        );
        set_packet_field(&mut packet, "face", "down");
        assert_eq!(
            face_from_name(&packet_field(&packet, "face").unwrap()),
            Face::Down
        );
        set_packet_field(&mut packet, "fresh", "value");
        assert_eq!(packet_field(&packet, "fresh").as_deref(), Some("value"));
    }

    #[test]
    fn hotbar_helpers_read_the_inventory() {
        let mut env = TestEnv::in_world();
        env.inventory = vec![
            crate::meteor_core::env::InvSlot {
                index: 0,
                stack: Some(ItemStackInfo {
                    id: "minecraft:stone".into(),
                    count: 64,
                    ..Default::default()
                }),
            },
            crate::meteor_core::env::InvSlot {
                index: 1,
                stack: Some(ItemStackInfo {
                    id: "minecraft:shears".into(),
                    count: 1,
                    ..Default::default()
                }),
            },
        ];
        assert_eq!(hotbar_slot(&env, &["minecraft:shears"]), Some(1));
        assert_eq!(hotbar_slot(&env, &["minecraft:netherite_pickaxe"]), None);
        let mut settings = crate::meteor_core::setting::Settings::new();
        let list = settings
            .default_group()
            .list_setting("blocks", &["minecraft:stone"]);
        assert_eq!(hotbar_block_slot(&env, &list), Some(0));
        assert_eq!(
            stack_in(&env, 1).map(|s| s.id),
            Some("minecraft:shears".to_string())
        );
    }
}

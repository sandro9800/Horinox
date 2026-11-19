//! `systems.commands.LocateCommand` -?`.locate (buried_treasure|mansion|monument|
//! stronghold|nether_fortress|end_city|lodestone|cancel)`.
//!
//! Two mechanisms in the Java original:
//!
//! * maps and compasses -?the decoration's coordinates come out of the held
//!   item's data components;
//! * `BaritoneAPI.getProvider().getWorldScanner().scanChunkRadius(...)` -?Baritone
//!   scans loaded chunks for the structure's blocks.
//!
//! Horizon exposes neither item components nor Baritone, so the port reads blocks
//! through `Env::block` over a local box around the player and says so. The
//! eye-of-ender stronghold triangle is ported in full: `state::locate_eye_spawn` /
//! `locate_eye_land` are driven by [`on_receive`], which is `PacketEvent.Receive`
//! plus `EntityRemovedEvent` in Java.
//!
//! PORT GAP: `DataComponents.ITEM_NAME` / `MAP_DECORATIONS` / `LODESTONE_TRACKER`
//! and `PathManagers.follow(EyeOfEnder.class::isInstance)`.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;
use crate::meteor_core::env::PacketInfo;
use crate::meteor_core::math::{BlockPos, Vec3d};

use crate::port::commands::{error, selected, state};

pub struct LocateCommand;

const NETHER_FORTRESS: &[&str] = &[
    "minecraft:nether_bricks",
    "minecraft:nether_brick_fence",
    "minecraft:nether_wart",
];
const MONUMENT: &[&str] = &[
    "minecraft:prismarine_bricks",
    "minecraft:sea_lantern",
    "minecraft:dark_prismarine",
];
const STRONGHOLD: &[&str] = &["minecraft:end_portal_frame"];
const END_CITY: &[&str] = &[
    "minecraft:purpur_block",
    "minecraft:purpur_pillar",
    "minecraft:purpur_slab",
    "minecraft:purpur_stairs",
    "minecraft:end_stone_bricks",
    "minecraft:end_rod",
];

/// The box `findByBlockList` searches, in blocks around the player (Baritone
/// scans a chunk radius; the port keeps a smaller local box).
const SCAN_RADIUS: i32 = 16;

impl Command for LocateCommand {
    fn name(&self) -> &'static str {
        "locate"
    }

    fn description(&self) -> &'static str {
        "Locates structures"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["loc"]
    }

    fn usage(&self) -> String {
        "locate (buried_treasure|mansion|monument|stronghold|nether_fortress|end_city|lodestone|cancel)"
            .to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(what) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new("Specify a structure to locate."));
        };

        match what.as_str() {
            "buried_treasure" => map_locate(ctx, "buried treasure map"),
            "mansion" => map_locate(ctx, "woodland mansion map"),
            "monument" => {
                let holding_map = selected(ctx)
                    .map(|stack| stack.id == "minecraft:filled_map")
                    .unwrap_or(false);
                if holding_map {
                    // The map's own name decides whether it is an ocean monument
                    // map; the port cannot read it (see the module doc).
                    error(ctx, "locate", "Couldn't locate the map icons!");
                    return Ok(());
                }
                report(
                    ctx,
                    MONUMENT,
                    "Monument",
                    "No monument found. Try using an (highlight)ocean monument map(default) for \
                     more success.",
                )
            }
            "stronghold" => stronghold(ctx),
            "nether_fortress" => {
                if !dimension(ctx, "the_nether") {
                    error(
                        ctx,
                        "locate",
                        "You need to be in the nether to locate a nether fortress.",
                    );
                    return Ok(());
                }
                report(
                    ctx,
                    NETHER_FORTRESS,
                    "Fortress",
                    "No nether fortress found.",
                )
            }
            "end_city" => {
                if !dimension(ctx, "the_end") {
                    error(
                        ctx,
                        "locate",
                        "You need to be in the end to locate an end city.",
                    );
                    return Ok(());
                }
                report(ctx, END_CITY, "End city", "No end city found.")
            }
            "lodestone" => lodestone(ctx),
            "cancel" => {
                state::locate_reset();
                ctx.warn("Locate canceled");
                Ok(())
            }
            other => Err(CmdError::new(format!(
                "{} is not a structure to locate.",
                other
            ))),
        }
    }
}

/// `mc.level.dimension()` ends with the vanilla dimension path.
fn dimension(ctx: &CmdCtx, path: &str) -> bool {
    ctx.env
        .world()
        .map(|w| w.dimension.ends_with(path))
        .unwrap_or(false)
}

/// `findByBlockList` + the `info("-?located at -?)` line.
fn report(ctx: &mut CmdCtx, ids: &[&str], label: &str, missing: &str) -> CmdResult {
    let found = scan_blocks(ctx, ids);
    if found.is_empty() {
        error(ctx, "locate", missing);
        return Ok(());
    }
    if found.len() < 3 {
        ctx.warn(&format!(
            "Only {} block(s) found. This search might be a false positive.",
            found.len()
        ));
    }
    let pos = found[0];
    ctx.info(&format!(
        "{} located at X: {}, Y: {}, Z: {}.",
        label, pos.x, pos.y, pos.z
    ));
    Ok(())
}

/// The nearest matching block within [`SCAN_RADIUS`], nearest first.
fn scan_blocks(ctx: &CmdCtx, ids: &[&str]) -> Vec<BlockPos> {
    let Some(player) = ctx.env.player() else {
        return Vec::new();
    };
    let origin = player.pos.to_block_pos();
    let mut found = Vec::new();
    for dx in -SCAN_RADIUS..=SCAN_RADIUS {
        for dy in -SCAN_RADIUS..=SCAN_RADIUS {
            for dz in -SCAN_RADIUS..=SCAN_RADIUS {
                let pos = origin.offset(dx, dy, dz);
                let Some(block) = ctx.env.block(pos) else {
                    continue;
                };
                if ids.iter().any(|id| block.is(id)) {
                    found.push(pos);
                }
            }
        }
    }
    found.sort_by(|a, b| {
        a.distance(origin)
            .partial_cmp(&b.distance(origin))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    found
}

/// The map-based structures: the held map and its decorations.
fn map_locate(ctx: &mut CmdCtx, map_name: &str) -> CmdResult {
    let holding_map = selected(ctx)
        .map(|stack| stack.id == "minecraft:filled_map")
        .unwrap_or(false);
    if !holding_map {
        error(
            ctx,
            "locate",
            &format!("You need to hold a (highlight){}(default)!", map_name),
        );
        return Ok(());
    }
    // PORT GAP: the map's `ITEM_NAME` and `MAP_DECORATIONS` components are not in
    // `ItemStackInfo`, so the decorations' coordinates cannot be read.
    error(ctx, "locate", "Couldn't locate the map icons!");
    Ok(())
}

fn lodestone(ctx: &mut CmdCtx) -> CmdResult {
    let holding_compass = selected(ctx)
        .map(|stack| stack.id == "minecraft:compass")
        .unwrap_or(false);
    if !holding_compass {
        error(
            ctx,
            "locate",
            "You need to hold a (highlight)lodestone(default) compass!",
        );
        return Ok(());
    }
    // PORT GAP: `DataComponents.LODESTONE_TRACKER`.
    error(
        ctx,
        "locate",
        "Couldn't get the components data. Are you holding a (highlight)lodestone(default) \
         compass?",
    );
    Ok(())
}

/// `.locate stronghold`: with an eye of ender the command waits for the two
/// throws; otherwise it scans for an end portal frame (Baritone's job in Java).
fn stronghold(ctx: &mut CmdCtx) -> CmdResult {
    let has_eye = ctx.env.inventory().into_iter().any(|slot| {
        slot.index < 9
            && slot
                .stack
                .map(|stack| stack.id == "minecraft:ender_eye")
                .unwrap_or(false)
    });
    if has_eye {
        state::locate_reset();
        ctx.info("Please throw the first Eye of Ender");
        // PORT GAP: `PathManagers.get().follow(EyeOfEnder.class::isInstance)`.
        return Ok(());
    }
    report(
        ctx,
        STRONGHOLD,
        "Stronghold",
        "No stronghold found nearby. You can use (highlight)Ender Eyes(default) for more success.",
    )
}

/// `PacketEvent.Receive` (`ClientboundAddEntityPacket` of an `EyeOfEnder`) and
/// `EntityRemovedEvent`: the stronghold triangle.
///
/// Returns true when the packet was one of the eye's.
pub fn on_receive(env: &mut dyn Env, packet: &PacketInfo) -> bool {
    let is_eye = packet
        .fields
        .iter()
        .any(|(_, value)| value.contains("eye_of_ender"));
    if !is_eye {
        return false;
    }
    let Some(pos) = position(packet) else {
        return false;
    };
    if packet.name.contains("AddEntity") || packet.name.contains("EntityAdded") {
        state::locate_eye_spawn(pos);
        return true;
    }
    if packet.name.contains("RemoveEntities") || packet.name.contains("EntityRemoved") {
        match state::locate_eye_land(pos) {
            state::LocateOutcome::AwaitSecond => {
                env.chat("Please throw the second Eye Of Ender from a different location.");
            }
            state::LocateOutcome::Located(hit) => {
                env.chat(&format!(
                    "Stronghold roughly located at X: {}, Y: 0, Z: {}.",
                    hit.x as i32, hit.z as i32
                ));
            }
            state::LocateOutcome::Failed => {
                env.chat("Unable to calculate intersection.");
                state::locate_reset();
            }
        }
        return true;
    }
    false
}

fn position(packet: &PacketInfo) -> Option<Vec3d> {
    let field = |key: &str| {
        packet
            .fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .and_then(|(_, v)| v.parse::<f64>().ok())
    };
    Some(Vec3d::new(field("x")?, field("y")?, field("z")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};
    use crate::port::commands::command_manager;

    fn holding(id: &str) -> TestEnv {
        let mut env = TestEnv::in_world();
        env.inventory = vec![InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: id.to_string(),
                count: 1,
                ..Default::default()
            }),
        }];
        env
    }

    #[test]
    fn locate_scans_the_local_area() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        env.world.as_mut().unwrap().dimension = "minecraft:the_nether".into();
        env.push_block(BlockPos::new(3, 64, 3), "minecraft:nether_bricks");
        assert_eq!(
            m.dispatch(&mut env, ".locate nether_fortress"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Fortress located at X: 3, Y: 64, Z: 3.")));
        // Fewer than three matching blocks: Meteor warns about a false positive.
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("might be a false positive")));

        let mut env = TestEnv::in_world();
        env.world.as_mut().unwrap().dimension = "minecraft:the_end".into();
        env.push_block(BlockPos::new(0, 64, 2), "minecraft:purpur_block");
        assert_eq!(m.dispatch(&mut env, ".loc l"), Dispatch::Failed); // `.loc` parses `l`
        assert_eq!(m.dispatch(&mut env, ".loc end_city"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("End city located at X: 0, Y: 64, Z: 2.")));
    }

    #[test]
    fn locate_dimension_and_map_gaps() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(
            m.dispatch(&mut env, ".locate nether_fortress"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("You need to be in the nether to locate a nether fortress.")));
        assert_eq!(m.dispatch(&mut env, ".locate end_city"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("You need to be in the end to locate an end city.")));

        // No map in hand.
        assert_eq!(
            m.dispatch(&mut env, ".locate buried_treasure"),
            Dispatch::Handled
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("You need to hold a") && c.contains("buried treasure map")));

        // A filled map can be held, but its decorations are unreachable.
        let mut env = holding("minecraft:filled_map");
        assert_eq!(m.dispatch(&mut env, ".locate mansion"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Couldn't locate the map icons!")));

        // A lodestone compass needs the tracker component.
        let mut env = holding("minecraft:compass");
        assert_eq!(m.dispatch(&mut env, ".locate lodestone"), Dispatch::Handled);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Couldn't get the components data")));
    }

    #[test]
    fn locate_cancel_and_errors() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".locate cancel"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Locate canceled")));
        assert_eq!(m.dispatch(&mut env, ".locate"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Specify a structure to locate.")));
        assert_eq!(m.dispatch(&mut env, ".locate atlantis"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("atlantis is not a structure to locate.")));
    }

    #[test]
    fn locate_stronghold_triangle_through_the_packet_hook() {
        state::locate_reset();
        let mut env = TestEnv::in_world();
        let eye = |name: &str, x: f64, z: f64| PacketInfo {
            name: name.to_string(),
            clientbound: true,
            fields: vec![
                ("entity".to_string(), "minecraft:eye_of_ender".to_string()),
                ("x".to_string(), format!("{x}")),
                ("y".to_string(), "64".to_string()),
                ("z".to_string(), format!("{z}")),
            ],
        };
        assert!(on_receive(
            &mut env,
            &eye("ClientboundAddEntityPacket", 0.0, 0.0)
        ));
        assert!(on_receive(
            &mut env,
            &eye("ClientboundRemoveEntitiesPacket", 10.0, 0.0)
        ));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Please throw the second Eye Of Ender")));
        assert!(on_receive(
            &mut env,
            &eye("ClientboundAddEntityPacket", 10.0, 10.0)
        ));
        assert!(on_receive(
            &mut env,
            &eye("ClientboundRemoveEntitiesPacket", 10.0, 0.0)
        ));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Stronghold roughly located at X: 10, Y: 0, Z: 0.")));
        state::locate_reset();
    }
}

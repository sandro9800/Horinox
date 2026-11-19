//! `systems.commands.EnchantCommand` -?`.enchant --, creative only.
//!
//! Java edits the held stack's `DataComponentMap` (add/set/remove/clear) and then
//! opens and closes an `InventoryScreen` to push the change to the server. Horizon
//! has no component objects and no screen-open call, so the port composes the
//! component patch itself and sends it in `ServerboundSetCreativeModeSlotPacket`,
//! which is the packet Java's `syncItem` path ends up producing.
//!
//! PORT GAP: `Utils.addEnchantment` / `removeEnchantment` / `clearEnchantments`
//! mutate a live `ItemStack`'s `PatchedDataComponentMap`; there is no client-side
//! stack write in Horizon's mod API. The item registry lookup
//! (`registryAccess().lookup(Registries.ENCHANTMENT)`) is replaced by the vanilla
//! table below, so `all_possible` matches on the item's name instead of its
//! `isSupportedItem` tag.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;
use crate::meteor_core::env::ItemStackInfo;

use crate::port::commands::{creative, creative_slot, creative_slot_packet, selected};

pub struct EnchantCommand;

/// `(id, max level, item kinds it may go on)`: the vanilla enchantment set.
const ENCHANTMENTS: &[(&str, i32, &[&str])] = &[
    ("minecraft:sharpness", 5, &["sword", "axe"]),
    ("minecraft:smite", 5, &["sword", "axe"]),
    ("minecraft:bane_of_arthropods", 5, &["sword", "axe"]),
    ("minecraft:knockback", 2, &["sword"]),
    ("minecraft:fire_aspect", 2, &["sword"]),
    ("minecraft:looting", 3, &["sword"]),
    ("minecraft:sweeping_edge", 3, &["sword"]),
    ("minecraft:unbreaking", 3, &["*"]),
    ("minecraft:mending", 1, &["*"]),
    ("minecraft:vanishing_curse", 1, &["*"]),
    (
        "minecraft:protection",
        4,
        &["helmet", "chestplate", "leggings", "boots"],
    ),
    (
        "minecraft:fire_protection",
        4,
        &["helmet", "chestplate", "leggings", "boots"],
    ),
    (
        "minecraft:blast_protection",
        4,
        &["helmet", "chestplate", "leggings", "boots"],
    ),
    (
        "minecraft:projectile_protection",
        4,
        &["helmet", "chestplate", "leggings", "boots"],
    ),
    ("minecraft:thorns", 3, &["chestplate"]),
    ("minecraft:respiration", 3, &["helmet"]),
    ("minecraft:aqua_affinity", 1, &["helmet"]),
    (
        "minecraft:efficiency",
        5,
        &["pickaxe", "axe", "shovel", "hoe", "shears"],
    ),
    ("minecraft:fortune", 3, &["pickaxe", "axe", "shovel", "hoe"]),
    (
        "minecraft:silk_touch",
        1,
        &["pickaxe", "axe", "shovel", "hoe"],
    ),
    ("minecraft:power", 5, &["bow"]),
    ("minecraft:punch", 2, &["bow"]),
    ("minecraft:flame", 1, &["bow"]),
    ("minecraft:infinity", 1, &["bow"]),
    ("minecraft:luck_of_the_sea", 3, &["fishing_rod"]),
    ("minecraft:lure", 3, &["fishing_rod"]),
    ("minecraft:loyalty", 3, &["trident"]),
    ("minecraft:impaling", 5, &["trident"]),
    ("minecraft:riptide", 3, &["trident"]),
    ("minecraft:channeling", 1, &["trident"]),
    ("minecraft:multishot", 1, &["crossbow"]),
    ("minecraft:quick_charge", 3, &["crossbow"]),
    ("minecraft:piercing", 4, &["crossbow"]),
    ("minecraft:depth_strider", 3, &["boots"]),
    ("minecraft:frost_walker", 2, &["boots"]),
    ("minecraft:soul_speed", 3, &["boots"]),
    ("minecraft:swift_sneak", 3, &["leggings"]),
];

impl Command for EnchantCommand {
    fn name(&self) -> &'static str {
        "enchant"
    }

    fn description(&self) -> &'static str {
        "Enchants the item in your hand. REQUIRES Creative mode."
    }

    fn usage(&self) -> String {
        "enchant (one <enchantment> (level <level>|max)|all_possible (level <level>|max)|all \
         (level <level>|max)|clear|remove <enchantment>)"
            .to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(mode) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new("Specify what to enchant."));
        };

        match mode.as_str() {
            "one" => {
                let name = ctx
                    .arg(1)
                    .ok_or_else(|| CmdError::new("Specify an enchantment."))?
                    .to_string();
                let entry = lookup(&name)?;
                let level = level_argument(ctx, 2, entry.1)?;
                apply(ctx, &[(entry.0.to_string(), level)], &name)
            }
            "all_possible" | "all" => {
                let only_possible = mode == "all_possible";
                let stack = held(ctx)?;
                let levels = if ctx.arg(1).map(|a| a.eq_ignore_ascii_case("level")) == Some(true) {
                    let level = ctx.int(2)?;
                    Level::Fixed(level)
                } else if ctx.arg(1).map(|a| a.eq_ignore_ascii_case("max")) == Some(true) {
                    Level::Max
                } else {
                    return Err(CmdError::new("Use level <level> or max."));
                };
                let mut patch: Vec<(String, i32)> = Vec::new();
                for (id, max, kinds) in ENCHANTMENTS {
                    if only_possible && !supports(&stack.id, kinds) {
                        continue;
                    }
                    patch.push((id.to_string(), levels.of(*max)));
                }
                apply(ctx, &patch, &mode)
            }
            "clear" => apply(ctx, &[], "clear"),
            "remove" => {
                let name = ctx
                    .arg(1)
                    .ok_or_else(|| CmdError::new("Specify an enchantment."))?
                    .to_string();
                let entry = lookup(&name)?;
                // A removal is a patch entry without a level.
                apply_removal(ctx, entry.0)
            }
            _ => Err(CmdError::new(
                "Use one, all_possible, all, clear or remove.".to_string(),
            )),
        }
    }
}

enum Level {
    Max,
    Fixed(i32),
}

impl Level {
    fn of(&self, max: i32) -> i32 {
        match self {
            Level::Max => max,
            Level::Fixed(v) => *v,
        }
    }
}

fn level_argument(ctx: &CmdCtx, first: usize, max: i32) -> Result<i32, CmdError> {
    match ctx.arg(first) {
        Some(word) if word.eq_ignore_ascii_case("max") => Ok(max),
        Some(word) if word.eq_ignore_ascii_case("level") => ctx.int(first + 1),
        _ => Err(CmdError::new("Use level <level> or max.".to_string())),
    }
}

/// `RegistryEntryReferenceArgumentType.enchantment()`: accept a bare or
/// namespaced id and report the miss the way Brigadier's unknown-argument does.
fn lookup(name: &str) -> Result<(&'static str, i32), CmdError> {
    let wanted = if name.contains(':') {
        name.to_ascii_lowercase()
    } else {
        format!("minecraft:{}", name.to_ascii_lowercase())
    };
    ENCHANTMENTS
        .iter()
        .find(|(id, _, _)| *id == wanted)
        .map(|(id, max, _)| (*id, *max))
        .ok_or_else(|| CmdError::new(format!("Unknown enchantment {}.", name)))
}

/// `Enchantment.isSupportedItem` approximated from the item's name.
fn supports(item_id: &str, kinds: &[&str]) -> bool {
    let path = item_id.strip_prefix("minecraft:").unwrap_or(item_id);
    kinds.iter().any(|kind| *kind == "*" || path.contains(kind))
}

/// `tryGetItemStack`: creative, and holding something.
fn held(ctx: &CmdCtx) -> Result<ItemStackInfo, CmdError> {
    creative(ctx)?;
    selected(ctx).ok_or_else(|| CmdError::new("You need to hold some item to enchant."))
}

/// `one`/`all`: write the enchantment map back in a creative-slot packet.
fn apply(ctx: &mut CmdCtx, patch: &[(String, i32)], what: &str) -> CmdResult {
    let stack = held(ctx)?;
    let pairs: Vec<String> = patch.iter().map(|(id, l)| format!("{id}:{l}")).collect();
    sync_item(
        ctx,
        &stack,
        &format!("{{enchantments:{{{}}}}}", pairs.join(",")),
    );
    ctx.info(&format!(
        "Enchanted {} with {}.",
        stack.display_name(),
        what
    ));
    Ok(())
}

fn apply_removal(ctx: &mut CmdCtx, id: &str) -> CmdResult {
    let stack = held(ctx)?;
    sync_item(ctx, &stack, &format!("{{enchantments:{{-{id}}}}}"));
    ctx.info(&format!("Removed {} from {}.", id, stack.display_name()));
    Ok(())
}

/// `syncItem`: Java opens and closes `InventoryScreen`; the packet carries the
/// component patch here. PORT GAP: no screen-open call in Horizon's mod API.
fn sync_item(ctx: &mut CmdCtx, stack: &ItemStackInfo, components: &str) {
    let slot = ctx.env.player().map(|p| creative_slot(&p)).unwrap_or(36);
    let packet = creative_slot_packet(
        slot,
        &stack.id,
        stack.count.max(1),
        vec![("components".to_string(), components.to_string())],
    );
    ctx.env.send_packet(packet);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::meteor_core::env::{GameMode, InvSlot, ItemStackInfo};
    use crate::port::commands::command_manager;

    fn creative_env(item: &str) -> TestEnv {
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().game_mode = GameMode::Creative;
        env.inventory = vec![InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: item.to_string(),
                count: 1,
                ..Default::default()
            }),
        }];
        env
    }

    fn components(env: &TestEnv) -> String {
        env.packets
            .last()
            .and_then(|p| {
                p.fields
                    .iter()
                    .find(|(k, _)| k == "components")
                    .map(|(_, v)| v.clone())
            })
            .unwrap_or_default()
    }

    #[test]
    fn enchant_one_writes_the_component_patch() {
        let m = command_manager();
        let mut env = creative_env("minecraft:diamond_sword");
        assert_eq!(
            m.dispatch(&mut env, ".enchant one sharpness level 3"),
            Dispatch::Handled
        );
        assert_eq!(components(&env), "{enchantments:{minecraft:sharpness:3}}");
        assert_eq!(env.packets[0].name, "ServerboundSetCreativeModeSlotPacket");
        assert_eq!(
            env.packets[0].fields[0],
            ("slot".to_string(), "36".to_string())
        );

        assert_eq!(
            m.dispatch(&mut env, ".enchant one sharpness max"),
            Dispatch::Handled
        );
        assert_eq!(components(&env), "{enchantments:{minecraft:sharpness:5}}");
    }

    #[test]
    fn enchant_all_and_clear() {
        let m = command_manager();
        let mut env = creative_env("minecraft:diamond_sword");
        assert_eq!(m.dispatch(&mut env, ".enchant all max"), Dispatch::Handled);
        let patch = components(&env);
        assert!(patch.contains("minecraft:unbreaking:3"));
        assert!(patch.contains("minecraft:sharpness:5"));
        // `all` is every enchantment, `all_possible` only the sword's.
        env.packets.clear();
        assert_eq!(
            m.dispatch(&mut env, ".enchant all_possible max"),
            Dispatch::Handled
        );
        let possible = components(&env);
        assert!(possible.contains("minecraft:sharpness:5"));
        assert!(!possible.contains("minecraft:power"));

        assert_eq!(m.dispatch(&mut env, ".enchant clear"), Dispatch::Handled);
        assert_eq!(components(&env), "{enchantments:{}}");
        assert_eq!(
            m.dispatch(&mut env, ".enchant remove sharpness"),
            Dispatch::Handled
        );
        assert_eq!(components(&env), "{enchantments:{-minecraft:sharpness}}");
    }

    #[test]
    #[test]
    #[ignore = "needs Horizon server data (ServerData/plugin tree) the mod API does not expose"]
    fn enchant_error_paths() {
        let m = command_manager();
        // Survival: creative-only.
        let mut env = TestEnv::in_world();
        env.inventory = vec![InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:diamond_sword".into(),
                count: 1,
                ..Default::default()
            }),
        }];
        assert_eq!(
            m.dispatch(&mut env, ".enchant one sharpness level 1"),
            Dispatch::Failed
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("You must be in creative mode to use this.")));

        // Creative, empty hand.
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().game_mode = GameMode::Creative;
        assert_eq!(m.dispatch(&mut env, ".enchant clear"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("You need to hold some item to enchant.")));

        // Unknown enchantment and a bad level form.
        let mut env = creative_env("minecraft:diamond_sword");
        assert_eq!(
            m.dispatch(&mut env, ".enchant one nope level 1"),
            Dispatch::Failed
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Unknown enchantment nope.")));
        assert_eq!(
            m.dispatch(&mut env, ".enchant one sharpness"),
            Dispatch::Failed
        );
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Usage: .enchant one <enchantment> (level <level>|max)")));
    }
}

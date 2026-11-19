//! Player modules (`systems.modules.player`).
//!
//! One file per module, registered here in the order `Modules.initPlayer()` adds
//! them. The helpers at the bottom stand in for the Java utilities these modules
//! share - `utils.player.SlotUtils`, `utils.player.InvUtils`, `ListMode` and the
//! food table `Utils.isFood` reads - so every module file stays a straight
//! translation of its Java original.

pub mod air_place;
pub mod anti_afk;
pub mod anti_hunger;
pub mod auto_clicker;
pub mod auto_eat;
pub mod auto_fish;
pub mod auto_gap;
pub mod auto_mend;
pub mod auto_replenish;
pub mod auto_respawn;
pub mod auto_tool;
pub mod break_delay;
pub mod chest_swap;
pub mod exp_thrower;
pub mod fake_player;
pub mod fast_use;
pub mod ghost_hand;
pub mod instant_rebreak;
pub mod liquid_interact;
pub mod middle_click_extra;
pub mod multitask;
pub mod name_protect;
pub mod no_interact;
pub mod no_mining_trace;
pub mod no_rotate;
pub mod no_status_effects;
pub mod portals;
pub mod potion_saver;
pub mod reach;
pub mod rotation;
pub mod speed_mine;

use std::sync::Arc;

use crate::meteor_core::env::{ClickMode, Env, ItemStackInfo, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::setting::{Settings, VisibleFn};

/// `Modules.initPlayer()` - the Java registration order, top to bottom.
pub fn register(modules: &mut Modules) {
    modules.add(Box::new(air_place::AirPlace::new()));
    modules.add(Box::new(anti_afk::AntiAfk::new()));
    modules.add(Box::new(anti_hunger::AntiHunger::new()));
    modules.add(Box::new(auto_eat::AutoEat::new()));
    modules.add(Box::new(auto_clicker::AutoClicker::new()));
    modules.add(Box::new(auto_fish::AutoFish::new()));
    modules.add(Box::new(auto_gap::AutoGap::new()));
    modules.add(Box::new(auto_mend::AutoMend::new()));
    modules.add(Box::new(auto_replenish::AutoReplenish::new()));
    modules.add(Box::new(auto_respawn::AutoRespawn::new()));
    modules.add(Box::new(auto_tool::AutoTool::new()));
    modules.add(Box::new(break_delay::BreakDelay::new()));
    modules.add(Box::new(chest_swap::ChestSwap::new()));
    modules.add(Box::new(exp_thrower::ExpThrower::new()));
    modules.add(Box::new(fake_player::FakePlayer::new()));
    modules.add(Box::new(fast_use::FastUse::new()));
    modules.add(Box::new(ghost_hand::GhostHand::new()));
    modules.add(Box::new(instant_rebreak::InstantRebreak::new()));
    modules.add(Box::new(liquid_interact::LiquidInteract::new()));
    modules.add(Box::new(middle_click_extra::MiddleClickExtra::new()));
    modules.add(Box::new(multitask::Multitask::new()));
    modules.add(Box::new(name_protect::NameProtect::new()));
    modules.add(Box::new(no_interact::NoInteract::new()));
    modules.add(Box::new(no_mining_trace::NoMiningTrace::new()));
    modules.add(Box::new(no_rotate::NoRotate::new()));
    modules.add(Box::new(no_status_effects::NoStatusEffects::new()));
    modules.add(Box::new(portals::Portals::new()));
    modules.add(Box::new(potion_saver::PotionSaver::new()));
    modules.add(Box::new(reach::Reach::new()));
    modules.add(Box::new(rotation::RotationModule::new()));
    modules.add(Box::new(speed_mine::SpeedMine::new()));
}

// ---------------------------------------------------------------------------
// `utils.player.SlotUtils` - indices into `mc.player.getInventory()`.
// ---------------------------------------------------------------------------

/// `SlotUtils.HOTBAR_START`.
pub const HOTBAR_START: usize = 0;
/// `SlotUtils.HOTBAR_END`.
pub const HOTBAR_END: usize = 8;
/// `SlotUtils.MAIN_START`.
pub const MAIN_START: usize = 9;
/// `SlotUtils.MAIN_END`.
pub const MAIN_END: usize = 35;
/// `SlotUtils.ARMOR_START`.
pub const ARMOR_START: usize = 36;
/// `SlotUtils.ARMOR_END`.
pub const ARMOR_END: usize = 39;
/// `SlotUtils.OFFHAND`.
pub const OFFHAND: usize = 40;

/// The chest slot (`EquipmentSlot.CHEST`, index 38 in `Inventory`).
pub const CHEST_SLOT: usize = 38;

/// `SlotUtils.isHotbar`.
pub fn is_hotbar(slot: usize) -> bool {
    (HOTBAR_START..=HOTBAR_END).contains(&slot)
}

/// `SlotUtils.isMain`.
pub fn is_main(slot: usize) -> bool {
    (MAIN_START..=MAIN_END).contains(&slot)
}

/// `SlotUtils.isArmor`.
pub fn is_armor(slot: usize) -> bool {
    (ARMOR_START..=ARMOR_END).contains(&slot)
}

/// `SlotUtils.indexToId` for the player's own inventory screen
/// (`survivalInventory`): the id a container click packet has to carry.
///
/// PORT GAP: the port has no `AbstractContainerMenu`, so the chest/crafting/
/// furnace mappings `SlotUtils.indexToId` switches over are not applied - every
/// click the player modules make is assumed to happen in the inventory screen.
pub fn index_to_id(index: usize) -> usize {
    if is_hotbar(index) {
        36 + index
    } else if is_armor(index) {
        5 + (index - ARMOR_START)
    } else if index == OFFHAND {
        45
    } else {
        index
    }
}

// ---------------------------------------------------------------------------
// `utils.player.InvUtils` - inventory reads and the clicks it makes.
// ---------------------------------------------------------------------------

/// The stack in one inventory index, `None` when the slot is empty.
pub fn stack(env: &dyn Env, index: usize) -> Option<ItemStackInfo> {
    env.inventory()
        .into_iter()
        .find(|s| s.index == index)
        .and_then(|s| s.stack)
        .filter(|s| !s.is_empty())
}

/// The stacks of the off hand, for `mc.player.getOffhandItem()`.
pub fn offhand(env: &dyn Env) -> Option<ItemStackInfo> {
    stack(env, OFFHAND)
}

/// `InvUtils.find(predicate, start, end)`: the first matching index.
pub fn find(
    env: &dyn Env,
    start: usize,
    end: usize,
    pred: impl Fn(&ItemStackInfo) -> bool,
) -> Option<usize> {
    let inventory = env.inventory();
    (start..=end).find(|i| {
        inventory
            .iter()
            .find(|s| s.index == *i)
            .and_then(|s| s.stack.as_ref())
            .map(|st| !st.is_empty() && pred(st))
            .unwrap_or(false)
    })
}

/// `InvUtils.findEmpty()`.
pub fn find_empty(env: &dyn Env, start: usize, end: usize) -> Option<usize> {
    let inventory = env.inventory();
    (start..=end).find(|i| {
        inventory
            .iter()
            .find(|s| s.index == *i)
            .and_then(|s| s.stack.as_ref())
            .map(|st| st.is_empty())
            .unwrap_or(true)
    })
}

/// `InvUtils.swap(slot, swapBack)`, server half: `ServerboundSetCarriedItemPacket`.
///
/// PORT GAP: Horizon exposes no `Inventory.setSelectedSlot`, so the client keeps
/// its own selection - the server is told, the local `selected_slot` is not
/// updated until the host applies a selection of its own.
pub fn select_hotbar(env: &mut dyn Env, slot: usize) -> bool {
    if !is_hotbar(slot) {
        return false;
    }
    env.send_packet(PacketInfo {
        name: "set_carried_item".to_string(),
        clientbound: false,
        fields: vec![("slot".to_string(), slot.to_string())],
    });
    true
}

/// `InvUtils.move().from(from).to(to)` - two `PICKUP` clicks, the Java way a
/// stack is moved inside the inventory screen.
pub fn move_stack(env: &mut dyn Env, from: usize, to: usize) {
    env.click_slot(index_to_id(from), 0, ClickMode::Pickup);
    env.click_slot(index_to_id(to), 0, ClickMode::Pickup);
}

/// `InvUtils.quickSwap().fromId(hotbarId).to(slot)` - one `SWAP` click.
pub fn quick_swap(env: &mut dyn Env, hotbar_id: usize, slot: usize) {
    env.click_slot(index_to_id(slot), hotbar_id as i32, ClickMode::Swap);
}

/// `ItemStack.isStackable()` / `!stack.isStackable()` with what
/// `ItemStackInfo` carries: an unstackable item always has a count of one.
pub fn is_stackable(stack: &ItemStackInfo) -> bool {
    stack.count > 1 || !is_unstackable(&stack.id)
}

/// The item ids vanilla treats as unstackable.
fn is_unstackable(id: &str) -> bool {
    let id = bare(id);
    id.ends_with("_bucket")
        || id.ends_with("_potion")
        || id.ends_with("_boat")
        || id.ends_with("_minecart")
        || id == "water_bucket"
        || id == "lava_bucket"
        || id == "milk_bucket"
        || id == "elytra"
        || id == "shield"
        || id == "turtle_helmet"
        || id == "totem_of_undying"
        || id == "bow"
        || id == "crossbow"
        || id == "trident"
        || id == "fishing_rod"
        || id == "flint_and_steel"
        || id == "carrot_on_a_stick"
        || id == "warped_fungus_on_a_stick"
        || id.ends_with("_sword")
        || id.ends_with("_pickaxe")
        || id.ends_with("_axe")
        || id.ends_with("_shovel")
        || id.ends_with("_hoe")
        || id.ends_with("_helmet")
        || id.ends_with("_chestplate")
        || id.ends_with("_leggings")
        || id.ends_with("_boots")
}

/// `stack.is(item)` with or without the namespace.
pub fn is(stack: &ItemStackInfo, id: &str) -> bool {
    stack.id.eq_ignore_ascii_case(id) || bare(&stack.id).eq_ignore_ascii_case(bare(id))
}

/// The id without its `minecraft:` namespace.
pub fn bare(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

/// `Enchantments.MENDING` and friends, as `ItemStackInfo.enchant_level` reads them.
pub fn has_enchant(stack: &ItemStackInfo, enchantment: &str) -> bool {
    stack.enchant_level(enchantment) > 0
}

// ---------------------------------------------------------------------------
// Packet fields (Horizon hands the port `(key, value)` text).
// ---------------------------------------------------------------------------

fn field_key_eq(key: &str, name: &str) -> bool {
    key.eq_ignore_ascii_case(name)
        || key
            .replace('_', "")
            .eq_ignore_ascii_case(&name.replace('_', ""))
}

/// One field of a packet, by name (`snake_case` or `camelCase`).
pub fn field<'a>(packet: &'a PacketInfo, name: &str) -> Option<&'a str> {
    packet
        .fields
        .iter()
        .find(|(k, _)| field_key_eq(k, name))
        .map(|(_, v)| v.as_str())
}

/// Writes one field, adding it when the packet does not carry it yet.
pub fn set_field(packet: &mut PacketInfo, name: &str, value: &str) {
    if let Some((_, v)) = packet
        .fields
        .iter_mut()
        .find(|(k, _)| field_key_eq(k, name))
    {
        *v = value.to_string();
    } else {
        packet.fields.push((name.to_string(), value.to_string()));
    }
}

/// A block position as the packet fields carry it, `"x,y,z"`.
pub fn parse_pos(text: &str) -> Option<BlockPos> {
    let mut parts = text.split(',');
    let x = parts.next()?.trim().parse().ok()?;
    let y = parts.next()?.trim().parse().ok()?;
    let z = parts.next()?.trim().parse().ok()?;
    Some(BlockPos::new(x, y, z))
}

/// A block position in the packet's `"x,y,z"` form.
pub fn format_pos(pos: BlockPos) -> String {
    format!("{},{},{}", pos.x, pos.y, pos.z)
}

// ---------------------------------------------------------------------------
// `ListMode` - `utils.misc.ListMode`.
// ---------------------------------------------------------------------------

/// `ListMode.allows(contains)`, with `whitelist` standing for the enum choice.
pub fn list_allows(whitelist: bool, contains: bool) -> bool {
    if whitelist {
        contains
    } else {
        !contains
    }
}

// ---------------------------------------------------------------------------
// Settings helpers: `Setting.Builder.description(...)` / `.visible(...)`.
// ---------------------------------------------------------------------------

/// `Setting.Builder.description(...)` for the named setting.
pub fn describe(settings: &mut Settings, name: &str, description: &str) {
    for setting in settings.iter_mut() {
        if setting.name.eq_ignore_ascii_case(name) {
            setting.description = description.to_string();
        }
    }
}

/// `Setting.Builder.visible(...)` for the named setting.
///
/// The CONTRACT's `set_visible_last` only reaches the setting just added in the
/// *last* group; the player modules add into the default group after creating
/// the others, so they bind the predicate by name instead.
pub fn visible(
    settings: &mut Settings,
    name: &str,
    predicate: impl Fn() -> bool + Send + Sync + 'static,
) {
    let predicate: VisibleFn = Arc::new(predicate);
    for setting in settings.iter_mut() {
        if setting.name.eq_ignore_ascii_case(name) {
            setting.visible = Some(predicate.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Item knowledge `Utils.isFood` / the tool tags read from ItemStack.
// ---------------------------------------------------------------------------

/// `(nutrition, saturation)` of every food `Utils.isFood` accepts, keyed by the
/// id without its namespace. PORT GAP: `ItemStackInfo` carries no
/// `DataComponents.FOOD`, so the vanilla table lives here instead.
pub fn food(id: &str) -> Option<(f32, f32)> {
    let value = match bare(id) {
        "apple" => (4.0, 2.4),
        "baked_potato" => (5.0, 6.0),
        "beef" => (3.0, 1.8),
        "beetroot" => (1.0, 1.2),
        "beetroot_soup" => (6.0, 7.2),
        "bread" => (5.0, 6.0),
        "carrot" => (3.0, 3.6),
        "chicken" => (2.0, 1.2),
        "chorus_fruit" => (4.0, 2.4),
        "cod" => (2.0, 0.4),
        "cooked_beef" => (8.0, 12.8),
        "cooked_chicken" => (6.0, 7.2),
        "cooked_cod" => (5.0, 6.0),
        "cooked_mutton" => (6.0, 9.6),
        "cooked_porkchop" => (8.0, 12.8),
        "cooked_rabbit" => (5.0, 6.0),
        "cooked_salmon" => (6.0, 9.6),
        "cookie" => (2.0, 0.4),
        "dried_kelp" => (1.0, 0.6),
        "enchanted_golden_apple" => (4.0, 9.6),
        "glow_berries" => (2.0, 0.4),
        "golden_apple" => (4.0, 9.6),
        "golden_carrot" => (6.0, 14.4),
        "honey_bottle" => (6.0, 1.2),
        "melon_slice" => (2.0, 1.2),
        "mushroom_stew" => (6.0, 7.2),
        "mutton" => (2.0, 1.2),
        "poisonous_potato" => (2.0, 1.2),
        "porkchop" => (3.0, 1.8),
        "potato" => (1.0, 0.6),
        "pufferfish" => (1.0, 0.2),
        "pumpkin_pie" => (8.0, 4.8),
        "rabbit" => (3.0, 1.8),
        "rabbit_stew" => (10.0, 12.0),
        "rotten_flesh" => (4.0, 0.8),
        "salmon" => (2.0, 0.4),
        "spider_eye" => (2.0, 3.2),
        "suspicious_stew" => (6.0, 7.2),
        "sweet_berries" => (2.0, 0.4),
        "tropical_fish" => (1.0, 0.2),
        _ => return None,
    };
    Some(value)
}

/// `food.canAlwaysEat()` - the foods that are edible at a full hunger bar.
pub fn can_always_eat(id: &str) -> bool {
    matches!(
        bare(id),
        "golden_apple" | "enchanted_golden_apple" | "chorus_fruit"
    )
}

/// The pickaxes `ItemTags.PICKAXES` covers.
pub fn is_pickaxe(id: &str) -> bool {
    bare(id).ends_with("_pickaxe")
}

/// `ItemTags.AXES`.
pub fn is_axe(id: &str) -> bool {
    bare(id).ends_with("_axe")
}

/// `ItemTags.SHOVELS`.
pub fn is_shovel(id: &str) -> bool {
    bare(id).ends_with("_shovel")
}

/// `ItemTags.HOES`.
pub fn is_hoe(id: &str) -> bool {
    bare(id).ends_with("_hoe")
}

/// `ItemTags.SWORDS`.
pub fn is_sword(id: &str) -> bool {
    bare(id).ends_with("_sword")
}

/// `AutoTool.isTool`: axes, hoes, pickaxes, shovels and shears.
pub fn is_tool(id: &str) -> bool {
    is_axe(id) || is_hoe(id) || is_pickaxe(id) || is_shovel(id) || bare(id) == "shears"
}

/// The material multiplier `ItemStack.getDestroySpeed(state)` is built from.
pub fn tool_tier(id: &str) -> f32 {
    let id = bare(id);
    if id == "shears" {
        5.0
    } else if id.starts_with("wooden_") {
        2.0
    } else if id.starts_with("golden_") {
        12.0
    } else if id.starts_with("stone_") {
        4.0
    } else if id.starts_with("iron_") {
        6.0
    } else if id.starts_with("diamond_") {
        8.0
    } else if id.starts_with("netherite_") {
        9.0
    } else {
        1.0
    }
}

//! Combat modules (`systems.modules.combat`).
//!
//! One file per module, registered in `Modules.initCombat()`'s order. Everything
//! the Java combat package shares (`utils.entity.TargetUtils`,
//! `utils.entity.SortPriority`, `utils.world.BlockUtils`, `utils.player.InvUtils`
//! and `utils.entity.DamageUtils`) lives here as `pub(crate)` helpers, so each
//! module file stays a direct translation of its Java class.

pub mod anchor_aura;
pub mod anti_anvil;
pub mod anti_bed;
pub mod arrow_dodge;
pub mod attribute_swap;
pub mod auto_anvil;
pub mod auto_armor;
pub mod auto_city;
pub mod auto_exp;
pub mod auto_log;
pub mod auto_totem;
pub mod auto_trap;
pub mod auto_weapon;
pub mod auto_web;
pub mod bed_aura;
pub mod bow_aimbot;
pub mod bow_spam;
pub mod burrow;
pub mod criticals;
pub mod crystal_aura;
pub mod hitboxes;
pub mod hole_filler;
pub mod kill_aura;
pub mod offhand;
pub mod quiver;
pub mod self_anvil;
pub mod self_trap;
pub mod self_web;
pub mod surround;

use crate::meteor_core::env::{
    BlockInfo, EntityInfo, Env, Hand, ItemStackInfo, PlayerInfo, Renderer,
};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{normalize_yaw, BlockPos, Dir, Rotation, Vec3d};
use crate::meteor_core::setting::{EnumSet, SettingGroup, Settings};

/// `Modules.initCombat()`.
pub fn register(modules: &mut Modules) {
    modules.add(Box::new(anchor_aura::AnchorAura::new()));
    modules.add(Box::new(anti_anvil::AntiAnvil::new()));
    modules.add(Box::new(anti_bed::AntiBed::new()));
    modules.add(Box::new(arrow_dodge::ArrowDodge::new()));
    modules.add(Box::new(attribute_swap::AttributeSwap::new()));
    modules.add(Box::new(auto_anvil::AutoAnvil::new()));
    modules.add(Box::new(auto_armor::AutoArmor::new()));
    modules.add(Box::new(auto_city::AutoCity::new()));
    modules.add(Box::new(auto_exp::AutoExp::new()));
    modules.add(Box::new(auto_log::AutoLog::new()));
    modules.add(Box::new(auto_totem::AutoTotem::new()));
    modules.add(Box::new(auto_trap::AutoTrap::new()));
    modules.add(Box::new(auto_weapon::AutoWeapon::new()));
    modules.add(Box::new(auto_web::AutoWeb::new()));
    modules.add(Box::new(bed_aura::BedAura::new()));
    modules.add(Box::new(bow_aimbot::BowAimbot::new()));
    modules.add(Box::new(bow_spam::BowSpam::new()));
    modules.add(Box::new(burrow::Burrow::new()));
    modules.add(Box::new(criticals::Criticals::new()));
    modules.add(Box::new(crystal_aura::CrystalAura::new()));
    modules.add(Box::new(hitboxes::Hitboxes::new()));
    modules.add(Box::new(hole_filler::HoleFiller::new()));
    modules.add(Box::new(kill_aura::KillAura::new()));
    modules.add(Box::new(offhand::Offhand::new()));
    modules.add(Box::new(quiver::Quiver::new()));
    modules.add(Box::new(self_anvil::SelfAnvil::new()));
    modules.add(Box::new(self_trap::SelfTrap::new()));
    modules.add(Box::new(self_web::SelfWeb::new()));
    modules.add(Box::new(surround::Surround::new()));
}

// ---------------------------------------------------------------------------
// Settings plumbing
// ---------------------------------------------------------------------------

/// The group `s.create_group(...)` / `s.default_group()` last touched, so a
/// module can add one setting at a time and still call
/// `s.set_visible_last(...)` between them (`Settings::create_group` hands out a
/// borrow that would otherwise live to the end of the block).
pub(crate) fn last_group(s: &mut Settings) -> &mut SettingGroup {
    let len = s.groups.len();
    if len == 0 {
        return s.default_group();
    }
    &mut s.groups[len - 1]
}

// ---------------------------------------------------------------------------
// `utils.entity.SortPriority` / `utils.entity.TargetUtils`
// ---------------------------------------------------------------------------

/// `SortPriority`'s constants, in declaration order.
pub(crate) const SORT_PRIORITIES: &[&str] = &[
    "lowest-distance",
    "highest-distance",
    "lowest-health",
    "highest-health",
    "closest-angle",
];

/// `SortPriority.LowestDistance`.
pub(crate) const SORT_LOWEST_DISTANCE: usize = 0;
/// `SortPriority.LowestHealth`.
pub(crate) const SORT_LOWEST_HEALTH: usize = 2;
/// `SortPriority.ClosestAngle`.
pub(crate) const SORT_CLOSEST_ANGLE: usize = 4;

/// `Target.Body`: the point `Rotations.getPitch(entity, Target.Body)` aims at.
pub(crate) fn body_pos(e: &EntityInfo) -> Vec3d {
    let height = if e.baby { 0.9 } else { 1.8 };
    Vec3d::new(e.pos.x, e.pos.y + height / 2.0, e.pos.z)
}

/// `TargetUtils.getList`'s sort: the Java comparators, ported one for one.
pub(crate) fn sort_targets(
    list: &mut [EntityInfo],
    priority: usize,
    eye: Vec3d,
    yaw: f32,
    pitch: f32,
) {
    let squared = |e: &EntityInfo| eye.distance_sq(e.pos);
    let angle = |e: &EntityInfo| {
        let r = Rotation::between(eye, body_pos(e));
        let dy = normalize_yaw(r.yaw - yaw) as f64;
        let dp = (r.pitch - pitch) as f64;
        dy * dy + dp * dp
    };
    list.sort_by(|a, b| match priority {
        SORT_LOWEST_DISTANCE => squared(a).total_cmp(&squared(b)),
        1 => squared(b).total_cmp(&squared(a)),
        SORT_LOWEST_HEALTH => a.health.total_cmp(&b.health),
        3 => b.health.total_cmp(&a.health),
        SORT_CLOSEST_ANGLE => angle(a).total_cmp(&angle(b)),
        _ => squared(a).total_cmp(&squared(b)),
    });
}

/// `TargetUtils.getList(targets, isGood, priority, maxCount)`.
pub(crate) fn select_targets(
    env: &dyn Env,
    priority: usize,
    max: usize,
    is_good: impl Fn(&EntityInfo) -> bool,
) -> Vec<EntityInfo> {
    let Some(player) = env.player() else {
        return Vec::new();
    };
    let mut list: Vec<EntityInfo> = env.entities().into_iter().filter(|e| is_good(e)).collect();
    sort_targets(
        &mut list,
        priority,
        player.eye_pos(),
        player.yaw,
        player.pitch,
    );
    list.truncate(max.max(1));
    list
}

/// `TargetUtils.get`: the single best match.
pub(crate) fn select_target(
    env: &dyn Env,
    priority: usize,
    is_good: impl Fn(&EntityInfo) -> bool,
) -> Option<EntityInfo> {
    select_targets(env, priority, 1, is_good).into_iter().next()
}

/// `TargetUtils.getPlayerTarget(range, priority)`.
pub(crate) fn player_target(env: &dyn Env, range: f64, priority: usize) -> Option<EntityInfo> {
    let player = env.player()?;
    let eye = player.eye_pos();
    select_target(env, priority, |e| {
        if !e.is_player || !e.alive || e.health <= 0.0 {
            return false;
        }
        if let Some(name) = &e.name {
            if env.is_friend(name) {
                return false;
            }
        }
        eye.distance_sq(e.pos) <= range * range
    })
}

/// `TargetUtils.isBadTarget(target, range)`.
pub(crate) fn is_bad_target(env: &dyn Env, target: Option<&EntityInfo>, range: f64) -> bool {
    let Some(t) = target else { return true };
    let Some(player) = env.player() else {
        return true;
    };
    !t.alive || t.health <= 0.0 || player.eye_pos().distance_sq(t.pos) > range * range
}

// ---------------------------------------------------------------------------
// `PlayerUtils` reads + maths
// ---------------------------------------------------------------------------

/// `PlayerUtils.isWithin(pos, r)` measured from the player's eyes, as the Java
/// combat modules do for blocks.
pub(crate) fn within(eye: Vec3d, pos: Vec3d, r: f64) -> bool {
    eye.distance_sq(pos) <= r * r
}

/// `PlayerUtils.distance(target, explosion)`: a plain point-to-point distance.
pub(crate) fn distance(a: Vec3d, b: Vec3d) -> f64 {
    a.distance_to(b)
}

/// `mc.player.getEyePos()`.
pub(crate) fn eye(player: &PlayerInfo) -> Vec3d {
    player.eye_pos()
}

// ---------------------------------------------------------------------------
// Block predicates (`BlockUtils`)
// ---------------------------------------------------------------------------

/// `BlockState.isAir()` plus the plants and fluids a placement can overwrite.
pub(crate) fn replaceable(b: &BlockInfo) -> bool {
    if b.is_air() {
        return true;
    }
    let id = b.id.strip_prefix("minecraft:").unwrap_or(&b.id);
    matches!(
        id,
        "water"
            | "lava"
            | "short_grass"
            | "grass"
            | "tall_grass"
            | "fern"
            | "dead_bush"
            | "snow"
            | "fire"
            | "vine"
            | "seagrass"
            | "tall_seagrass"
            | "kelp"
            | "light"
            | "structure_void"
    )
}

/// What a ray stops at. `BlockUtils` counts every collider; a block id is the
/// closest thing `Env` exposes.
pub(crate) fn opaque(b: &BlockInfo) -> bool {
    !replaceable(b)
}

/// `PlayerUtils.canSeeBlock` / `canSeeEntity`: is the straight line clear?
///
/// PORT GAP: `Level.clip` uses each block's collision shape; the port only knows
/// the block id, so every non-replaceable block is a full cube here.
pub(crate) fn line_of_sight(env: &dyn Env, from: Vec3d, to: Vec3d) -> bool {
    !blocked(env, from, to, None)
}

/// The body of `line_of_sight`, with one position the caller may override
/// (`DamageUtils.getOverridingHitFactory`).
pub(crate) fn blocked(
    env: &dyn Env,
    from: Vec3d,
    to: Vec3d,
    override_pos: Option<(BlockPos, bool)>,
) -> bool {
    let delta = to.sub(from);
    let len = delta.length();
    if len < 1.0e-6 {
        return false;
    }
    let dir = delta.mul(1.0 / len);
    let steps = (len / 0.25).ceil() as i32;
    let mut last: Option<BlockPos> = None;
    for i in 1..steps {
        let p = from.add(dir.mul(i as f64 * 0.25));
        let bp = p.to_block_pos();
        if Some(bp) == last {
            continue;
        }
        last = Some(bp);
        if let Some((op, solid)) = override_pos {
            if bp == op {
                if solid {
                    return true;
                }
                continue;
            }
        }
        let Some(b) = env.block(bp) else { continue };
        if opaque(&b) {
            return true;
        }
    }
    false
}

/// `BlockUtils.canPlace(BlockPos)`: the position is empty and a neighbour can be
/// clicked to place against.
pub(crate) fn can_place_at(env: &dyn Env, pos: BlockPos) -> bool {
    match env.block(pos) {
        Some(b) => replaceable(&b),
        None => true,
    }
}

/// `BlockUtils.getPlaceSide`: the first neighbour a placement can be clicked on,
/// as `(support pos, clicked face)`.
pub(crate) fn place_side(env: &dyn Env, pos: BlockPos) -> Option<(BlockPos, Dir)> {
    for d in [
        Dir::Down,
        Dir::North,
        Dir::South,
        Dir::West,
        Dir::East,
        Dir::Up,
    ] {
        let n = pos.offset_dir(d);
        if let Some(b) = env.block(n) {
            if opaque(&b) {
                return Some((n, d.opposite()));
            }
        }
    }
    None
}

/// `BlockUtils.place(pos, item, rotate, swing, checkRange)` reduced to what
/// `Env` can say.
///
/// `state` is the block to write (`env.set_block`), the item click goes through
/// `env.use_item_on` on the support block. `air_place` allows placing into thin
/// air without a neighbour (`BlockUtils.place`'s air-place path).
///
/// PORT GAP: the real `BlockUtils.place` also does an inventory swap into the
/// hotbar (`InvUtils.swap`) and sends `PlayerActionC2SPacket`/`swing`; `Env`
/// applies the block directly and has no hand-swing event beyond `Action::Use`.
pub(crate) fn place_block(
    env: &mut dyn Env,
    pos: BlockPos,
    state: &str,
    rotate: bool,
    air_place: bool,
) -> bool {
    if !can_place_at(env, pos) {
        return false;
    }
    let support = place_side(env, pos).or_else(|| {
        if air_place {
            let below = pos.offset(0, -1, 0);
            Some((below, Dir::Up))
        } else {
            None
        }
    });
    let Some((support_pos, face)) = support else {
        return false;
    };
    if rotate {
        if let Some(player) = env.player() {
            env.set_rotation(Rotation::between(player.eye_pos(), pos.center()));
        }
    }
    env.use_item_on(support_pos, face, Hand::Main);
    env.set_block(pos, state)
}

/// `PlayerUtils.isWithin` for a placement, honouring the walls range: a block
/// behind another one may only be reached at `walls_range`.
pub(crate) fn in_place_range(
    env: &dyn Env,
    eye: Vec3d,
    pos: BlockPos,
    range: f64,
    walls_range: f64,
) -> bool {
    let center = pos.center();
    if eye.distance_sq(center) > range * range {
        return false;
    }
    line_of_sight(env, eye, center) || eye.distance_sq(center) <= walls_range * walls_range
}

// ---------------------------------------------------------------------------
// Inventory (`InvUtils` + `FindItemResult`)
// ---------------------------------------------------------------------------

/// The container slot of the off hand (`SlotUtils.OFFHAND`).
pub(crate) const OFFHAND_SLOT: usize = 40;

/// `FindItemResult.slot()` when nothing was found.
pub(crate) const NO_SLOT: usize = usize::MAX;

/// `ItemStack.is(Item)` / `ItemListSetting.contains`.
pub(crate) fn item_matches(stack: &ItemStackInfo, ids: &[&str]) -> bool {
    if stack.is_empty() {
        return false;
    }
    ids.iter()
        .any(|id| stack.id == *id || stack.id.strip_prefix("minecraft:") == Some(*id))
}

/// The item tag groups Meteor checks (`ItemTags.SWORDS`, `.AXES`, ...). Ported
/// as id suffix tests because `Env` deals in ids.
pub(crate) fn is_tag(stack: &ItemStackInfo, tag: &str) -> bool {
    if stack.is_empty() {
        return false;
    }
    let id = stack.id.strip_prefix("minecraft:").unwrap_or(&stack.id);
    match tag {
        "swords" => id.ends_with("_sword"),
        "axes" => id.ends_with("_axe"),
        "pickaxes" => id.ends_with("_pickaxe"),
        "shovels" => id.ends_with("_shovel"),
        "hoes" => id.ends_with("_hoe"),
        "spears" => id.ends_with("_spear"),
        "trident" => id == "trident",
        "mace" => id == "mace",
        _ => false,
    }
}

/// `InvUtils.find(Item...)`: the first inventory slot holding one of `ids`.
pub(crate) fn find_item(env: &dyn Env, ids: &[&str]) -> Option<(usize, ItemStackInfo)> {
    for slot in env.inventory() {
        let Some(stack) = slot.stack else { continue };
        if item_matches(&stack, ids) {
            return Some((slot.index, stack));
        }
    }
    None
}

/// `InvUtils.find(..., 0, 8)`: a hotbar slot only, as Auto Switch needs.
pub(crate) fn find_hotbar_item(env: &dyn Env, ids: &[&str]) -> Option<usize> {
    for slot in env.inventory() {
        if slot.index > 8 {
            continue;
        }
        let Some(stack) = slot.stack else { continue };
        if item_matches(&stack, ids) {
            return Some(slot.index);
        }
    }
    None
}

/// `FindItemResult.count()`.
pub(crate) fn count_item(env: &dyn Env, ids: &[&str]) -> i32 {
    env.inventory()
        .into_iter()
        .filter_map(|s| s.stack)
        .filter(|s| item_matches(s, ids))
        .map(|s| s.count)
        .sum()
}

/// `InvUtils.move().from(slot).toOffhand()`.
///
/// PORT GAP: the Java helper sends a `SWAP` click against the off-hand slot of
/// the *player* inventory menu; the port uses the same click on
/// [`OFFHAND_SLOT`].
pub(crate) fn move_to_offhand(env: &mut dyn Env, from: usize) {
    env.click_slot(
        from,
        OFFHAND_SLOT as i32,
        crate::meteor_core::env::ClickMode::Swap,
    );
}

/// `InvUtils.quickMove().slot(slot)` - shift-click, used to equip from anywhere.
pub(crate) fn quick_move(env: &mut dyn Env, slot: usize) {
    env.click_slot(slot, 0, crate::meteor_core::env::ClickMode::QuickMove);
}

// ---------------------------------------------------------------------------
// `utils.entity.DamageUtils`
// ---------------------------------------------------------------------------

/// `Attributes.ARMOR` for the four armour slots, from the item ids.
fn armor_of_item(id: &str) -> f32 {
    let id = id.strip_prefix("minecraft:").unwrap_or(id);
    match id {
        "leather_helmet" => 1.0,
        "leather_chestplate" => 3.0,
        "leather_leggings" => 2.0,
        "leather_boots" => 1.0,
        "chainmail_helmet" => 2.0,
        "chainmail_chestplate" => 5.0,
        "chainmail_leggings" => 4.0,
        "chainmail_boots" => 1.0,
        "copper_helmet" => 2.0,
        "copper_chestplate" => 4.0,
        "copper_leggings" => 3.0,
        "copper_boots" => 1.0,
        "iron_helmet" => 2.0,
        "iron_chestplate" => 6.0,
        "iron_leggings" => 5.0,
        "iron_boots" => 2.0,
        "turtle_helmet" => 2.0,
        "golden_helmet" => 2.0,
        "golden_chestplate" => 5.0,
        "golden_leggings" => 3.0,
        "golden_boots" => 1.0,
        "diamond_helmet" => 3.0,
        "diamond_chestplate" => 8.0,
        "diamond_leggings" => 6.0,
        "diamond_boots" => 3.0,
        "netherite_helmet" => 3.0,
        "netherite_chestplate" => 8.0,
        "netherite_leggings" => 6.0,
        "netherite_boots" => 3.0,
        _ => 0.0,
    }
}

/// `getArmor(entity)`.
pub(crate) fn armor_value(e: &EntityInfo) -> f32 {
    e.equipment
        .iter()
        .take(4)
        .filter_map(|s| s.as_ref())
        .map(|s| armor_of_item(&s.id))
        .sum::<f32>()
        .floor()
}

/// `ARMOR_TOUGHNESS`, read off the same four slots.
pub(crate) fn armor_toughness(e: &EntityInfo) -> f32 {
    e.equipment
        .iter()
        .take(4)
        .filter_map(|s| s.as_ref())
        .map(|s| {
            let id = s.id.strip_prefix("minecraft:").unwrap_or(&s.id);
            if id.starts_with("diamond_") {
                2.0
            } else if id.starts_with("netherite_") {
                3.0
            } else {
                0.0
            }
        })
        .sum()
}

/// `DamageUtils.protectionReduction` for explosions only.
pub(crate) fn blast_protection(e: &EntityInfo) -> i32 {
    e.equipment
        .iter()
        .take(4)
        .filter_map(|s| s.as_ref())
        .map(|s| {
            s.enchant_level("minecraft:blast_protection") as i32 * 2
                + s.enchant_level("minecraft:protection") as i32
        })
        .sum()
}

/// `CombatRules.getDamageAfterAbsorb`.
fn after_absorb(damage: f32, armor: f32, toughness: f32) -> f32 {
    let f = (armor / 5.0)
        .max(damage / (2.0 + toughness / 4.0))
        .min(20.0);
    damage * (1.0 - f / 25.0)
}

/// `CombatRules.getDamageAfterMagicAbsorb`.
fn after_magic(damage: f32, protection: i32) -> f32 {
    damage * (1.0 - (protection.min(20) as f32) / 25.0)
}

/// `DamageUtils.getExposure`: the share of the target's box the explosion can
/// reach. Sampled exactly like `ServerExplosion.getSeenPercent`.
fn exposure(
    env: &dyn Env,
    min: Vec3d,
    max: Vec3d,
    source: Vec3d,
    override_pos: Option<(BlockPos, bool)>,
) -> f32 {
    let x_diff = max.x - min.x;
    let y_diff = max.y - min.y;
    let z_diff = max.z - min.z;

    let mut x_step = 1.0 / (x_diff * 2.0 + 1.0);
    let mut y_step = 1.0 / (y_diff * 2.0 + 1.0);
    let mut z_step = 1.0 / (z_diff * 2.0 + 1.0);
    if x_step <= 0.0 || y_step <= 0.0 || z_step <= 0.0 {
        return 0.0;
    }

    let x_offset = (1.0 - (1.0 / x_step).floor() * x_step) * 0.5;
    let z_offset = (1.0 - (1.0 / z_step).floor() * z_step) * 0.5;
    x_step *= x_diff;
    y_step *= y_diff;
    z_step *= z_diff;

    let mut misses = 0;
    let mut hits = 0;
    let mut x = min.x + x_offset;
    while x <= max.x + x_offset {
        let mut y = min.y;
        while y <= max.y {
            let mut z = min.z + z_offset;
            while z <= max.z + z_offset {
                if !blocked(env, Vec3d::new(x, y, z), source, override_pos) {
                    misses += 1;
                }
                hits += 1;
                z += z_step;
            }
            y += y_step;
        }
        x += x_step;
    }

    if hits == 0 {
        0.0
    } else {
        misses as f32 / hits as f32
    }
}

/// `DamageUtils.explosionDamage(target, targetPos, targetBox, explosionPos, power, ...)`.
pub(crate) fn explosion_damage_box(
    env: &dyn Env,
    min: Vec3d,
    max: Vec3d,
    armor: f32,
    toughness: f32,
    blast_prot: i32,
    explosion: Vec3d,
    power: f32,
    override_pos: Option<(BlockPos, bool)>,
) -> f32 {
    let target_pos = Vec3d::new(
        (min.x + max.x) / 2.0,
        (min.y + max.y) / 2.0,
        (min.z + max.z) / 2.0,
    );
    let mod_distance = distance(target_pos, explosion);
    if mod_distance > power as f64 {
        return 0.0;
    }
    let exposure = exposure(env, min, max, explosion, override_pos) as f64;
    let impact = (1.0 - mod_distance / power as f64) * exposure;
    let damage = ((impact * impact + impact) / 2.0 * 7.0 * 12.0 + 1.0) as f32;
    let damage = after_absorb(damage, armor, toughness);
    let damage = after_magic(damage, blast_prot);
    damage.max(0.0)
}

/// `DamageUtils.crystalDamage(target, crystal)`.
pub(crate) fn crystal_damage(env: &dyn Env, target: &EntityInfo, crystal: Vec3d) -> f32 {
    let (min, max) = target.bounding_box();
    explosion_damage_box(
        env,
        min,
        max,
        armor_value(target),
        armor_toughness(target),
        blast_protection(target),
        crystal,
        12.0,
        Some((crystal.to_block_pos(), true)),
    )
}

/// `DamageUtils.anchorDamage(target, anchor)`.
pub(crate) fn anchor_damage(env: &dyn Env, target: &EntityInfo, anchor: Vec3d) -> f32 {
    let (min, max) = target.bounding_box();
    explosion_damage_box(
        env,
        min,
        max,
        armor_value(target),
        armor_toughness(target),
        blast_protection(target),
        anchor,
        10.0,
        Some((anchor.to_block_pos(), false)),
    )
}

/// `DamageUtils.bedDamage(target, bed)`.
pub(crate) fn bed_damage(env: &dyn Env, target: &EntityInfo, bed: Vec3d) -> f32 {
    let (min, max) = target.bounding_box();
    explosion_damage_box(
        env,
        min,
        max,
        armor_value(target),
        armor_toughness(target),
        blast_protection(target),
        bed,
        10.0,
        None,
    )
}

/// The local player as an `EntityInfo`, so the self-damage paths reuse the same
/// maths as the target paths.
///
/// PORT GAP: `PlayerInfo` carries only an armour *count* (`armor`), no per-slot
/// stacks, so the local player's armour points / toughness / enchantments are
/// not readable and self-damage is estimated unarmoured. `damageutils` note.
pub(crate) fn self_entity(env: &dyn Env) -> Option<EntityInfo> {
    let p = env.player()?;
    Some(EntityInfo {
        id: -1,
        kind: "minecraft:player".into(),
        name: Some(p.name.clone()),
        pos: p.pos,
        prev_pos: p.prev_pos,
        velocity: p.velocity,
        yaw: p.yaw,
        pitch: p.pitch,
        head_yaw: p.head_yaw,
        health: p.health,
        max_health: p.max_health,
        alive: p.is_alive(),
        is_player: true,
        ..Default::default()
    })
}

/// `PlayerUtils.possibleHealthReductions(entities, fall)`.
pub(crate) fn possible_health_reductions(env: &dyn Env, entities: bool, fall: bool) -> f32 {
    let Some(player) = env.player() else {
        return 0.0;
    };
    let me = self_entity(env);
    let mut damage_taken = 0.0f32;

    if entities {
        for e in env.entities() {
            if e.kind.ends_with("end_crystal") {
                if let Some(me) = &me {
                    let d = crystal_damage(env, me, e.pos);
                    if d > damage_taken {
                        damage_taken = d;
                    }
                }
            } else if e.is_player && !e.name.as_deref().map(|n| env.is_friend(n)).unwrap_or(false) {
                // PORT GAP: `DamageUtils.getAttackDamage(player, target)` needs the
                // attacker's attribute modifiers, which `EntityInfo` does not
                // carry; a diamond sword's 7 damage is the stand-in.
                if within(player.eye_pos(), e.pos, 5.0) && e.equipment[4].is_some() {
                    let d = 7.0;
                    if d > damage_taken {
                        damage_taken = d;
                    }
                }
            }
        }
    }

    if fall && !player.on_ground {
        // PORT GAP: `player.fallDistance` is not exposed; the drop from the
        // current position down to the first solid block stands in for it.
        let mut drop = 0;
        let base = player.pos.to_block_pos();
        for i in 1..=8 {
            match env.block(base.offset(0, -i, 0)) {
                Some(b) if !replaceable(&b) => break,
                _ => drop = i,
            }
        }
        if drop > 3 {
            let d = after_absorb((drop - 3) as f32, 0.0, 0.0);
            if d > damage_taken {
                damage_taken = d;
            }
        }
    }

    damage_taken
}

// ---------------------------------------------------------------------------
// Render (`renderer.ShapeMode` + the modules' side/line colours)
// ---------------------------------------------------------------------------

/// `ShapeMode`'s constants, in declaration order.
pub(crate) const SHAPE_MODES: &[&str] = &["lines", "sides", "both"];

/// `ShapeMode.Both`.
pub(crate) const SHAPE_BOTH: usize = 2;

/// `ShapeMode.sides()`.
pub(crate) fn shape_sides(mode: &EnumSet) -> bool {
    mode.index() >= 1
}

/// `ShapeMode.lines()`.
pub(crate) fn shape_lines(mode: &EnumSet) -> bool {
    mode.index() != 1
}

/// Draw one block with the module's `shape-mode` and colours.
pub(crate) fn draw_shape(
    r: &mut dyn Renderer,
    pos: BlockPos,
    mode: &EnumSet,
    side: u32,
    line: u32,
) {
    let min = pos.as_vec();
    let max = min.add(Vec3d::new(1.0, 1.0, 1.0));
    if shape_sides(mode) {
        r.filled_box(min, max, side, true);
    }
    if shape_lines(mode) {
        r.box_outline(min, max, line, 2.0, true);
    }
}

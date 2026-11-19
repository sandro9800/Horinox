//! Plumbing shared by the HUD elements: Meteor's inventory slot numbering, the
//! `Settings` builder sugar (Java's `SettingGroup` fields plus the
//! `.describe(...)`/`.visible(...)` calls it chains before `.build()`), and the
//! drawing helpers `Renderer` does not carry.
//!
//! Port gaps that every element shares are recorded here once, instead of in
//! fourteen files:
//!
//! * `Renderer::text2d` has no `shadow` flag and there is no `text_height`, so an
//!   element's `shadow` setting only feeds the size metrics
//!   ([`text_height`]); the engine's font draws its own shadow. Meteor's
//!   `textHeight(shadow, scale)` is `9 * scale` either way for the vanilla font.
//! * `Hud.get().getTextScale()` is a *Hud* setting; `Env` exposes no Hud settings,
//!   so the elements that fall back to it use the setting's default
//!   ([`hud_text_scale`]).
//! * `HudEditorScreen.isOpen()` has no `Env` counterpart, so `isInEditor()` is
//!   ported as `!Env::can_update()` ([`is_in_editor`]).
//! * `Renderer::line` is world space (3D) and there is no 2D line primitive, so
//!   the placeholder cross Meteor draws in the HUD editor is an outline
//!   ([`placeholder`]).
//! * `HudRenderer.item`'s `overlay` flag (the vanilla count/durability overlay)
//!   is not part of `Renderer::item2d`; counts and durability bars are drawn with
//!   `rect`/`text2d` here so the elements keep Meteor's look.

use std::sync::Arc;

use crate::meteor_core::color::{Color, SettingColor};
use crate::meteor_core::env::{Env, ItemStackInfo, Renderer};
use crate::meteor_core::hud::{Alignment, HudBox};
use crate::meteor_core::math::Dir;
use crate::meteor_core::setting::{
    BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, ListSet, Settings, StringSet, VisibleFn,
};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// `HudElement.isInEditor()` - `!Utils.canUpdate() || HudEditorScreen.isOpen()`.
// PORT GAP: the HUD editor screen is not visible through `Env`, so only the
// `!canUpdate()` half of Java's check is portable.
pub fn is_in_editor(env: &dyn Env) -> bool {
    !env.can_update()
}

/// `Hud.get().getTextScale()`, the fallback of every element's `getScale()`.
// PORT GAP: the global text scale lives on `Hud`, and `Env` exposes no Hud
// settings; the elements use the setting's default value.
pub const fn hud_text_scale() -> f64 {
    1.0
}

/// `customScale.get() ? scale.get() : <fallback>`, the `getScale()` every element
/// with a custom scale opens with.
pub fn scale_of(custom_scale: &BoolSet, scale: &DoubleSet, fallback: f64) -> f64 {
    if custom_scale.get() {
        scale.get()
    } else {
        fallback
    }
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

/// `HudRenderer.textHeight(shadow, scale)` for the vanilla font: nine pixels per
/// scale step (a 9px line at scale 1).
pub fn text_height(scale: f64) -> f64 {
    9.0 * scale
}

/// The vertical step Meteor uses between rows of text (`textHeight + 2`).
pub fn line_step(scale: f64) -> f64 {
    text_height(scale) + 2.0
}

/// `16 * scale` - the item icon size Meteor passes to `HudRenderer.item`.
pub fn item_size(scale: f64) -> f64 {
    16.0 * scale
}

/// `18 * scale` - the grid pitch of an item slot.
pub fn item_pitch(scale: f64) -> f64 {
    18.0 * scale
}

/// `Mth.wrapDegrees`.
pub fn wrap_degrees(v: f64) -> f64 {
    let mut d = v % 360.0;
    if d >= 180.0 {
        d -= 360.0;
    }
    if d < -180.0 {
        d += 360.0;
    }
    d
}

/// `Direction.fromYRot(yaw)`: the horizontal direction a yaw faces. Vanilla's
/// `BY_2D_DATA` order is south, west, north, east for the four quarter turns.
pub fn direction_from_yaw(yaw: f64) -> Dir {
    const ORDER: [Dir; 4] = [Dir::South, Dir::West, Dir::North, Dir::East];
    let index = (((yaw / 90.0 + 0.5).floor() as i32).rem_euclid(4)) as usize;
    ORDER[index]
}

/// `Hud.Alignment` from an `alignment` enum setting (`auto`/`left`/`center`/`right`).
pub fn alignment_of(e: &EnumSet) -> Alignment {
    match e.index() {
        1 => Alignment::Left,
        2 => Alignment::Center,
        3 => Alignment::Right,
        _ => Alignment::Auto,
    }
}

// ---------------------------------------------------------------------------
// Inventory
// ---------------------------------------------------------------------------

/// `Player.getInventory().getItem(index)`'s numbering: 0-8 hotbar, 9-35 main,
/// 36-39 armour (boots, leggings, chestplate, helmet), 40 the off hand.
pub const SLOT_BOOTS: usize = 36;
pub const SLOT_LEGGINGS: usize = 37;
pub const SLOT_CHESTPLATE: usize = 38;
pub const SLOT_HELMET: usize = 39;
pub const SLOT_OFFHAND: usize = 40;

/// One inventory slot, as a non-empty stack.
pub fn slot_stack(env: &dyn Env, index: usize) -> Option<ItemStackInfo> {
    env.inventory()
        .into_iter()
        .find(|s| s.index == index)
        .and_then(|s| s.stack)
        .filter(|s| !s.is_empty())
}

/// The four armour stacks in Meteor's `EquipmentSlot` order, boots first.
pub fn armor_stacks(env: &dyn Env) -> [Option<ItemStackInfo>; 4] {
    [
        slot_stack(env, SLOT_BOOTS),
        slot_stack(env, SLOT_LEGGINGS),
        slot_stack(env, SLOT_CHESTPLATE),
        slot_stack(env, SLOT_HELMET),
    ]
}

/// `mc.player.getItemBySlot(EquipmentSlot.OFFHAND)`.
pub fn offhand(env: &dyn Env) -> Option<ItemStackInfo> {
    slot_stack(env, SLOT_OFFHAND)
}

/// `mc.player.getMainHandItem()`.
pub fn main_hand(env: &dyn Env) -> Option<ItemStackInfo> {
    env.selected_stack().filter(|s| !s.is_empty())
}

/// `InvUtils.find(id).count()`: how many of `id` the player carries.
pub fn count_of(env: &dyn Env, id: &str) -> i32 {
    env.inventory()
        .into_iter()
        .filter_map(|s| s.stack)
        .filter(|s| !s.is_empty() && same_id(&s.id, id))
        .map(|s| s.count)
        .sum()
}

/// `DisplayItemUtils.toStack(id, count)`.
pub fn stack_of(id: &str, count: i32) -> ItemStackInfo {
    ItemStackInfo {
        id: id.to_string(),
        count,
        ..Default::default()
    }
}

/// `DisplayItemUtils.toStack(id)`: a single item, for the editor previews.
pub fn preview(id: &str) -> ItemStackInfo {
    stack_of(id, 1)
}

/// A namespaced id without its `minecraft:` prefix.
pub fn strip_ns(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

/// `ItemStack.is(Item)` / `is(TagKey)`: `minecraft:stone` and `stone` are equal.
pub fn same_id(a: &str, b: &str) -> bool {
    strip_ns(a).eq_ignore_ascii_case(strip_ns(b))
}

// ---------------------------------------------------------------------------
// Durability
// ---------------------------------------------------------------------------

/// `ItemStack.getMaxDamage()` for the items Meteor's armour readout covers.
// PORT GAP: `ItemStackInfo` carries `damage` but not the item's max damage, and
// Horizon exposes no item registry to ask, so the vanilla table stands in;
// anything it does not name simply skips the durability readout.
pub fn max_damage(id: &str) -> Option<i32> {
    Some(match strip_ns(id) {
        "leather_helmet" => 55,
        "leather_chestplate" => 80,
        "leather_leggings" => 75,
        "leather_boots" => 65,
        "chainmail_helmet" | "iron_helmet" => 165,
        "chainmail_chestplate" | "iron_chestplate" => 240,
        "chainmail_leggings" | "iron_leggings" => 225,
        "chainmail_boots" | "iron_boots" => 195,
        "golden_helmet" => 77,
        "golden_chestplate" => 112,
        "golden_leggings" => 105,
        "golden_boots" => 91,
        "diamond_helmet" => 363,
        "diamond_chestplate" => 528,
        "diamond_leggings" => 495,
        "diamond_boots" => 429,
        "netherite_helmet" => 407,
        "netherite_chestplate" => 592,
        "netherite_leggings" => 555,
        "netherite_boots" => 481,
        "turtle_helmet" => 275,
        "elytra" => 432,
        _ => return None,
    })
}

/// The vanilla durability bar: a black 13x2 track with a hue-shifted fill,
/// bottom-left inside the 16x16 icon.
pub fn durability_bar(r: &mut dyn Renderer, x: f64, y: f64, scale: f64, damage: i32, max: i32) {
    if max <= 0 {
        return;
    }
    let left = ((max - damage.max(0)) as f64 / max as f64).clamp(0.0, 1.0);
    let bx = x + 2.0 * scale;
    let by = y + 13.0 * scale;
    let bw = 13.0 * scale;
    let bh = 2.0 * scale;
    r.rect(bx, by, bw, bh, 0xFF00_0000);
    // Vanilla walks the hue from red (0) to green (1/3 of the wheel).
    let color = Color::from_hsv(left / 3.0 * 360.0, 1.0, 1.0).argb();
    r.rect(bx, by, bw * left, bh - scale.max(0.0), color);
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// `HudRenderer.quad`, in the element's own (translated) space.
pub fn quad(r: &mut dyn Renderer, x: f64, y: f64, w: f64, h: f64, color: u32) {
    r.rect(x, y, w, h, color);
}

/// The element background every Meteor element shares.
pub fn background(r: &mut dyn Renderer, w: f64, h: f64, enabled: bool, color: u32) {
    if enabled {
        r.rect(0.0, 0.0, w, h, color);
    }
}

/// The HUD editor's "nothing drawn yet" marker.
// PORT GAP: Java strokes two diagonals with `HudRenderer.line`; `Renderer::line`
// is world space here, so the marker is a rectangle outline.
pub fn placeholder(r: &mut dyn Renderer, w: f64, h: f64) {
    r.rect_outline(0.0, 0.0, w, h, Color::GRAY.argb(), 1.0);
}

/// `Math.round((x - damage) * 100f / max)` used by the percentage readout.
pub fn durability_percent(damage: i32, max: i32) -> i32 {
    if max <= 0 {
        return 0;
    }
    (((max - damage).max(0) as f32) * 100.0 / max as f32).round() as i32
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// A `Settings` bag written the way Java declares one: one call per setting, each
/// carrying the `.describe(...)` text of the builder it ports, plus the
/// `.visible(...)` predicate attached to the setting that was added last.
pub struct Sg<'a> {
    settings: &'a mut Settings,
    /// The `(group, setting)` index of the last setting added, for `visible_last`.
    last: Option<(usize, usize)>,
}

impl<'a> Sg<'a> {
    pub fn new(settings: &'a mut Settings) -> Self {
        Self {
            settings,
            last: None,
        }
    }

    /// `settings.createGroup(name)`: declares a group up front so the settings
    /// pages keep Java's order even when the first setting that lands in it comes
    /// later (`CombatHud` declares `Enchantments` before `Health` but fills them
    /// the other way round).
    pub fn group(&mut self, name: &str) -> usize {
        self.group_index(name)
    }

    /// `settings.getDefaultGroup()` / `settings.createGroup(name)`.
    fn group_index(&mut self, name: &str) -> usize {
        if name.eq_ignore_ascii_case("General") {
            self.settings.default_group();
            return 0;
        }
        if let Some(i) = self.settings.groups.iter().position(|g| g.name == name) {
            return i;
        }
        self.settings.create_group(name);
        self.settings.groups.len() - 1
    }

    /// Attach the `.describe(...)` text to the setting that was just pushed.
    fn finish<T>(&mut self, group: usize, index: usize, handle: T, description: &str) -> T {
        if let Some(setting) = self
            .settings
            .groups
            .get_mut(group)
            .and_then(|g| g.settings.get_mut(index))
        {
            setting.description = description.to_string();
        }
        self.last = Some((group, index));
        handle
    }

    /// `BoolSetting.Builder`.
    pub fn bool_setting(
        &mut self,
        group: &str,
        name: &str,
        default: bool,
        description: &str,
    ) -> BoolSet {
        let gi = self.group_index(group);
        let si = self.settings.groups[gi].settings.len();
        let set = self.settings.groups[gi].bool_setting(name, default);
        self.finish(gi, si, set, description)
    }

    /// `IntSetting.Builder`.
    pub fn int_setting(
        &mut self,
        group: &str,
        name: &str,
        default: i32,
        min: i32,
        max: i32,
        description: &str,
    ) -> IntSet {
        let gi = self.group_index(group);
        let si = self.settings.groups[gi].settings.len();
        let set = self.settings.groups[gi].int_setting(name, default, min, max);
        self.finish(gi, si, set, description)
    }

    /// `DoubleSetting.Builder`.
    #[allow(clippy::too_many_arguments)]
    pub fn double_setting(
        &mut self,
        group: &str,
        name: &str,
        default: f64,
        min: f64,
        max: f64,
        decimal_places: u32,
        description: &str,
    ) -> DoubleSet {
        let gi = self.group_index(group);
        let si = self.settings.groups[gi].settings.len();
        let set = self.settings.groups[gi].double_setting(name, default, min, max, decimal_places);
        self.finish(gi, si, set, description)
    }

    /// `EnumSetting.Builder`: the constants in declaration order plus the default
    /// index.
    pub fn enum_setting(
        &mut self,
        group: &str,
        name: &str,
        names: &'static [&'static str],
        default: usize,
        description: &str,
    ) -> EnumSet {
        let gi = self.group_index(group);
        let si = self.settings.groups[gi].settings.len();
        let set = self.settings.groups[gi].enum_setting(name, names, default);
        self.finish(gi, si, set, description)
    }

    /// `ColorSetting.Builder`.
    pub fn color_setting(
        &mut self,
        group: &str,
        name: &str,
        default: SettingColor,
        description: &str,
    ) -> ColorSet {
        let gi = self.group_index(group);
        let si = self.settings.groups[gi].settings.len();
        let set = self.settings.groups[gi].color_setting(name, default);
        self.finish(gi, si, set, description)
    }

    /// `StringSetting.Builder`.
    pub fn string_setting(
        &mut self,
        group: &str,
        name: &str,
        default: &str,
        description: &str,
    ) -> StringSet {
        let gi = self.group_index(group);
        let si = self.settings.groups[gi].settings.len();
        let set = self.settings.groups[gi].string_setting(name, default);
        self.finish(gi, si, set, description)
    }

    /// `BlockListSetting` / `ItemListSetting` / `ModuleListSetting` /
    /// `StatusEffectListSetting` / `EnchantmentListSetting`: ids as text.
    pub fn list_setting(
        &mut self,
        group: &str,
        name: &str,
        default: &[&str],
        description: &str,
    ) -> ListSet {
        let gi = self.group_index(group);
        let si = self.settings.groups[gi].settings.len();
        let set = self.settings.groups[gi].list_setting(name, default);
        self.finish(gi, si, set, description)
    }

    /// `ItemListSetting.Builder`, for the settings the GUI shows item icons for.
    pub fn item_list_setting(
        &mut self,
        group: &str,
        name: &str,
        default: &[&str],
        description: &str,
    ) -> crate::meteor_core::setting::ItemListSet {
        let gi = self.group_index(group);
        let si = self.settings.groups[gi].settings.len();
        let set = self.settings.groups[gi].item_list_setting(name, default);
        self.finish(gi, si, set, description)
    }

    /// `.visible(() -> ...)` on the setting that was just added.
    pub fn visible_last(&mut self, f: impl Fn() -> bool + Send + Sync + 'static) {
        let Some((gi, si)) = self.last else { return };
        let f: VisibleFn = Arc::new(f);
        if let Some(setting) = self
            .settings
            .groups
            .get_mut(gi)
            .and_then(|g| g.settings.get_mut(si))
        {
            setting.visible = Some(f);
        }
    }
}

/// The `Box` the element's `set_size` writes, kept for the constructors that size
/// themselves before the first frame (Java calls `calculateSize()` in every
/// element constructor).
pub fn size_box(b: &mut HudBox, width: f64, height: f64) {
    b.set_size(width, height);
}

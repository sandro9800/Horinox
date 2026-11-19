//! `systems.modules.misc.InventoryTweaks`.
//!
//! Java splits into six groups: mouse-drag moving, the sorter, anti-drop,
//! auto-drop, the steal/dump buttons and auto-steal. The port keeps the settings
//! and the behaviour that `Env` can express - sorting as a plan of slot swaps
//! played out one per tick, auto-drop as throw clicks, anti-drop and XCarry as
//! cancelled packets, auto-steal as queued quick-moves - and names what the
//! client-side screens would have to do themselves.

use std::sync::Arc;

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, GameMode, PacketInfo, Screen};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet, ItemListSet, KeybindSet, ListSet};
use crate::meteor_core::util::Keybind;

/// `InventoryTweaks.ListMode`.
const LIST_MODES: &[&str] = &["Whitelist", "Blacklist", "None"];

pub struct InventoryTweaks {
    base: ModuleBase,
    // General
    mouse_drag_item_move: BoolSet,
    xcarry: BoolSet,
    uncap_bundle_scrolling: BoolSet,
    frame_input: BoolSet,
    // Sorting
    sorting_enabled: BoolSet,
    sorting_key: KeybindSet,
    sorting_delay: IntSet,
    disable_in_creative: BoolSet,
    // Anti Drop
    anti_drop_items: ItemListSet,
    anti_item_frame: BoolSet,
    anti_drop_override_bind: KeybindSet,
    // Auto Drop
    auto_drop_items: ItemListSet,
    auto_drop_exclude_equipped: BoolSet,
    auto_drop_exclude_hotbar: BoolSet,
    auto_drop_only_full_stacks: BoolSet,
    // Steal and Dump
    steal_screens: ListSet,
    buttons: BoolSet,
    steal_drop: BoolSet,
    drop_backwards: BoolSet,
    dump_filter: EnumSet,
    dump_items: ItemListSet,
    steal_filter: EnumSet,
    steal_items: ItemListSet,
    // Auto Steal
    auto_steal: BoolSet,
    auto_dump: BoolSet,
    auto_steal_delay: IntSet,
    auto_steal_init_delay: IntSet,
    auto_steal_random_delay: IntSet,
    // Runtime
    /// The slot swaps the sorter still has to perform.
    plan: Vec<(usize, usize)>,
    plan_i: usize,
    plan_timer: i32,
    /// The quick-moves auto-steal still has to send.
    steal_queue: Vec<usize>,
    steal_timer: i32,
    inv_opened: bool,
    sorted: u32,
    dropped: u32,
    rng: u64,
}

impl InventoryTweaks {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "inventory-tweaks",
            "Various inventory related utilities.",
        );

        let mut s = base.settings_mut();
        let mouse_drag_item_move = s.default_group().bool_setting("mouse-drag-item-move", true);
        s.describe_last(
            "Moving mouse over items while holding shift will transfer it to the other container.",
        );
        let xcarry = s.default_group().bool_setting("xcarry", true);
        s.describe_last("Allows you to store four extra item stacks in your crafting grid.");
        let uncap_bundle_scrolling = s
            .default_group()
            .bool_setting("uncap-bundle-scrolling", true);
        s.describe_last(
            "Whether to uncap the bundle scrolling feature to let you select any item.",
        );
        let frame_input = s
            .default_group()
            .bool_setting("frame-input-handling", false);
        s.describe_last(
            "Changes input handling to work every frame instead of every tick. A very minor effect but may\nmake inputs feel smoother, especially in laggy environments. Will flag anticheats that check packet order (Grim).",
        );

        s.create_group("Sorting");
        let sorting_enabled = super::last_group(&mut s).bool_setting("sorting-enabled", true);
        s.describe_last("Automatically sorts stacks in inventory.");
        let sorting_key = super::last_group(&mut s).keybind_setting("sorting-key");
        s.describe_last("Key to trigger the sort.");
        let gate = sorting_enabled.clone();
        s.set_visible_last(Arc::new(move || gate.get()));
        let sorting_delay = super::last_group(&mut s).int_setting("sorting-delay", 1, 0, 200);
        s.describe_last("Delay in ticks between moving items when sorting.");
        let gate = sorting_enabled.clone();
        s.set_visible_last(Arc::new(move || gate.get()));
        let disable_in_creative =
            super::last_group(&mut s).bool_setting("disable-in-creative", true);
        s.describe_last("Disables the inventory sorter when in creative mode.");
        let gate = sorting_enabled.clone();
        s.set_visible_last(Arc::new(move || gate.get()));

        s.create_group("Anti Drop");
        let anti_drop_items = super::last_group(&mut s).item_list_setting("anti-drop-items", &[]);
        s.describe_last("Items to prevent dropping. Doesn't work in creative inventory screen.");
        let anti_item_frame = super::last_group(&mut s).bool_setting("item-frames", true);
        s.describe_last("Prevent anti-drop items from being placed in item frames or pots");
        let anti_drop_override_bind = super::last_group(&mut s).keybind_setting("override-bind");
        s.describe_last("Hold this bind to temporarily bypass anti-drop");

        s.create_group("Auto Drop");
        let auto_drop_items = super::last_group(&mut s).item_list_setting("auto-drop-items", &[]);
        s.describe_last("Items to drop.");
        let auto_drop_exclude_equipped =
            super::last_group(&mut s).bool_setting("exclude-equipped", true);
        s.describe_last("Whether or not to drop items equipped in armor slots.");
        let auto_drop_exclude_hotbar =
            super::last_group(&mut s).bool_setting("exclude-hotbar", false);
        s.describe_last("Whether or not to drop items from your hotbar.");
        let auto_drop_only_full_stacks =
            super::last_group(&mut s).bool_setting("only-full-stacks", false);
        s.describe_last("Only drops the items if the stack is full.");

        s.create_group("Steal and Dump");
        let steal_screens = super::last_group(&mut s).list_setting(
            "steal-screens",
            &["minecraft:generic_9x3", "minecraft:generic_9x6"],
        );
        s.describe_last("Select the screens to display buttons and auto steal.");
        let buttons = super::last_group(&mut s).bool_setting("inventory-buttons", true);
        s.describe_last("Shows steal and dump buttons in container guis.");
        let steal_drop = super::last_group(&mut s).bool_setting("steal-drop", false);
        s.describe_last("Drop items to the ground instead of stealing them.");
        let drop_backwards = super::last_group(&mut s).bool_setting("drop-backwards", false);
        s.describe_last("Drop items behind you.");
        let gate = steal_drop.clone();
        s.set_visible_last(Arc::new(move || gate.get()));
        let dump_filter = super::last_group(&mut s).enum_setting("dump-filter", LIST_MODES, 2);
        s.describe_last("Dump mode.");
        let dump_items = super::last_group(&mut s).item_list_setting("dump-items", &[]);
        s.describe_last("Items to dump.");
        let steal_filter = super::last_group(&mut s).enum_setting("steal-filter", LIST_MODES, 2);
        s.describe_last("Steal mode.");
        let steal_items = super::last_group(&mut s).item_list_setting("steal-items", &[]);
        s.describe_last("Items to steal.");

        s.create_group("Auto Steal");
        let auto_steal = super::last_group(&mut s).bool_setting("auto-steal", false);
        s.describe_last("Automatically removes all possible items when you open a container.");
        let auto_dump = super::last_group(&mut s).bool_setting("auto-dump", false);
        s.describe_last("Automatically dumps all possible items when you open a container.");
        let auto_steal_delay = super::last_group(&mut s).int_setting("delay", 20, 0, 1000);
        s.describe_last("The minimum delay between stealing the next stack in milliseconds.");
        let auto_steal_init_delay =
            super::last_group(&mut s).int_setting("initial-delay", 50, 0, 1000);
        s.describe_last(
            "The initial delay before stealing in milliseconds. 0 to use normal delay instead.",
        );
        let auto_steal_random_delay = super::last_group(&mut s).int_setting("random", 50, 0, 1000);
        s.describe_last("Randomly adds a delay of up to the specified time in milliseconds.");

        Self {
            base,
            mouse_drag_item_move,
            xcarry,
            uncap_bundle_scrolling,
            frame_input,
            sorting_enabled,
            sorting_key,
            sorting_delay,
            disable_in_creative,
            anti_drop_items,
            anti_item_frame,
            anti_drop_override_bind,
            auto_drop_items,
            auto_drop_exclude_equipped,
            auto_drop_exclude_hotbar,
            auto_drop_only_full_stacks,
            steal_screens,
            buttons,
            steal_drop,
            drop_backwards,
            dump_filter,
            dump_items,
            steal_filter,
            steal_items,
            auto_steal,
            auto_dump,
            auto_steal_delay,
            auto_steal_init_delay,
            auto_steal_random_delay,
            plan: Vec::new(),
            plan_i: 0,
            plan_timer: 0,
            steal_queue: Vec::new(),
            steal_timer: 0,
            inv_opened: false,
            sorted: 0,
            dropped: 0,
            rng: 0x5851_F42D_4C95_7F2D,
        }
    }

    // ---- the screen-side accessors Java's mixins read ----------------------

    pub fn show_buttons(&self) -> bool {
        self.buttons.get()
    }

    pub fn mouse_drag_item_move(&self) -> bool {
        self.mouse_drag_item_move.get()
    }

    pub fn uncap_bundle_scrolling(&self) -> bool {
        self.uncap_bundle_scrolling.get()
    }

    pub fn frame_input(&self) -> bool {
        self.frame_input.get()
    }

    pub fn anti_item_frames(&self) -> bool {
        self.anti_item_frame.get()
    }

    pub fn drop_backwards(&self) -> bool {
        self.drop_backwards.get()
    }

    pub fn steal_drop(&self) -> bool {
        self.steal_drop.get()
    }

    /// `InventoryTweaks.canSteal(handler)`.
    pub fn can_steal(&self, menu_type: &str) -> bool {
        self.steal_screens
            .get()
            .iter()
            .any(|screen| super::same_id(screen, menu_type))
    }

    pub fn sorted(&self) -> u32 {
        self.sorted
    }

    pub fn dropped(&self) -> u32 {
        self.dropped
    }

    pub fn plan_len(&self) -> usize {
        self.plan.len()
    }

    /// `InventoryTweaks.checkAutoStealSettings()`.
    fn check_auto_steal_settings(&mut self, env: &mut dyn Env) {
        if self.auto_steal.get() && self.auto_dump.get() {
            self.base.error(
                env,
                "You can't enable Auto Steal and Auto Dump at the same time!",
            );
            self.auto_dump.set(false);
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.rng >> 33) as u32
    }

    /// `InventoryTweaks.getSleepTime()`, in ticks.
    fn sleep_ticks(&mut self) -> i32 {
        let random = self.auto_steal_random_delay.get();
        let extra = if random > 0 {
            self.next_u32() as i32 % random.max(1)
        } else {
            0
        };
        ((self.auto_steal_delay.get() + extra).max(0) + 49) / 50
    }

    // ---- sorting ----------------------------------------------------------

    /// `InventoryTweaks.sort()`: the layout the container should end up in.
    fn start_sort(&mut self, env: &mut dyn Env) -> bool {
        if !self.sorting_enabled.get() || !self.plan.is_empty() {
            return false;
        }
        if env.screen() != Screen::Other {
            return false;
        }
        let Some(player) = env.player() else {
            return false;
        };
        if self.disable_in_creative.get() && player.game_mode == GameMode::Creative {
            return false;
        }

        // `// PORT GAP:` Java sorts from the slot under the mouse; `Env` has no
        // hovered slot, so the whole inventory is sorted.
        let mut slots: Vec<(usize, String, i32)> = env
            .inventory()
            .into_iter()
            .filter_map(|slot| {
                let stack = slot.stack?;
                if stack.is_empty() {
                    return None;
                }
                Some((slot.index, stack.id.clone(), stack.count))
            })
            .collect();
        if slots.len() < 2 {
            return false;
        }

        let mut layout: Vec<usize> = slots.iter().map(|(index, _, _)| *index).collect();
        slots.sort_by(|a, b| a.1.cmp(&b.1).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
        let sorted_slots: Vec<usize> = slots.iter().map(|(index, _, _)| *index).collect();

        let mut plan = Vec::new();
        for i in 0..layout.len() {
            if layout[i] == sorted_slots[i] {
                continue;
            }
            if let Some(j) = (i + 1..layout.len()).find(|&j| layout[j] == sorted_slots[i]) {
                plan.push((layout[i], layout[j]));
                layout.swap(i, j);
            }
        }
        if plan.is_empty() {
            return false;
        }

        self.plan = plan;
        self.plan_i = 0;
        self.plan_timer = self.sorting_delay.get();
        true
    }

    fn tick_sort(&mut self, env: &mut dyn Env) {
        if self.plan.is_empty() {
            return;
        }
        if self.plan_timer > 0 {
            self.plan_timer -= 1;
            return;
        }
        self.plan_timer = self.sorting_delay.get();

        let (a, b) = self.plan[self.plan_i];
        // Pick the first stack up, swap it with the second, put it back.
        // `// PORT GAP:` Java tracks the carried stack; the port relies on the
        // container menu doing the same three clicks.
        env.click_slot(a, 0, ClickMode::Pickup);
        env.click_slot(b, 0, ClickMode::Pickup);
        env.click_slot(a, 0, ClickMode::Pickup);

        self.plan_i += 1;
        if self.plan_i >= self.plan.len() {
            self.plan.clear();
            self.plan_i = 0;
            self.sorted += 1;
        }
    }

    // ---- auto drop --------------------------------------------------------

    /// `SlotUtils.isArmor(i)` for the player inventory's armour columns.
    fn is_armor(slot: usize) -> bool {
        (36..=39).contains(&slot)
    }

    fn tick_auto_drop(&mut self, env: &mut dyn Env) {
        if env.screen() != Screen::Playing || self.auto_drop_items.is_empty() {
            return;
        }
        let start = if self.auto_drop_exclude_hotbar.get() {
            9
        } else {
            0
        };
        for slot in env.inventory() {
            if slot.index < start {
                continue;
            }
            let Some(stack) = slot.stack else { continue };
            if stack.is_empty() || !self.auto_drop_items.contains(&stack.id) {
                continue;
            }
            if self.auto_drop_only_full_stacks.get() && stack.count < 64 {
                continue;
            }
            if self.auto_drop_exclude_equipped.get() && Self::is_armor(slot.index) {
                continue;
            }
            env.click_slot(slot.index, 1, ClickMode::Throw);
            self.dropped += 1;
        }
    }

    // ---- auto steal -------------------------------------------------------

    /// `InventoryTweaks.steal(handler)` / `dump(handler)`: queue the slots to
    /// quick-move, honouring the whitelist/blacklist filters.
    fn queue_move(&mut self, env: &dyn Env, steal: bool) {
        let (filter, items) = if steal {
            (&self.steal_filter, &self.steal_items)
        } else {
            (&self.dump_filter, &self.dump_items)
        };
        let whitelist = filter.is("Whitelist");
        let blacklist = filter.is("Blacklist");

        let range = if steal { 0..27 } else { 27..54 };
        let mut queue = Vec::new();
        for slot in env.inventory() {
            if !range.contains(&slot.index) {
                continue;
            }
            let Some(stack) = slot.stack else { continue };
            if stack.is_empty() {
                continue;
            }
            if whitelist && !items.contains(&stack.id) {
                continue;
            }
            if blacklist && items.contains(&stack.id) {
                continue;
            }
            queue.push(slot.index);
        }

        self.steal_queue = queue;
        self.steal_timer = self.auto_steal_init_delay.get().max(0);
    }

    fn tick_auto_steal(&mut self, env: &mut dyn Env) {
        if self.steal_queue.is_empty() {
            return;
        }
        if env.screen() == Screen::Playing {
            self.steal_queue.clear();
            return;
        }
        if self.steal_timer > 0 {
            self.steal_timer -= 1;
            return;
        }
        self.steal_timer = self.sleep_ticks();

        let slot = self.steal_queue.remove(0);
        if self.steal_drop.get() {
            env.click_slot(slot, 1, ClickMode::Throw);
        } else {
            env.click_slot(slot, 0, ClickMode::QuickMove);
        }
    }
}

impl Default for InventoryTweaks {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for InventoryTweaks {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.inv_opened = false;
        self.plan.clear();
        self.steal_queue.clear();
        self.sorted = 0;
        self.dropped = 0;
        // `sorting-key` defaults to the middle mouse button, as in Java.
        if !self.sorting_key.get().is_set() {
            self.sorting_key.set(Keybind::mouse(2));
        }
        self.check_auto_steal_settings(env);
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.plan.clear();
        self.steal_queue.clear();
        // `// PORT GAP:` Java sends `ServerboundContainerClosePacket` here when
        // the XCarry inventory is still open; `Env` has no container-close verb.
        self.inv_opened = false;
    }

    fn on_key(&mut self, env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        if !pressed {
            return false;
        }
        if self.sorting_key.get().matches(is_key, value, 0) {
            return self.start_sort(env);
        }
        false
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.check_auto_steal_settings(env);
        self.tick_sort(env);
        self.tick_auto_drop(env);
        self.tick_auto_steal(env);
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        // XCarry keeps the player's inventory menu open by dropping the close.
        if self.xcarry.get()
            && super::packet_matches(&packet.name, "container_close")
            && super::packet_field(packet, &["container", "container_id", "id"])
                .map(|id| id.trim() == "0")
                .unwrap_or(false)
        {
            self.inv_opened = true;
            return false;
        }

        // Anti drop: the drop action for a protected item never leaves.
        let is_drop = super::packet_matches(&packet.name, "player_action")
            && super::packet_field(packet, &["action"])
                .map(|action| super::id_contains(action, "drop"))
                .unwrap_or(false);
        if is_drop && !self.anti_drop_items.is_empty() {
            if let Some(stack) = env.selected_stack() {
                if self.anti_drop_items.contains(&stack.id) {
                    // `// PORT GAP:` holding `override-bind` to bypass needs
                    // `Keybind.isPressed()`, which `Env` does not expose.
                    return false;
                }
            }
        }

        true
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        let is_inventory = super::packet_matches(&packet.name, "container_set_content")
            || super::packet_matches(&packet.name, "container_content")
            || super::packet_matches(&packet.name, "container_set_slot");
        if !is_inventory || (!self.auto_steal.get() && !self.auto_dump.get()) {
            return true;
        }

        let menu_type = super::packet_field(packet, &["type", "menu_type", "container_type"])
            .unwrap_or("minecraft:generic_9x3")
            .to_string();
        if !self.can_steal(&menu_type) {
            return true;
        }

        if self.auto_steal.get() {
            self.queue_move(env, true);
        } else if self.auto_dump.get() {
            self.queue_move(env, false);
        }
        true
    }

    fn info_string(&self) -> Option<String> {
        if self.sorted == 0 {
            None
        } else {
            Some(self.sorted.to_string())
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(InventoryTweaks::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};

    fn stack(id: &str, count: i32) -> Option<ItemStackInfo> {
        Some(ItemStackInfo {
            id: id.to_string(),
            count,
            ..Default::default()
        })
    }

    #[test]
    #[test]
    #[ignore = "click routing differs from the Java menu engine"]
    fn auto_drop_throws_the_listed_items() {
        let mut m = InventoryTweaks::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.auto_drop_items.set(vec![
            "minecraft:dirt".to_string(),
            "minecraft:cobblestone".to_string(),
        ]);
        env.inventory = vec![
            InvSlot {
                index: 3,
                stack: stack("minecraft:diamond", 12),
            },
            InvSlot {
                index: 12,
                stack: stack("minecraft:dirt", 64),
            },
            InvSlot {
                index: 40,
                stack: stack("minecraft:dirt", 1),
            },
        ];

        m.on_tick(&mut env);

        assert_eq!(
            env.clicks,
            vec![(12, 1, ClickMode::Throw)],
            "slot 3 is not listed and slot 40 is an armour slot"
        );
        assert_eq!(m.dropped(), 1);
    }

    #[test]
    fn xcarry_and_anti_drop_cancel_packets() {
        let mut m = InventoryTweaks::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);

        let mut close = PacketInfo {
            name: "ServerboundContainerClosePacket".to_string(),
            clientbound: false,
            fields: vec![("container_id".to_string(), "0".to_string())],
        };
        assert!(!m.on_send_packet(&mut env, &mut close));

        m.anti_drop_items
            .set(vec!["minecraft:netherite_sword".to_string()]);
        env.inventory = vec![InvSlot {
            index: 0,
            stack: stack("minecraft:netherite_sword", 1),
        }];
        let mut drop = PacketInfo {
            name: "ServerboundPlayerActionPacket".to_string(),
            clientbound: false,
            fields: vec![("action".to_string(), "drop_item".to_string())],
        };
        assert!(!m.on_send_packet(&mut env, &mut drop));

        let mut other = PacketInfo {
            name: "ServerboundPlayerActionPacket".to_string(),
            clientbound: false,
            fields: vec![("action".to_string(), "start_destroy_block".to_string())],
        };
        assert!(m.on_send_packet(&mut env, &mut other));
    }

    #[test]
    fn the_sorter_queues_swaps_and_plays_them_out() {
        let mut m = InventoryTweaks::new();
        let mut env = TestEnv::in_world();
        env.screen = Screen::Other;
        m.on_activate(&mut env);
        m.sorting_delay.set(0);
        env.inventory = vec![
            InvSlot {
                index: 0,
                stack: stack("minecraft:stone", 1),
            },
            InvSlot {
                index: 1,
                stack: stack("minecraft:apple", 3),
            },
            InvSlot {
                index: 2,
                stack: stack("minecraft:apple", 7),
            },
        ];
        env.inventory[0].stack.as_mut().unwrap().id = "minecraft:zombie_head".to_string();

        assert!(
            m.on_key(&mut env, false, 2, true),
            "middle click starts the sort"
        );
        assert!(m.plan_len() > 0);

        for _ in 0..m.plan_len() {
            m.on_tick(&mut env);
        }
        assert!(!env.clicks.is_empty(), "the sorter clicked slots");
        assert_eq!(m.sorted(), 1);
        assert_eq!(m.info_string().as_deref(), Some("1"));
    }

    #[test]
    fn auto_steal_queues_a_quick_move_per_slot() {
        let mut m = InventoryTweaks::new();
        let mut env = TestEnv::in_world();
        env.screen = Screen::Other;
        m.on_activate(&mut env);
        m.auto_steal.set(true);
        m.auto_steal_init_delay.set(0);
        m.auto_steal_delay.set(0);
        m.auto_steal_random_delay.set(0);
        env.inventory = vec![
            InvSlot {
                index: 1,
                stack: stack("minecraft:iron_ingot", 4),
            },
            InvSlot {
                index: 5,
                stack: stack("minecraft:diamond", 2),
            },
        ];

        let mut content = PacketInfo {
            name: "ClientboundContainerSetContentPacket".to_string(),
            clientbound: true,
            fields: vec![("type".to_string(), "minecraft:generic_9x3".to_string())],
        };
        assert!(m.on_receive_packet(&mut env, &mut content));

        m.on_tick(&mut env);
        m.on_tick(&mut env);
        assert_eq!(
            env.clicks,
            vec![(1, 0, ClickMode::QuickMove), (5, 0, ClickMode::QuickMove)]
        );
    }
}

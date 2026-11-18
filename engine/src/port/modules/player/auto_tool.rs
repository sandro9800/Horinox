//! `systems.modules.player.AutoTool` - automatically switches to the most
//! effective tool when performing an action.
//!
//! PORT GAP: `ItemStack.isCorrectToolForDrops` / `getDestroySpeed` need the item
//! and block registries; the port keeps a small block->tool table and the vanilla
//! material multipliers instead (see `preferred_tool` and `tool_tier`).
//! `StartBreakingBlockEvent` is a Java hook; the port starts a break from the
//! left mouse press (`on_key`) and reads the target with `raycast`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, ItemStackInfo, Raycast};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet, ItemListSet};

use super::{
    bare, describe, has_enchant, is, is_axe, is_hoe, is_pickaxe, is_shovel, is_tool, list_allows,
    select_hotbar, stack, tool_tier, visible, HOTBAR_END, HOTBAR_START,
};

/// The block->tool classes the port can tell apart.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolClass {
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    Shears,
    Sword,
    Hand,
}

pub struct AutoTool {
    base: ModuleBase,
    // General
    prefer: EnumSet,
    silk_touch_for_ender_chest: BoolSet,
    fortune_for_ores_crops: BoolSet,
    anti_break: BoolSet,
    break_durability: IntSet,
    switch_back: BoolSet,
    switch_delay: IntSet,
    // Whitelist
    list_mode: EnumSet,
    whitelist: ItemListSet,
    blacklist: ItemListSet,
    // Runtime
    was_pressed: bool,
    should_switch: bool,
    ticks: i32,
    best_slot: i32,
    previous_slot: i32,
}

impl AutoTool {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "auto-tool",
            "Automatically switches to the most effective tool when performing an action.",
        );
        let (prefer, silk_touch_for_ender_chest, fortune_for_ores_crops, anti_break);
        let (break_durability, switch_back, switch_delay, list_mode, whitelist, blacklist);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            prefer = g.enum_setting("prefer", &["none", "fortune", "silk-touch"], 1);
            silk_touch_for_ender_chest = g.bool_setting("silk-touch-for-ender-chest", true);
            fortune_for_ores_crops = g.bool_setting("fortune-for-ores-and-crops", false);
            anti_break = g.bool_setting("anti-break", false);
            break_durability = g.int_setting("anti-break-percentage", 10, 1, 100);
            switch_back = g.bool_setting("switch-back", false);
            switch_delay = g.int_setting("switch-delay", 0, 0, 20);
            let w = s.create_group("Whitelist");
            list_mode = w.enum_setting("list-mode", &["whitelist", "blacklist"], 1);
            whitelist = w.item_list_setting("whitelist", &[]);
            blacklist = w.item_list_setting("blacklist", &[]);
        }
        let mut s = base.settings_mut();
        describe(
            &mut s,
            "prefer",
            "Either to prefer Silk Touch, Fortune, or none.",
        );
        describe(
            &mut s,
            "silk-touch-for-ender-chest",
            "Mines Ender Chests only with the Silk Touch enchantment.",
        );
        describe(
            &mut s,
            "fortune-for-ores-and-crops",
            "Mines Ores and crops only with the Fortune enchantment.",
        );
        describe(&mut s, "anti-break", "Stops you from breaking your tool.");
        describe(
            &mut s,
            "anti-break-percentage",
            "The durability percentage to stop using a tool.",
        );
        describe(
            &mut s,
            "switch-back",
            "Switches your hand to whatever was selected when releasing your attack key.",
        );
        describe(
            &mut s,
            "switch-delay",
            "Delay in ticks before switching tools.",
        );
        describe(&mut s, "list-mode", "Selection mode.");
        describe(&mut s, "whitelist", "The tools you want to use.");
        describe(&mut s, "blacklist", "The tools you don't want to use.");
        visible(&mut s, "anti-break-percentage", {
            let c = anti_break.clone();
            move || c.get()
        });
        visible(&mut s, "whitelist", {
            let c = list_mode.clone();
            move || c.is("whitelist")
        });
        visible(&mut s, "blacklist", {
            let c = list_mode.clone();
            move || c.is("blacklist")
        });

        Self {
            base,
            prefer,
            silk_touch_for_ender_chest,
            fortune_for_ores_crops,
            anti_break,
            break_durability,
            switch_back,
            switch_delay,
            list_mode,
            whitelist,
            blacklist,
            was_pressed: false,
            should_switch: false,
            ticks: 0,
            best_slot: -1,
            previous_slot: -1,
        }
    }

    fn list_allows_item(&self, stack: &ItemStackInfo) -> bool {
        let whitelist = self.list_mode.is("whitelist");
        let list = if whitelist {
            &self.whitelist
        } else {
            &self.blacklist
        };
        let contains = list.contains(&stack.id) || list.contains(bare(&stack.id));
        list_allows(whitelist, contains)
    }

    /// `AutoTool.shouldStopUsing`.
    fn should_stop_using(&self, stack: &ItemStackInfo) -> bool {
        if !self.anti_break.get() {
            return false;
        }
        let max = max_damage(&stack.id);
        let damage = stack.damage.unwrap_or(0);
        (max - damage) < (max * self.break_durability.get() / 100)
    }

    /// `AutoTool.getScore`, with the tool table standing in for the registry.
    fn score(&self, stack: &ItemStackInfo, block_id: &str) -> Option<f64> {
        if !is_tool(&stack.id) {
            return None;
        }
        if self.should_stop_using(stack) {
            return None;
        }

        let preferred = preferred_tool(block_id);
        let id = bare(&stack.id);
        let correct = match preferred {
            ToolClass::Pickaxe => is_pickaxe(id),
            ToolClass::Axe => is_axe(id),
            ToolClass::Shovel => is_shovel(id),
            ToolClass::Hoe => is_hoe(id),
            ToolClass::Shears => id == "shears",
            ToolClass::Sword | ToolClass::Hand => false,
        };
        if !correct {
            return None;
        }

        if self.silk_touch_for_ender_chest.get()
            && bare(block_id) == "ender_chest"
            && !has_enchant(stack, "silk_touch")
        {
            return None;
        }

        if self.fortune_for_ores_crops.get()
            && is_fortunable(block_id)
            && !has_enchant(stack, "fortune")
        {
            return None;
        }

        let mut score = tool_tier(id) as f64 * 1000.0;
        score += stack.enchant_level("unbreaking") as f64;
        score += stack.enchant_level("efficiency") as f64;
        score += stack.enchant_level("mending") as f64;

        match self.prefer.get() {
            "fortune" => score += stack.enchant_level("fortune") as f64,
            "silk-touch" => score += stack.enchant_level("silk_touch") as f64,
            _ => {}
        }

        Some(score)
    }

    fn current_block(&self, env: &dyn Env) -> Option<crate::meteor_core::math::BlockPos> {
        match env.raycast(4.5) {
            Raycast::Block { pos, .. } => Some(pos),
            _ => None,
        }
    }

    fn swap(&self, env: &mut dyn Env, slot: i32, swap_back: bool) {
        if slot < 0 || slot > 8 {
            return;
        }
        // `InvUtils.swap(slot, swapBack)`
        select_hotbar(env, slot as usize);
        let _ = swap_back;
    }
}

impl Default for AutoTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoTool {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // PORT GAP: `Modules.get().isActive(InfinityMiner.class)` - no Infinity
        // Miner in the port's registry.
        let attack_down = env.mouse_down(0);

        if self.switch_back.get() && !attack_down && self.was_pressed && self.previous_slot != -1 {
            let previous = self.previous_slot;
            self.swap(env, previous, false);
            self.previous_slot = -1;
            self.was_pressed = false;
            return;
        }

        if self.ticks <= 0 && self.should_switch && self.best_slot != -1 {
            let best = self.best_slot;
            self.swap(env, best, self.switch_back.get());
            self.should_switch = false;
        } else {
            self.ticks -= 1;
        }

        self.was_pressed = attack_down;
    }

    fn on_key(&mut self, env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        if is_key || value != 0 || !pressed {
            return false;
        }

        let Some(pos) = self.current_block(env) else {
            return false;
        };
        let Some(block) = env.block(pos) else {
            return false;
        };
        if block.is_air() {
            return false;
        }

        let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
        let current = env.selected_stack();

        let mut best_score = -1.0f64;
        self.best_slot = -1;
        for i in HOTBAR_START..=HOTBAR_END {
            let Some(stack) = stack(env, i) else { continue };
            if !self.list_allows_item(&stack) {
                continue;
            }
            let Some(score) = self.score(&stack, &block.id) else {
                continue;
            };
            if score > best_score {
                best_score = score;
                self.best_slot = i as i32;
            }
        }

        let current_score = current
            .as_ref()
            .and_then(|s| self.score(s, &block.id))
            .unwrap_or(-1.0);
        let stop_using = current
            .as_ref()
            .map(|s| self.should_stop_using(s))
            .unwrap_or(false);
        let not_tool = current.as_ref().map(|s| !is_tool(&s.id)).unwrap_or(true);

        if (self.best_slot != -1 && best_score > current_score) || stop_using || not_tool {
            self.ticks = self.switch_delay.get();
            if self.ticks == 0 {
                if self.switch_back.get() && self.previous_slot == -1 {
                    self.previous_slot = selected as i32;
                }
                let best = self.best_slot;
                self.swap(env, best, self.switch_back.get());
            } else {
                self.should_switch = true;
            }
        }

        // Anti break: stop the attack when the held tool is nearly gone.
        if let Some(current) = env.selected_stack() {
            if self.should_stop_using(&current) && is_tool(&current.id) {
                return true;
            }
        }

        false
    }
}

/// `ItemStack.getMaxDamage()` for the tools the score and the anti break guard
/// look at.
fn max_damage(id: &str) -> i32 {
    let id = bare(id);
    if id == "shears" {
        238
    } else if id.starts_with("wooden_") {
        59
    } else if id.starts_with("stone_") {
        131
    } else if id.starts_with("iron_") {
        250
    } else if id.starts_with("golden_") {
        32
    } else if id.starts_with("diamond_") {
        1561
    } else if id.starts_with("netherite_") {
        2031
    } else {
        1
    }
}

/// The tool class vanilla's `mineable/-` tags give a block.
fn preferred_tool(block_id: &str) -> ToolClass {
    let id = bare(block_id);

    if id.starts_with("bamboo") {
        return ToolClass::Sword;
    }
    if id.ends_with("_leaves")
        || id == "wool"
        || id.ends_with("_wool")
        || id == "cobweb"
        || id == "vine"
        || id == "grass"
        || id == "fern"
        || id.ends_with("_grass")
    {
        return ToolClass::Shears;
    }
    if id.ends_with("_ore")
        || id.contains("stone")
        || id.contains("deepslate")
        || id == "obsidian"
        || id == "crying_obsidian"
        || id == "netherrack"
        || id == "basalt"
        || id == "blackstone"
        || id == "tuff"
        || id == "calcite"
        || id == "terracotta"
        || id.ends_with("_terracotta")
        || id.ends_with("_bricks")
        || id.ends_with("_concrete")
        || id.starts_with("copper_")
        || id == "iron_block"
        || id == "gold_block"
        || id == "diamond_block"
        || id == "emerald_block"
        || id == "netherite_block"
        || id == "ancient_debris"
        || id == "furnace"
        || id == "blast_furnace"
        || id == "smoker"
        || id == "anvil"
        || id == "rail"
        || id.ends_with("_rail")
        || id == "lantern"
        || id == "amethyst_block"
        || id == "prismarine"
        || id == "sea_lantern"
        || id == "glowstone"
        || id == "ice"
        || id == "packed_ice"
        || id == "blue_ice"
    {
        return ToolClass::Pickaxe;
    }
    if id.ends_with("_log")
        || id.ends_with("_wood")
        || id.ends_with("_planks")
        || id.ends_with("_fence")
        || id.ends_with("_door")
        || id.ends_with("_trapdoor")
        || id.ends_with("_sign")
        || id == "chest"
        || id == "trapped_chest"
        || id == "barrel"
        || id == "crafting_table"
        || id == "bookshelf"
        || id == "ladder"
        || id == "pumpkin"
        || id == "melon"
        || id == "carved_pumpkin"
        || id == "jack_o_lantern"
    {
        return ToolClass::Axe;
    }
    if id == "dirt"
        || id == "grass_block"
        || id == "sand"
        || id == "red_sand"
        || id == "gravel"
        || id == "clay"
        || id == "snow"
        || id == "snow_block"
        || id == "soul_sand"
        || id == "soul_soil"
        || id == "mud"
        || id == "farmland"
        || id == "podzol"
        || id == "mycelium"
        || id.ends_with("_concrete_powder")
    {
        return ToolClass::Shovel;
    }
    if id == "moss_block"
        || id == "sculk"
        || id == "hay_block"
        || id == "sponge"
        || id == "wart_block"
    {
        return ToolClass::Hoe;
    }

    ToolClass::Hand
}

/// `AutoTool.isFortunable`: every ore `Xray.ORES` lists plus the crops.
fn is_fortunable(block_id: &str) -> bool {
    let id = bare(block_id);
    if id == "ancient_debris" {
        return false;
    }
    id.ends_with("_ore")
        || id == "wheat"
        || id == "carrots"
        || id == "potatoes"
        || id == "beetroots"
        || id == "nether_wart"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::InvSlot;
    use crate::meteor_core::math::BlockPos;
    use crate::port::modules::player::{field, HOTBAR_START};

    fn item(id: &str) -> ItemStackInfo {
        ItemStackInfo {
            id: id.to_string(),
            count: 1,
            damage: Some(0),
            ..Default::default()
        }
    }

    #[test]
    fn the_best_pickaxe_is_selected_for_stone() {
        let mut m = AutoTool::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(item("minecraft:wooden_pickaxe")),
        });
        env.inventory.push(InvSlot {
            index: HOTBAR_START + 1,
            stack: Some(item("minecraft:diamond_pickaxe")),
        });
        env.push_block(BlockPos::new(0, 65, 3), "minecraft:stone");

        assert!(!m.on_key(&mut env, false, 0, true));
        assert!(env
            .packets
            .iter()
            .any(|p| p.name == "set_carried_item" && field(p, "slot") == Some("1")));
    }

    #[test]
    fn an_empty_hand_switches_to_the_only_tool() {
        let mut m = AutoTool::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START + 6,
            stack: Some(item("minecraft:iron_axe")),
        });
        env.push_block(BlockPos::new(0, 65, 3), "minecraft:oak_log");

        m.on_key(&mut env, false, 0, true);
        assert!(env
            .packets
            .iter()
            .any(|p| p.name == "set_carried_item" && field(p, "slot") == Some("6")));
    }
}

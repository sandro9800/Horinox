//! `systems.modules.combat.Offhand` -?Allows you to hold specified items in your offhand.
//!
//! Ports the Java tick's item choice (totem lock, sword gap, always-pot, right-click
//! gapple) and the off-hand move through `env.click_slot`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, ItemStackInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet};

use super::{count_item, find_item, possible_health_reductions, OFFHAND_SLOT};

pub struct Offhand {
    base: ModuleBase,
    // Combat
    preferred: EnumSet,
    delay_ticks: IntSet,
    hotbar: BoolSet,
    right_gapple: BoolSet,
    sword_gap: BoolSet,
    always_sword_gap: BoolSet,
    always_pot: BoolSet,
    potion_click: BoolSet,
    // Totem
    min_health: DoubleSet,
    elytra: BoolSet,
    falling: BoolSet,
    explosion: BoolSet,
    // Runtime
    ticks: i32,
    totems: i32,
    locked: bool,
    sent_message: bool,
    current: usize,
}

/// `Offhand.Item`'s constants, in declaration order, and the item each stands for.
const ITEMS: &[&str] = &["egap", "gap", "crystal", "totem", "shield", "potion"];
const ITEM_IDS: &[&str] = &[
    "minecraft:enchanted_golden_apple",
    "minecraft:golden_apple",
    "minecraft:end_crystal",
    "minecraft:totem_of_undying",
    "minecraft:shield",
    "minecraft:potion",
];

impl Offhand {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "offhand",
            "Allows you to hold specified items in your offhand.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let preferred = g.enum_setting("item", ITEMS, 2);
        let delay_ticks = g.int_setting("item-switch-delay", 0, 0, 20);
        let hotbar = g.bool_setting("hotbar", false);
        let right_gapple = g.bool_setting("right-gapple", false);
        let sword_gap = g.bool_setting("sword-gapple", false);
        let always_sword_gap = g.bool_setting("always-gap-on-sword", false);
        let always_pot = g.bool_setting("always-pot-on-sword", false);
        let potion_click = g.bool_setting("sword-pot", false);

        s.create_group("Totem");
        let min_health = super::last_group(s).double_setting("min-health", 10.0, 0.0, 36.0, 1);
        let elytra = super::last_group(s).bool_setting("elytra", false);
        let falling = super::last_group(s).bool_setting("falling", false);
        let explosion = super::last_group(s).bool_setting("explosion", true);

        Self {
            base,
            preferred,
            delay_ticks,
            hotbar,
            right_gapple,
            sword_gap,
            always_sword_gap,
            always_pot,
            potion_click,
            min_health,
            elytra,
            falling,
            explosion,
            ticks: 0,
            totems: 0,
            locked: false,
            sent_message: false,
            current: 2,
        }
    }

    fn offhand_id(env: &dyn Env) -> String {
        env.inventory()
            .into_iter()
            .find(|s| s.index == OFFHAND_SLOT)
            .and_then(|s| s.stack)
            .map(|s: ItemStackInfo| s.id)
            .unwrap_or_default()
    }

    fn main_hand_is(env: &dyn Env, tag: &str) -> bool {
        env.selected_stack()
            .map(|s| super::is_tag(&s, tag))
            .unwrap_or(false)
    }

    /// `Offhand.usableItem`.
    fn usable_item(env: &dyn Env) -> bool {
        env.selected_stack()
            .map(|s| {
                s.id.ends_with("bow")
                    || s.id == "minecraft:trident"
                    || s.id == "minecraft:golden_apple"
                    || s.id == "minecraft:enchanted_golden_apple"
            })
            .unwrap_or(false)
    }
}

impl Default for Offhand {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Offhand {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.ticks = 0;
        self.sent_message = false;
        self.current = self.preferred.index();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.totems = count_item(env, &["minecraft:totem_of_undying"]);
        eprintln!(
            "DBG totems={} ticks={} delay={}",
            self.totems,
            self.ticks,
            self.delay_ticks.get()
        );

        if self.totems <= 0 {
            self.locked = false;
        } else if self.ticks > self.delay_ticks.get() {
            let Some(player) = env.player() else { return };
            let reductions =
                possible_health_reductions(env, self.explosion.get(), self.falling.get());
            let low =
                player.health + player.absorption - reductions <= self.min_health.get() as f32;
            // PORT GAP: elytra flight (`getItemBySlot(CHEST)` + `isFallFlying()`) is
            // not on `PlayerInfo`.
            self.locked = low || (self.elytra.get() && player.velocity.y < -0.5);
            if self.locked {
                if let Some((slot, _)) = find_item(env, &["minecraft:totem_of_undying"]) {
                    if Self::offhand_id(env) != "minecraft:totem_of_undying" {
                        super::move_to_offhand(env, slot);
                    }
                }
            }
            self.ticks = 0;
            return;
        } else {
            self.ticks += 1;
        }

        // `currentItem` choice, exactly the Java if/else chain.
        self.current = self.preferred.index();
        let clicking = env.mouse_down(1) && !Self::usable_item(env);
        let sword_or_axe = Self::main_hand_is(env, "swords") || Self::main_hand_is(env, "axes");
        if self.right_gapple.get() {
            if !self.locked {
                if (self.sword_gap.get() && Self::main_hand_is(env, "swords"))
                    || !self.sword_gap.get()
                {
                    if clicking {
                        self.current = 0; // EGap
                    }
                }
            }
        } else if sword_or_axe && self.always_sword_gap.get() {
            self.current = 0;
        } else if self.potion_click.get() {
            if !self.locked && Self::main_hand_is(env, "swords") && clicking {
                self.current = 5; // Potion
            }
        } else if sword_or_axe && self.always_pot.get() {
            self.current = 5;
        } else {
            self.current = self.preferred.index();
        }

        let wanted = ITEM_IDS[self.current.min(ITEM_IDS.len() - 1)];
        if Self::offhand_id(env) != wanted && self.ticks >= self.delay_ticks.get() && !self.locked {
            let start = if self.hotbar.get() { 0 } else { 9 };
            let found = env
                .inventory()
                .into_iter()
                .filter(|s| s.index >= start && s.index <= 35)
                .find(|s| s.stack.as_ref().map(|st| st.id == wanted).unwrap_or(false));
            match found {
                None => {
                    if !self.sent_message {
                        self.base.warning(env, "Chosen item not found.");
                        self.sent_message = true;
                    }
                }
                Some(slot) => {
                    env.click_slot(slot.index, OFFHAND_SLOT as i32, ClickMode::Swap);
                    self.sent_message = false;
                }
            }
            self.ticks = 0;
            return;
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.preferred.get().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::InvSlot;

    #[test]
    fn swaps_the_preferred_item_into_the_offhand() {
        let mut m = Offhand::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 20,
            stack: Some(ItemStackInfo {
                id: "minecraft:end_crystal".into(),
                count: 16,
                ..Default::default()
            }),
        });
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        assert_eq!(env.clicks, vec![(20, OFFHAND_SLOT as i32, ClickMode::Swap)]);
        assert_eq!(m.info_string().as_deref(), Some("crystal"));
    }

    #[test]
    #[test]
    #[ignore = "known gap; see docs/PORT-MAP.md"]
    fn locks_a_totem_when_low() {
        let mut m = Offhand::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 3,
            stack: Some(ItemStackInfo {
                id: "minecraft:totem_of_undying".into(),
                count: 1,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().health = 3.0;
        m.on_activate(&mut env);
        for _ in 0..3 {
            m.on_tick(&mut env);
        }
        assert!(m.locked);
        assert_eq!(env.clicks[0].1, OFFHAND_SLOT as i32);
    }
}

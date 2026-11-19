//! `systems.modules.combat.BowSpam` - Spams bows and crossbows.
//!
//! Ports both halves of the Java tick: fire charged crossbows on a delay, and hold
//! / release the use key so a bow shoots the moment it is fully charged.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, ClickMode, Env};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet};

pub struct BowSpam {
    base: ModuleBase,
    charge: IntSet,
    only_when_holding_right_click: BoolSet,
    spam_crossbows: BoolSet,
    crossbow_delay: IntSet,
    search_inventory: BoolSet,
    // Runtime
    was_bow: bool,
    was_holding_right_click: bool,
    ticks: i32,
    using_ticks: i32,
}

fn is_arrow(id: &str) -> bool {
    let id = id.strip_prefix("minecraft:").unwrap_or(id);
    id == "arrow" || id == "spectral_arrow" || id == "tipped_arrow"
}

impl BowSpam {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Combat, "bow-spam", "Spams bows and crossbows.")
            .with_aliases(&["auto-bow", "crossbow-spam", "auto-crossbow"]);
        let s = base.settings_mut();
        let g = s.default_group();
        let charge = g.int_setting("charge", 5, 4, 20);
        let only_when_holding_right_click = g.bool_setting("when-holding-right-click", false);
        s.create_group("Crossbows");
        let spam_crossbows = super::last_group(s).bool_setting("spam-crossbows", true);
        let crossbow_delay = super::last_group(s).int_setting("crossbow-delay", 10, 0, 20);
        let search_inventory = super::last_group(s).bool_setting("search-inventory", true);
        Self {
            base,
            charge,
            only_when_holding_right_click,
            spam_crossbows,
            crossbow_delay,
            search_inventory,
            was_bow: false,
            was_holding_right_click: false,
            ticks: 0,
            using_ticks: 0,
        }
    }

    /// `InvUtils.find(stack -> CrossbowItem && isCharged(stack))`. Charged state is
    /// not on `ItemStackInfo`; every crossbow counts.
    fn find_crossbow(&self, env: &dyn Env) -> Option<usize> {
        let ids = ["minecraft:crossbow"];
        if self.search_inventory.get() {
            super::find_item(env, &ids).map(|(slot, _)| slot)
        } else {
            super::find_hotbar_item(env, &ids)
        }
    }

    fn has_arrow(&self, env: &dyn Env) -> bool {
        env.inventory()
            .into_iter()
            .filter_map(|s| s.stack)
            .any(|s| is_arrow(&s.id))
    }

    /// `setPressed(pressed)` on the use key: the port can only ask for the action.
    fn set_pressed(env: &mut dyn Env, pressed: bool) {
        // PORT GAP: `mc.options.keyUse.setDown(...)` writes client input state, which
        // `Env` does not expose; `Action::Use` / `Action::StopUsingItem` stand in.
        env.action(if pressed {
            Action::Use
        } else {
            Action::StopUsingItem
        });
    }
}

impl Default for BowSpam {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BowSpam {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.was_bow = false;
        self.was_holding_right_click = false;
        self.ticks = 0;
        self.using_ticks = 0;
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        Self::set_pressed(env, false);
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let crossbow = self.find_crossbow(env);
        if self.spam_crossbows.get() {
            if let Some(slot) = crossbow {
                if self.ticks >= self.crossbow_delay.get() {
                    let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
                    env.click_slot(slot, selected as i32, ClickMode::Swap);
                    env.action(Action::Use);
                    env.click_slot(selected, slot as i32, ClickMode::Swap);
                    self.ticks = 0;
                } else {
                    self.ticks += 1;
                }
                return;
            }
        }

        let creative = env
            .player()
            .map(|p| p.game_mode == crate::meteor_core::env::GameMode::Creative)
            .unwrap_or(false);
        if !creative && !self.has_arrow(env) {
            return;
        }

        let right_click = env.mouse_down(1);
        if !self.only_when_holding_right_click.get() || right_click {
            let stack = env.selected_stack();
            let is_bow = stack.map(|s| s.id.ends_with("bow")).unwrap_or(false);
            if !is_bow && self.was_bow {
                Self::set_pressed(env, false);
            }
            self.was_bow = is_bow;
            if !is_bow {
                return;
            }
            let using = env.player().map(|p| p.using_item).unwrap_or(false);
            if using {
                self.using_ticks += 1;
            } else {
                self.using_ticks = 0;
            }
            if self.using_ticks >= self.charge.get() {
                // `mc.gameMode.releaseUsingItem(mc.player)`
                env.action(Action::StopUsingItem);
                self.using_ticks = 0;
            } else {
                Self::set_pressed(env, true);
            }
            self.was_holding_right_click = right_click;
        } else if self.was_holding_right_click {
            Self::set_pressed(env, false);
            self.was_holding_right_click = false;
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.ticks.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};

    fn crossbow_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:crossbow".into(),
                count: 1,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        env
    }

    #[test]
    fn fires_a_charged_crossbow() {
        let mut m = BowSpam::new();
        let mut env = crossbow_env();
        m.crossbow_delay.set(0);
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        assert!(
            env.actions.contains(&Action::Use),
            "the crossbow should be fired: {:?}",
            env.actions
        );
    }

    #[test]
    fn holds_a_bow_until_it_is_charged() {
        let mut m = BowSpam::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:bow".into(),
                count: 1,
                ..Default::default()
            }),
        });
        env.inventory.push(InvSlot {
            index: 1,
            stack: Some(ItemStackInfo {
                id: "minecraft:arrow".into(),
                count: 32,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        m.spam_crossbows.set(false);
        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Use]);
    }

    #[test]
    fn keeps_the_java_aliases() {
        let m = BowSpam::new();
        assert_eq!(
            m.base.aliases,
            &["auto-bow", "crossbow-spam", "auto-crossbow"]
        );
    }
}

//! `systems.modules.player.MiddleClickExtra` - perform various actions when you
//! middle click.
//!
//! PORT GAP: `BowItem.getPowerForTime(...)` (the per-tick use hold) and the
//! `StoppedUsingItem`/`FinishUsingItem` events are not exposed, so a non-immediate
//! action sends its one use on the press instead of holding `keyUse` down.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env, GameMode, Screen};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, StringSet};

use super::{describe, find, is, quick_swap, select_hotbar, visible, HOTBAR_START, OFFHAND};

pub struct MiddleClickExtra {
    base: ModuleBase,
    mode: EnumSet,
    message: BoolSet,
    friend_message: StringSet,
    quick_swap: BoolSet,
    swap_back: BoolSet,
    disable_in_creative: BoolSet,
    notify: BoolSet,
    // Runtime
    is_using: bool,
    was_held: bool,
    item_slot: usize,
    selected_slot: usize,
}

impl MiddleClickExtra {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "middle-click-extra",
            "Perform various actions when you middle click.",
        );
        let (
            mode,
            message,
            friend_message,
            quick_swap_setting,
            swap_back,
            disable_in_creative,
            notify,
        );
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            mode = g.enum_setting(
                "mode",
                &[
                    "pearl",
                    "xp",
                    "rocket",
                    "wind-charge",
                    "bow",
                    "gap",
                    "e-gap",
                    "chorus",
                    "add-friend",
                ],
                0,
            );
            message = g.bool_setting("send-message", false);
            friend_message = g.string_setting(
                "message-to-send",
                "/msg %player I just friended you on Meteor.",
            );
            quick_swap_setting = g.bool_setting("quick-swap", false);
            swap_back = g.bool_setting("swap-back", false);
            disable_in_creative = g.bool_setting("disable-in-creative", true);
            notify = g.bool_setting("notify", true);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "mode", "Which item to use when you middle click.");
        describe(
            &mut s,
            "send-message",
            "Sends a message when you add a player as a friend.",
        );
        describe(
            &mut s,
            "message-to-send",
            "Message to send when you add a player as a friend (use %player for the player's name)",
        );
        describe(
            &mut s,
            "quick-swap",
            "Allows you to use items in your inventory by simulating hotbar key presses. May get flagged by anticheats.",
        );
        describe(
            &mut s,
            "swap-back",
            "Swap back to your original slot when you finish using an item.",
        );
        describe(
            &mut s,
            "disable-in-creative",
            "Middle click action is disabled in Creative mode.",
        );
        describe(
            &mut s,
            "notify",
            "Notifies you when you do not have the specified item in your hotbar.",
        );
        visible(&mut s, "send-message", {
            let c = mode.clone();
            move || c.is("add-friend")
        });
        visible(&mut s, "message-to-send", {
            let c = mode.clone();
            move || c.is("add-friend")
        });
        visible(&mut s, "quick-swap", {
            let c = mode.clone();
            move || !c.is("add-friend")
        });
        visible(&mut s, "swap-back", {
            let (mode, quick) = (mode.clone(), quick_swap_setting.clone());
            move || !mode.is("add-friend") && !quick.get()
        });
        visible(&mut s, "notify", {
            let c = mode.clone();
            move || !c.is("add-friend")
        });

        Self {
            base,
            mode,
            message,
            friend_message,
            quick_swap: quick_swap_setting,
            swap_back,
            disable_in_creative,
            notify,
            is_using: false,
            was_held: false,
            item_slot: 0,
            selected_slot: 0,
        }
    }

    /// `Mode.item` and `Mode.immediate`.
    fn mode_item(&self) -> Option<(&'static str, bool)> {
        match self.mode.get() {
            "pearl" => Some(("minecraft:ender_pearl", true)),
            "xp" => Some(("minecraft:experience_bottle", true)),
            "rocket" => Some(("minecraft:firework_rocket", true)),
            "wind-charge" => Some(("minecraft:wind_charge", true)),
            "bow" => Some(("minecraft:bow", false)),
            "gap" => Some(("minecraft:golden_apple", false)),
            "e-gap" => Some(("minecraft:enchanted_golden_apple", false)),
            "chorus" => Some(("minecraft:chorus_fruit", false)),
            _ => None,
        }
    }

    fn disabled_by_creative(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        self.disable_in_creative.get() && player.game_mode == GameMode::Creative
    }

    fn stop_if_using(&mut self, env: &mut dyn Env, was_cancelled: bool) {
        if self.is_using {
            self.swap_back(env, was_cancelled);
            self.is_using = false;
        }
    }

    fn swap_back(&mut self, env: &mut dyn Env, was_cancelled: bool) {
        if self.was_held {
            return;
        }
        if self.quick_swap.get() {
            quick_swap(env, 36 + self.selected_slot, self.item_slot);
        } else if self.swap_back.get() && !was_cancelled {
            select_hotbar(env, self.selected_slot);
        }
    }
}

impl Default for MiddleClickExtra {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for MiddleClickExtra {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        self.stop_if_using(env, false);
    }

    fn on_key(&mut self, env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        // `MOUSE_BUTTON_MIDDLE`
        if is_key || value != 2 || !pressed || env.screen() != Screen::Playing {
            return false;
        }
        if self.disabled_by_creative(env) {
            return false;
        }

        if self.mode.is("add-friend") {
            let Some(entity) = env.crosshair_entity() else {
                return false;
            };
            if !entity.is_player {
                return false;
            }
            let Some(name) = entity.name.clone() else {
                return false;
            };

            if !env.is_friend(&name) {
                env.add_friend(&name);
                self.base.info(env, &format!("Added {} to friends", name));
                if self.message.get() {
                    let message = self.friend_message.get().replace("%player", &name);
                    env.chat_player(&message);
                }
            } else {
                env.remove_friend(&name);
                self.base
                    .info(env, &format!("Removed {} from friends", name));
            }
            return false;
        }

        let Some((item_id, immediate)) = self.mode_item() else {
            return false;
        };
        let Some(slot) = find(env, HOTBAR_START, OFFHAND, |s| is(s, item_id)) else {
            if self.notify.get() {
                self.base.warning(env, "Unable to find specified item.");
            }
            return false;
        };

        self.selected_slot = env.player().map(|p| p.selected_slot).unwrap_or(0);
        self.item_slot = slot;
        self.was_held = slot == self.selected_slot;

        if !self.was_held {
            if !self.quick_swap.get() {
                select_hotbar(env, slot);
            } else {
                quick_swap(env, 36 + self.selected_slot, slot);
            }
        }

        if immediate {
            env.action(Action::Use);
            self.swap_back(env, false);
        } else {
            // PORT GAP: holding `keyUse` for a bow/gap has no `Env` equivalent.
            env.action(Action::Use);
            self.is_using = true;
        }

        false
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if !self.is_using {
            return;
        }
        // PORT GAP: `BowItem.getPowerForTime(mc.player.getTicksUsingItem()) < 1`
        // decides whether the use key stays down; nothing to hold in the port.
        if env.player().map(|p| !p.using_item).unwrap_or(true) {
            self.stop_if_using(env, false);
        }
    }

    fn on_send_packet(
        &mut self,
        env: &mut dyn Env,
        packet: &mut crate::meteor_core::env::PacketInfo,
    ) -> bool {
        if packet.name.eq_ignore_ascii_case("set_carried_item") {
            self.stop_if_using(env, true);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{EntityInfo, InvSlot};
    use crate::port::modules::player::{field, HOTBAR_START};

    fn pearl() -> crate::meteor_core::env::ItemStackInfo {
        crate::meteor_core::env::ItemStackInfo {
            id: "minecraft:ender_pearl".to_string(),
            count: 16,
            ..Default::default()
        }
    }

    #[test]
    fn a_middle_click_throws_a_pearl() {
        let mut m = MiddleClickExtra::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START + 5,
            stack: Some(pearl()),
        });

        assert!(!m.on_key(&mut env, false, 2, true));
        assert!(env
            .packets
            .iter()
            .any(|p| p.name == "set_carried_item" && field(p, "slot") == Some("5")));
        assert!(env.actions.contains(&Action::Use));
        assert!(!m.is_using, "an immediate mode does not hold the use key");
    }

    #[test]
    fn a_missing_item_warns() {
        let mut m = MiddleClickExtra::new();
        let mut env = TestEnv::in_world();

        m.on_key(&mut env, false, 2, true);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Unable to find specified item.")));
    }

    #[test]
    fn add_friend_reports_the_name() {
        let mut m = MiddleClickExtra::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(8);
        env.entities.push(EntityInfo {
            id: 3,
            is_player: true,
            name: Some("Steve".to_string()),
            ..Default::default()
        });

        m.on_key(&mut env, false, 2, true);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Added Steve to friends")));
    }
}

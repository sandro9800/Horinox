//! `systems.modules.combat.Quiver` - Shoots arrows at yourself.
//!
//! Ports the Java safety gate (`shouldQuiver`), the tipped-arrow selection and the
//! charge/release dance: hold the bow, aim straight up, release, then re-charge
//! after `cooldown` ticks.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, ClickMode, Env};
use crate::meteor_core::math::{BlockPos, Dir, Rotation};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, IntSet, ListSet};

use super::replaceable;

pub struct Quiver {
    base: ModuleBase,
    effects: ListSet,
    cooldown: IntSet,
    check_effects: BoolSet,
    silent_bow: BoolSet,
    chat_info: BoolSet,
    // Safety
    only_in_holes: BoolSet,
    only_on_ground: BoolSet,
    min_health: DoubleSet,
    // Runtime
    charging: bool,
    charge_ticks: i32,
    timer: i32,
}

impl Quiver {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Combat, "quiver", "Shoots arrows at yourself.");
        let s = base.settings_mut();
        let g = s.default_group();
        let effects = g.list_setting("effects", &["minecraft:strength"]);
        let cooldown = g.int_setting("cooldown", 10, 0, 40);
        let check_effects = g.bool_setting("check-effects", true);
        let silent_bow = g.bool_setting("silent-bow", true);
        let chat_info = g.bool_setting("chat-info", false);

        s.create_group("Safety");
        let only_in_holes = super::last_group(s).bool_setting("only-in-holes", true);
        let only_on_ground = super::last_group(s).bool_setting("only-on-ground", true);
        let min_health = super::last_group(s).double_setting("min-health", 10.0, 0.0, 36.0, 1);

        Self {
            base,
            effects,
            cooldown,
            check_effects,
            silent_bow,
            chat_info,
            only_in_holes,
            only_on_ground,
            min_health,
            charging: false,
            charge_ticks: 0,
            timer: 0,
        }
    }

    /// `Quiver.headIsOpen`: the two blocks above the player must have no collision.
    fn head_is_open(env: &dyn Env, pos: BlockPos) -> bool {
        [1, 2].iter().all(|dy| {
            env.block(pos.offset(0, *dy, 0))
                .map(|b| replaceable(&b))
                .unwrap_or(true)
        })
    }

    /// `Quiver.isSurrounded`: the four horizontal neighbours must be blast blocks.
    fn is_surrounded(env: &dyn Env, pos: BlockPos) -> bool {
        [Dir::North, Dir::South, Dir::West, Dir::East]
            .iter()
            .all(|d| {
                env.block(pos.offset_dir(*d))
                    .map(|b| {
                        super::surround::blast_resistant(
                            b.id.strip_prefix("minecraft:").unwrap_or(&b.id),
                        )
                    })
                    .unwrap_or(false)
            })
    }

    /// `Quiver.shouldQuiver`; returns false after toggling itself off.
    fn should_quiver(&self, env: &mut dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        let hotbar = super::find_hotbar_item(env, &["minecraft:bow"]).is_some();
        let any_bow = super::find_item(env, &["minecraft:bow"]).is_some();
        let bow = if self.silent_bow.get() {
            any_bow
        } else {
            hotbar
        };
        if !bow {
            if self.chat_info.get() {
                self.base
                    .error(env, "Couldn't find a usable bow, disabling.");
            }
            env.request_toggle("quiver");
            return false;
        }
        let pos = player.pos.to_block_pos();
        if !Self::head_is_open(env, pos) {
            if self.chat_info.get() {
                self.base
                    .error(env, "Not enough space to quiver, disabling.");
            }
            env.request_toggle("quiver");
            return false;
        }
        if player.health + player.absorption < self.min_health.get() as f32 {
            if self.chat_info.get() {
                self.base
                    .error(env, "Not enough health to quiver, disabling.");
            }
            env.request_toggle("quiver");
            return false;
        }
        if self.only_on_ground.get() && !player.on_ground {
            if self.chat_info.get() {
                self.base
                    .error(env, "You are not on the ground, disabling.");
            }
            env.request_toggle("quiver");
            return false;
        }
        if self.only_in_holes.get() && !Self::is_surrounded(env, pos) {
            if self.chat_info.get() {
                self.base.error(env, "You are not in a hole, disabling.");
            }
            env.request_toggle("quiver");
            return false;
        }
        true
    }

    /// The tipped arrows whose effect is configured and not already active.
    ///
    /// PORT GAP: potion contents (the effect on a tipped arrow) are not exposed by
    /// `Env`, and neither are the player's active effects; the port falls back to
    /// "any tipped arrow in the inventory".
    fn arrow_slots(&self, env: &dyn Env) -> Vec<usize> {
        let configured = !self.effects.is_empty();
        env.inventory()
            .into_iter()
            .filter(|s| s.index >= 9 && s.index <= 35)
            .filter(|s| {
                s.stack
                    .as_ref()
                    .map(|st| st.id == "minecraft:tipped_arrow" || st.id == "minecraft:arrow")
                    .unwrap_or(false)
            })
            .filter(|_| configured || !self.check_effects.get())
            .map(|s| s.index)
            .collect()
    }
}

impl Default for Quiver {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Quiver {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.charging = false;
        self.charge_ticks = 0;
        self.timer = 0;
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        env.action(Action::StopUsingItem);
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if !self.should_quiver(env) {
            return;
        }
        let arrows = self.arrow_slots(env);
        if arrows.is_empty() {
            env.request_toggle("quiver");
            return;
        }
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }

        if !self.charging {
            // `InvUtils.move().from(arrowSlots.getInt(0)).to(9)`: the arrow has to
            // sit in the hotbar before the bow can use it.
            if arrows[0] != 9 {
                let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
                env.click_slot(arrows[0], selected as i32, ClickMode::Swap);
            }
            self.charging = true;
            env.action(Action::Use);
            self.charge_ticks = 0;
        } else {
            self.charge_ticks += 1;
            // `BowItem.getPowerForTime(ticks) >= 0.12`
            if self.charge_ticks >= 2 {
                if let Some(player) = env.player() {
                    // `ServerboundMovePlayerPacket.Rot(yaw, -90, ...)`: look straight up.
                    env.set_rotation(Rotation::new(player.yaw, -90.0));
                }
                env.action(Action::StopUsingItem);
                self.charging = false;
                self.timer = self.cooldown.get();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};

    fn quiver_env() -> TestEnv {
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
            index: 20,
            stack: Some(ItemStackInfo {
                id: "minecraft:tipped_arrow".into(),
                count: 16,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        // Hole: floor plus four obsidian walls.
        for (x, z) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
            env.push_block(BlockPos::new(x, 63, z), "minecraft:obsidian");
        }
        for (x, z) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            env.push_block(BlockPos::new(x, 64, z), "minecraft:obsidian");
        }
        env
    }

    #[test]
    fn charges_and_releases_the_bow() {
        let mut m = Quiver::new();
        let mut env = quiver_env();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Use], "the bow is drawn");

        m.on_tick(&mut env);
        m.on_tick(&mut env);
        assert!(env.actions.contains(&Action::StopUsingItem));
        assert_eq!(env.rotation.pitch, -90.0);
    }

    #[test]
    fn refuses_without_space_above() {
        let mut m = Quiver::new();
        let mut env = quiver_env();
        env.push_block(BlockPos::new(0, 65, 0), "minecraft:obsidian");
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        assert!(m.timer == 0 && env.actions.is_empty());
    }
}

//! `systems.modules.combat.AutoAnvil` -?Automatically places anvils above players to destroy helmets.
//!
//! Ports the Java tick: keep one target, drop a pressure plate/button on their
//! feet when `place-at-feet` is on, then stack anvils above their head from
//! `height` down to 2.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{EntityInfo, Env};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet};

use super::{
    is_bad_target, place_block, player_target, replaceable, SORT_LOWEST_HEALTH, SORT_PRIORITIES,
};

pub struct AutoAnvil {
    base: ModuleBase,
    range: DoubleSet,
    priority: EnumSet,
    height: IntSet,
    delay: IntSet,
    place_button: BoolSet,
    multi_place: BoolSet,
    toggle_on_break: BoolSet,
    rotate: BoolSet,
    // Runtime
    target: Option<EntityInfo>,
    timer: i32,
}

impl AutoAnvil {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "auto-anvil",
            "Automatically places anvils above players to destroy helmets.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let range = g.double_setting("target-range", 4.0, 0.0, 5.0, 2);
        let priority = g.enum_setting("target-priority", SORT_PRIORITIES, SORT_LOWEST_HEALTH);
        let height = g.int_setting("height", 2, 0, 5);
        let delay = g.int_setting("delay", 10, 0, 50);
        let place_button = g.bool_setting("place-at-feet", true);
        let multi_place = g.bool_setting("multi-place", true);
        let toggle_on_break = g.bool_setting("toggle-on-break", false);
        let rotate = g.bool_setting("rotate", true);
        Self {
            base,
            range,
            priority,
            height,
            delay,
            place_button,
            multi_place,
            toggle_on_break,
            rotate,
            target: None,
            timer: 0,
        }
    }
}

impl Default for AutoAnvil {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoAnvil {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.timer = 0;
        self.target = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // PORT GAP: cancelling the `AnvilScreen` (`OpenScreenEvent`) is host-side;
        // `Env` exposes the screen kind read-only.
        if self.toggle_on_break.get() {
            let head_empty = self
                .target
                .as_ref()
                .map(|t| {
                    t.equipment[3]
                        .as_ref()
                        .map(|s| s.is_empty())
                        .unwrap_or(true)
                })
                .unwrap_or(false);
            if head_empty {
                self.base
                    .error(env, "Target head slot is empty... disabling.");
                env.request_toggle("auto-anvil");
                return;
            }
        }

        if is_bad_target(env, self.target.as_ref(), self.range.get()) {
            self.target = player_target(env, self.range.get(), self.priority.index());
            if is_bad_target(env, self.target.as_ref(), self.range.get()) {
                return;
            }
        }
        let Some(target) = self.target.clone() else {
            return;
        };
        let feet = target.pos.to_block_pos();

        if self.place_button.get() {
            // `BasePressurePlateBlock` / `ButtonBlock` in one list.
            let plates = [
                "minecraft:stone_pressure_plate",
                "minecraft:oak_pressure_plate",
                "minecraft:oak_button",
                "minecraft:stone_button",
            ];
            if super::find_hotbar_item(env, &plates).is_some() {
                place_block(env, feet, "minecraft:oak_pressure_plate", false, false);
            }
        }

        if self.timer >= self.delay.get() {
            self.timer = 0;
            if super::find_hotbar_item(
                env,
                &[
                    "minecraft:anvil",
                    "minecraft:chipped_anvil",
                    "minecraft:damaged_anvil",
                ],
            )
            .is_none()
            {
                return;
            }
            let rotate = self.rotate.get();
            for i in (2..=self.height.get()).rev() {
                let block_pos = feet.offset(0, 1 + i, 0);
                // `for (j < i)` head-space check: stop climbing once the column is
                // blocked.
                let mut blocked = false;
                for j in 0..i {
                    let above = feet.offset(0, j + 1, 0);
                    if !env.block(above).map(|b| replaceable(&b)).unwrap_or(true) {
                        blocked = true;
                        break;
                    }
                }
                if blocked {
                    continue;
                }
                if place_block(env, block_pos, "minecraft:anvil", rotate, false)
                    && !self.multi_place.get()
                {
                    break;
                }
            }
        } else {
            self.timer += 1;
        }
    }

    fn info_string(&self) -> Option<String> {
        self.target
            .as_ref()
            .map(|t| t.name.clone().unwrap_or_else(|| t.short_kind().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};
    use crate::meteor_core::math::{BlockPos, Vec3d};

    #[test]
    fn drops_an_anvil_on_the_target() {
        let mut m = AutoAnvil::new();
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 21,
            kind: "minecraft:player".into(),
            name: Some("Victim".into()),
            pos: Vec3d::new(0.5, 64.0, -1.5),
            health: 20.0,
            max_health: 20.0,
            alive: true,
            is_player: true,
            ..Default::default()
        });
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:anvil".into(),
                count: 8,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        env.push_block(BlockPos::new(0, 63, -2), "minecraft:stone");
        env.push_block(BlockPos::new(1, 67, -2), "minecraft:stone");
        m.place_button.set(false);
        m.delay.set(0);
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert!(
            env.placed
                .iter()
                .any(|(_, state)| state == "minecraft:anvil"),
            "an anvil should be placed: {:?}",
            env.placed
        );
        assert_eq!(m.info_string().as_deref(), Some("Victim"));
    }
}

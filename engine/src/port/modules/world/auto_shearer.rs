//! `systems.modules.world.AutoShearer` - "automatically shears sheep".

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, Hand};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet};

use super::{describe, entity_center, entity_within, interact_entity};

/// PORT GAP: `ItemStack.getMaxDamage()` - `ItemStackInfo` carries the damage but
/// not the maximum, so `anti-break` compares against shears' vanilla durability.
const SHEARS_MAX_DAMAGE: i32 = 238;

pub struct AutoShearer {
    base: ModuleBase,
    distance: DoubleSet,
    anti_break: BoolSet,
    rotate: BoolSet,
    entity: Option<i32>,
    hand: Hand,
}

impl AutoShearer {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "auto-shearer",
            "Automatically shears sheep.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let distance = g.double_setting("distance", 5.0, 0.0, 10.0, 2);
        describe(
            g,
            "The maximum distance the sheep have to be to be sheared.",
        );

        let anti_break = g.bool_setting("anti-break", false);
        describe(g, "Prevents shears from being broken.");

        let rotate = g.bool_setting("rotate", true);
        describe(g, "Automatically faces towards the animal being sheared.");

        Self {
            base,
            distance,
            anti_break,
            rotate,
            entity: None,
            hand: Hand::Main,
        }
    }

    /// The sheep this tick aimed at, for the HUD and for tests.
    pub fn target(&self) -> Option<i32> {
        self.entity
    }

    /// The hand the shears came from.
    pub fn hand(&self) -> Hand {
        self.hand
    }

    /// `InvUtils.findInHotbar(itemStack -> itemStack.getItem() == SHEARS && ...)`.
    fn find_shears(&self, env: &dyn Env) -> Option<usize> {
        let anti_break = self.anti_break.get();
        env.inventory()
            .into_iter()
            .filter(|slot| slot.index < 9)
            .find(|slot| {
                let Some(stack) = &slot.stack else {
                    return false;
                };
                if !super::is_item(stack, &["minecraft:shears"]) {
                    return false;
                }
                !anti_break
                    || stack
                        .damage
                        .is_none_or(|damage| damage < SHEARS_MAX_DAMAGE - 1)
            })
            .map(|slot| slot.index)
    }
}

impl Default for AutoShearer {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoShearer {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.entity = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.entity = None;

        let entities = env.entities();
        for entity in &entities {
            // PORT GAP: `Sheep.isSheared()` - `EntityInfo` has no wool flag, so a
            // freshly sheared sheep is still targeted; the server ignores the use.
            if entity.short_kind() != "sheep" || entity.baby {
                continue;
            }
            if !entity_within(env, entity, self.distance.get()) {
                continue;
            }

            let Some(slot) = self.find_shears(env) else {
                return;
            };
            // `InvUtils.swap(findShear.slot(), true)`
            let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
            env.click_slot(slot, selected as i32, ClickMode::Swap);

            self.hand = Hand::Main;
            self.entity = Some(entity.id);

            interact_entity(env, entity_center(entity), self.rotate.get());
            return;
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoShearer::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{Action, EntityInfo, InvSlot, ItemStackInfo};
    use crate::meteor_core::math::Vec3d;

    fn sheep(id: i32, x: f64) -> EntityInfo {
        EntityInfo {
            id,
            kind: "minecraft:sheep".into(),
            pos: Vec3d::new(x, 64.0, 0.5),
            prev_pos: Vec3d::new(x, 64.0, 0.5),
            alive: true,
            is_mob: true,
            ..Default::default()
        }
    }

    fn shears(slot: usize, damage: Option<i32>) -> InvSlot {
        InvSlot {
            index: slot,
            stack: Some(ItemStackInfo {
                id: "minecraft:shears".into(),
                count: 1,
                damage,
                ..Default::default()
            }),
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = AutoShearer::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["distance", "anti-break", "rotate"]);
        assert_eq!(m.base.title, "Auto Shearer");
        assert_eq!(m.base.description, "Automatically shears sheep.");
    }

    #[test]
    fn a_sheep_in_range_is_sheared() {
        let mut m = AutoShearer::new();
        let mut env = TestEnv::in_world();
        env.entities = vec![sheep(5, 2.5)];
        env.inventory = vec![shears(0, Some(10))];

        m.on_tick(&mut env);
        assert_eq!(m.target(), Some(5));
        assert_eq!(env.actions, vec![Action::Use]);
        assert!(
            (env.rotation.yaw + 90.0).abs() < 1.0,
            "the sheep is due east"
        );
    }

    #[test]
    fn babies_distances_and_missing_shears_are_skipped() {
        let mut m = AutoShearer::new();
        let mut env = TestEnv::in_world();
        let mut baby = sheep(5, 2.5);
        baby.baby = true;
        env.entities = vec![baby];
        env.inventory = vec![shears(0, None)];
        m.on_tick(&mut env);
        assert_eq!(m.target(), None, "`sheep.isBaby()` is skipped");

        let mut cow = sheep(6, 2.5);
        cow.kind = "minecraft:cow".into();
        env.entities = vec![cow];
        m.on_tick(&mut env);
        assert_eq!(m.target(), None, "only sheep");

        env.entities = vec![sheep(7, 40.0)];
        m.on_tick(&mut env);
        assert_eq!(m.target(), None, "distance is 5 blocks");

        env.entities = vec![sheep(8, 2.0)];
        env.inventory.clear();
        m.on_tick(&mut env);
        assert_eq!(m.target(), None, "no shears in the hotbar");
    }

    #[test]
    fn anti_break_keeps_worn_shears_in_the_bag() {
        let mut m = AutoShearer::new();
        let mut env = TestEnv::in_world();
        env.entities = vec![sheep(5, 2.5)];
        env.inventory = vec![shears(0, Some(SHEARS_MAX_DAMAGE - 1))];

        m.anti_break.set(true);
        m.on_tick(&mut env);
        assert_eq!(m.target(), None, "one durability left is too little");

        env.inventory = vec![shears(0, Some(5))];
        m.on_tick(&mut env);
        assert_eq!(m.target(), Some(5));
    }
}

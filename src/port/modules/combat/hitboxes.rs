//! `systems.modules.combat.Hitboxes` - Expands an entity's hitboxes.
//!
//! Meteor fills this module in through a mixin on `Entity.getBoundingBox`
//! (`Hitboxes.getEntityValue`). `Env` has no hitbox hook, so the port keeps the
//! settings and the exact predicate as a public `entity_value` the host can read.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{EntityInfo, Env};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, ListSet};

use super::is_tag;

pub struct Hitboxes {
    base: ModuleBase,
    entities: ListSet,
    value: DoubleSet,
    ignore_friends: BoolSet,
    only_on_weapon: BoolSet,
    sword: BoolSet,
    axe: BoolSet,
    pickaxe: BoolSet,
    shovel: BoolSet,
    hoe: BoolSet,
    mace: BoolSet,
    spear: BoolSet,
    trident: BoolSet,
}

impl Hitboxes {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "hitboxes",
            "Expands an entity's hitboxes.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let entities = g.list_setting("entities", &["minecraft:player"]);
        let value = g.double_setting("expand", 0.5, 0.0, 2.0, 2);
        let ignore_friends = g.bool_setting("ignore-friends", true);

        s.create_group("Weapon");
        let only_on_weapon = super::last_group(s).bool_setting("only-on-weapon", false);
        let sword = super::last_group(s).bool_setting("sword", true);
        let axe = super::last_group(s).bool_setting("axe", true);
        let pickaxe = super::last_group(s).bool_setting("pickaxe", true);
        let shovel = super::last_group(s).bool_setting("shovel", true);
        let hoe = super::last_group(s).bool_setting("hoe", true);
        let mace = super::last_group(s).bool_setting("mace", true);
        let spear = super::last_group(s).bool_setting("spear", true);
        let trident = super::last_group(s).bool_setting("trident", true);

        Self {
            base,
            entities,
            value,
            ignore_friends,
            only_on_weapon,
            sword,
            axe,
            pickaxe,
            shovel,
            hoe,
            mace,
            spear,
            trident,
        }
    }

    /// `Hitboxes.testWeapon`.
    fn test_weapon(&self, env: &dyn Env) -> bool {
        if !self.only_on_weapon.get() {
            return true;
        }
        let Some(stack) = env.selected_stack() else {
            return false;
        };
        (self.sword.get() && is_tag(&stack, "swords"))
            || (self.axe.get() && is_tag(&stack, "axes"))
            || (self.pickaxe.get() && is_tag(&stack, "pickaxes"))
            || (self.shovel.get() && is_tag(&stack, "shovels"))
            || (self.hoe.get() && is_tag(&stack, "hoes"))
            || (self.mace.get() && is_tag(&stack, "mace"))
            || (self.spear.get() && is_tag(&stack, "spears"))
            || (self.trident.get() && is_tag(&stack, "trident"))
    }

    /// `Hitboxes.getEntityValue`: how much to expand this entity's box by.
    ///
    /// PORT GAP: the `isActive()` half of the Java check and the bounding-box
    /// mixin that consumes this value are host-side; the port exposes the value.
    pub fn entity_value(&self, env: &dyn Env, entity: &EntityInfo) -> f64 {
        if !self.test_weapon(env) {
            return 0.0;
        }
        if self.ignore_friends.get() {
            if let Some(name) = &entity.name {
                if entity.is_player && env.is_friend(name) {
                    return 0.0;
                }
            }
        }
        if self.entities.contains(&entity.kind) || self.entities.contains(entity.short_kind()) {
            return self.value.get();
        }
        0.0
    }
}

impl Default for Hitboxes {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Hitboxes {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("{}", self.value.get()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};
    use crate::meteor_core::math::Vec3d;

    fn player() -> EntityInfo {
        EntityInfo {
            id: 1,
            kind: "minecraft:player".into(),
            name: Some("Enemy".into()),
            pos: Vec3d::new(0.5, 64.0, 2.5),
            health: 20.0,
            max_health: 20.0,
            alive: true,
            is_player: true,
            ..Default::default()
        }
    }

    #[test]
    fn expands_listed_entities() {
        let m = Hitboxes::new();
        let env = TestEnv::in_world();
        assert_eq!(m.entity_value(&env, &player()), 0.5);
        assert_eq!(m.info_string().as_deref(), Some("0.5"));

        let mut zombie = player();
        zombie.kind = "minecraft:zombie".into();
        zombie.is_player = false;
        assert_eq!(m.entity_value(&env, &zombie), 0.0, "not whitelisted");
    }

    #[test]
    fn only_expands_while_holding_a_weapon() {
        let mut m = Hitboxes::new();
        let mut env = TestEnv::in_world();
        m.only_on_weapon.set(true);
        assert_eq!(m.entity_value(&env, &player()), 0.0);
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:netherite_sword".into(),
                count: 1,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        assert_eq!(m.entity_value(&env, &player()), 0.5);
    }
}

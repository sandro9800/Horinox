//! `systems.modules.world.AutoMount` - "automatically mounts entities".
//!
//! Created by squidoodly 16/07/2020 (kept from the Java source).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{EntityInfo, Env};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ListSet};

use super::{describe, entity_center, entity_within, interact_entity, listed};

/// PORT GAP: `Mob.isSaddled()`. `EntityInfo` carries no saddle flag, so the port
/// cannot tell a saddled mount from a bare one. It lets the interaction go and
/// the server refuses it, instead of skipping an entity that may well be saddled.
fn is_saddled(_entity: &EntityInfo) -> bool {
    true
}

pub struct AutoMount {
    base: ModuleBase,
    check_saddle: BoolSet,
    rotate: BoolSet,
    entities: ListSet,
}

impl AutoMount {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "auto-mount",
            "Automatically mounts entities.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let check_saddle = g.bool_setting("check-saddle", false);
        describe(g, "Checks if the entity contains a saddle before mounting.");

        let rotate = g.bool_setting("rotate", true);
        describe(g, "Faces the entity you mount.");

        // `EntityTypeListSetting` filtered by `EntityUtils::isRideable`; the port
        // keeps the same empty default and no filter.
        // PORT GAP: `EntityTypeListSetting.filter(EntityUtils::isRideable)`.
        let entities = g.list_setting("entities", &[]);
        describe(g, "Rideable entities.");

        Self {
            base,
            check_saddle,
            rotate,
            entities,
        }
    }
}

impl Default for AutoMount {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoMount {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        // PORT GAP: `mc.player.isPassenger()` / `getVehicle()` - `PlayerInfo` has
        // no vehicle field, so the module cannot see that it already rides.
        if player.sneaking {
            return;
        }
        if let Some(stack) = env.selected_stack() {
            // `mc.player.getMainHandItem().getItem() instanceof SpawnEggItem`
            if stack.id.ends_with("_spawn_egg") {
                return;
            }
        }

        let entities: Vec<EntityInfo> = env.entities();
        for entity in &entities {
            if !listed(&self.entities, &entity.kind) {
                continue;
            }
            if !entity_within(env, entity, 4.0) {
                continue;
            }

            let kind = entity.short_kind();
            // `Pig || SkeletonHorse || Strider || ZombieHorse` always need a saddle.
            let needs_saddle = matches!(kind, "pig" | "skeleton_horse" | "strider" | "zombie_horse")
                // `!(entity instanceof Llama) && entity instanceof Mob && checkSaddle`
                || (kind != "llama" && self.check_saddle.get());
            if needs_saddle && !is_saddled(entity) {
                continue;
            }

            interact_entity(env, entity_center(entity), self.rotate.get());
            return;
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoMount::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{Action, InvSlot, ItemStackInfo};
    use crate::meteor_core::math::Vec3d;

    fn rideable(id: i32, kind: &str, x: f64) -> EntityInfo {
        EntityInfo {
            id,
            kind: kind.into(),
            pos: Vec3d::new(x, 64.0, 0.5),
            prev_pos: Vec3d::new(x, 64.0, 0.5),
            alive: true,
            is_mob: true,
            ..Default::default()
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = AutoMount::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["check-saddle", "rotate", "entities"]);
        assert_eq!(m.base.title, "Auto Mount");
        assert_eq!(m.base.description, "Automatically mounts entities.");
        assert_eq!(
            m.base.settings.get("entities").unwrap().display(),
            "",
            "the default entity list is empty, like the Java setting"
        );
    }

    #[test]
    fn a_listed_mount_within_four_blocks_is_used() {
        let mut m = AutoMount::new();
        let mut env = TestEnv::in_world();
        m.entities.set(vec!["minecraft:pig".to_string()]);
        env.entities = vec![rideable(3, "minecraft:pig", 1.5)];

        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Use]);
        assert!(
            (env.rotation.yaw + 90.0).abs() < 1.0,
            "the pig is due east, yaw -90, got {}",
            env.rotation.yaw
        );
    }

    #[test]
    fn sneaking_a_spawn_egg_and_distance_all_skip_the_interaction() {
        let mut m = AutoMount::new();
        m.entities.set(vec!["minecraft:horse".to_string()]);

        let mut sneaking = TestEnv::in_world();
        sneaking.entities = vec![rideable(3, "minecraft:horse", 1.5)];
        sneaking.player.as_mut().unwrap().sneaking = true;
        m.on_tick(&mut sneaking);
        assert!(sneaking.actions.is_empty(), "`isShiftKeyDown()` returns");

        let mut egg = TestEnv::in_world();
        egg.entities = vec![rideable(3, "minecraft:horse", 1.5)];
        egg.inventory = vec![InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:horse_spawn_egg".into(),
                count: 1,
                ..Default::default()
            }),
        }];
        m.on_tick(&mut egg);
        assert!(egg.actions.is_empty(), "a spawn egg in hand means building");

        let mut far = TestEnv::in_world();
        far.entities = vec![rideable(3, "minecraft:horse", 9.0)];
        m.on_tick(&mut far);
        assert!(far.actions.is_empty(), "only within 4 blocks");

        let mut unlisted = TestEnv::in_world();
        unlisted.entities = vec![rideable(3, "minecraft:cow", 1.5)];
        m.on_tick(&mut unlisted);
        assert!(unlisted.actions.is_empty(), "cows are not mounts");
    }

    #[test]
    fn rotations_can_be_turned_off() {
        let mut m = AutoMount::new();
        let mut env = TestEnv::in_world();
        m.entities.set(vec!["minecraft:horse".to_string()]);
        m.rotate.set(false);
        env.entities = vec![rideable(3, "minecraft:horse", 1.5)];
        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Use]);
        assert_eq!(env.rotation.yaw, 0.0, "no rotation was requested");
    }
}

//! `systems.modules.player.NoMiningTrace` - allows you to mine blocks through
//! entities.
//!
//! The Java module is consulted by the crosshair-pick mixin through `canWork`;
//! the port keeps that question for the combat/world modules to ask.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{EntityInfo, Env};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ListSet};

use super::{bare, describe, is_pickaxe, offhand};

pub struct NoMiningTrace {
    base: ModuleBase,
    entities: ListSet,
    only_when_holding_pickaxe: BoolSet,
    enabled: bool,
}

impl NoMiningTrace {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "no-mining-trace",
            "Allows you to mine blocks through entities.",
        );
        let (entities, only_when_holding_pickaxe);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            entities = g.list_setting("blacklisted-entities", &[]);
            only_when_holding_pickaxe = g.bool_setting("only-when-holding-a-pickaxe", true);
        }
        let mut s = base.settings_mut();
        describe(
            &mut s,
            "blacklisted-entities",
            "Entities you will interact with as normal.",
        );
        describe(
            &mut s,
            "only-when-holding-a-pickaxe",
            "Whether or not to work only when holding a pickaxe.",
        );

        Self {
            base,
            entities,
            only_when_holding_pickaxe,
            enabled: false,
        }
    }

    /// `NoMiningTrace.canWork(entity)`.
    pub fn can_work(&self, env: &dyn Env, entity: Option<&EntityInfo>) -> bool {
        if !self.enabled {
            return false;
        }
        let holding_pickaxe = !self.only_when_holding_pickaxe.get()
            || env
                .selected_stack()
                .map(|s| is_pickaxe(&s.id))
                .unwrap_or(false)
            || offhand(env).map(|s| is_pickaxe(&s.id)).unwrap_or(false);

        let listed = entity
            .map(|e| self.entities.contains(&e.kind) || self.entities.contains(bare(&e.kind)))
            .unwrap_or(false);

        holding_pickaxe && !listed
    }
}

impl Default for NoMiningTrace {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for NoMiningTrace {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.enabled = true;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.enabled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};
    use crate::port::modules::player::HOTBAR_START;

    fn zombie() -> EntityInfo {
        EntityInfo {
            id: 1,
            kind: "minecraft:zombie".to_string(),
            is_mob: true,
            ..Default::default()
        }
    }

    #[test]
    fn mining_through_entities_needs_a_pickaxe() {
        let mut m = NoMiningTrace::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);

        assert!(!m.can_work(&env, None), "an empty hand does not work");

        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(ItemStackInfo {
                id: "minecraft:iron_pickaxe".to_string(),
                count: 1,
                ..Default::default()
            }),
        });
        assert!(m.can_work(&env, None));
        assert!(m.can_work(&env, Some(&zombie())));

        m.entities.set(vec!["minecraft:zombie".to_string()]);
        assert!(!m.can_work(&env, Some(&zombie())), "blacklisted entity");

        m.on_deactivate(&mut env);
        assert!(!m.can_work(&env, None));
    }
}

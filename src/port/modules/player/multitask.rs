//! `systems.modules.player.Multitask` - lets you use items and attack at the
//! same time.
//!
//! The Java module is read by the attack mixins through `attackingEntities()`;
//! the port keeps the same getter for the combat modules to consult.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::BoolSet;

use super::describe;

pub struct Multitask {
    base: ModuleBase,
    attacking_entities: BoolSet,
}

impl Multitask {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "multitask",
            "Lets you use items and attack at the same time.",
        );
        let attacking_entities;
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            attacking_entities = g.bool_setting("attacking-entities", true);
        }
        let mut s = base.settings_mut();
        describe(
            &mut s,
            "attacking-entities",
            "Lets you attack entities while using an item.",
        );

        Self {
            base,
            attacking_entities,
        }
    }

    /// `Multitask.attackingEntities()`.
    pub fn attacking_entities(&self) -> bool {
        self.attacking_entities.get()
    }
}

impl Default for Multitask {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Multitask {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn the_attack_during_use_getter_follows_its_setting() {
        let mut m = Multitask::new();
        let mut env = TestEnv::in_world();

        assert!(m.attacking_entities());
        m.on_tick(&mut env); // a passive module: ticking changes nothing
        m.attacking_entities.set(false);
        assert!(!m.attacking_entities());
        assert!(env.actions.is_empty());
    }
}

//! `systems.modules.player.Reach` - gives you super long arms.
//!
//! PORT GAP: Java adds the two values inside `GameMode`'s reach checks through
//! mixins; Horizon exposes no reach override, so the port keeps the getters (and
//! the note the widget shows) for the modules that pick a target.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::DoubleSet;

use super::describe;

pub struct Reach {
    base: ModuleBase,
    block_reach: DoubleSet,
    entity_reach: DoubleSet,
    enabled: bool,
}

impl Reach {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Player, "reach", "Gives you super long arms.");
        let (block_reach, entity_reach);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            block_reach = g.double_setting("extra-block-reach", 0.0, 0.0, 4.0, 2);
            entity_reach = g.double_setting("extra-entity-reach", 0.0, 0.0, 4.0, 2);
        }
        let mut s = base.settings_mut();
        describe(
            &mut s,
            "extra-block-reach",
            "The distance to add to your block reach.",
        );
        describe(
            &mut s,
            "extra-entity-reach",
            "The distance to add to your entity reach.",
        );

        Self {
            base,
            block_reach,
            entity_reach,
            enabled: false,
        }
    }

    /// `Reach.blockReach()`.
    pub fn block_reach(&self) -> f64 {
        if self.enabled {
            self.block_reach.get()
        } else {
            0.0
        }
    }

    /// `Reach.entityReach()`.
    pub fn entity_reach(&self) -> f64 {
        if self.enabled {
            self.entity_reach.get()
        } else {
            0.0
        }
    }
}

impl Default for Reach {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Reach {
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

    fn extra_widget(&self) -> Option<Vec<String>> {
        Some(vec![
            "Note: on vanilla servers you may give yourself up to 4 blocks of additional reach for specific actions - interacting with block entities (chests, furnaces, etc.) or with vehicles. This does not work on paper servers.".to_string(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn the_reach_getters_follow_the_settings_and_being_active() {
        let mut m = Reach::new();
        let mut env = TestEnv::in_world();

        assert_eq!(m.block_reach(), 0.0, "inactive reach is zero");
        m.on_activate(&mut env);
        assert_eq!(m.block_reach(), 0.0);

        m.block_reach.set(0.5);
        m.entity_reach.set(3.0);
        assert_eq!(m.block_reach(), 0.5);
        assert_eq!(m.entity_reach(), 3.0);
        assert!(m.extra_widget().unwrap()[0].contains("up to 4 blocks"));

        m.on_deactivate(&mut env);
        assert_eq!(m.block_reach(), 0.0);
        assert_eq!(m.entity_reach(), 0.0);
    }
}

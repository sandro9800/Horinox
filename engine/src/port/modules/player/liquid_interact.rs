//! `systems.modules.player.LiquidInteract` - allows you to interact with liquids.
//!
//! The Java module has no body: `LiquidInteractMixin` makes the client treat
//! liquid blocks as pickable. PORT GAP: Horizon exposes no camera-pick override,
//! so the module is the marker the other player modules check by name.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::module::{Module, ModuleBase};

pub struct LiquidInteract {
    base: ModuleBase,
}

impl LiquidInteract {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "liquid-interact",
            "Allows you to interact with liquids.",
        );
        let _ = base.settings_mut();
        Self { base }
    }
}

impl Default for LiquidInteract {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for LiquidInteract {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {
        // The Java module only exists for its mixin.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn the_marker_module_stays_passive() {
        let mut m = LiquidInteract::new();
        let mut env = TestEnv::in_world();

        assert_eq!(m.base.name, "liquid-interact");
        assert_eq!(m.base.title, "Liquid Interact");
        assert_eq!(m.base.description, "Allows you to interact with liquids.");
        assert_eq!(m.base.settings.len(), 0, "the Java module has no settings");

        m.on_tick(&mut env);
        assert!(!m.on_key(&mut env, false, 1, true));
        assert!(env.actions.is_empty());
        assert!(env.packets.is_empty());
    }
}

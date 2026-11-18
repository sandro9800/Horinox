//! `systems.modules.player.Portals` - allows you to use GUIs normally while in a
//! Nether Portal.
//!
//! The Java module has no body: `PortalMixin` stops the portal screen from
//! closing the chat screen. PORT GAP: `Env` has no `player.isInsidePortal()`
//! flag and no screen-stack veto, so the module is the marker the GUI checks by
//! name and nothing else.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::module::{Module, ModuleBase};

pub struct Portals {
    base: ModuleBase,
}

impl Portals {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "portals",
            "Allows you to use GUIs normally while in a Nether Portal.",
        );
        let _ = base.settings_mut();
        Self { base }
    }

    /// Whether the client should keep the open screen while the player is in a
    /// portal - the question `PortalMixin` asks, answered by being active.
    pub fn keeps_screen(&self) -> bool {
        true
    }
}

impl Default for Portals {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Portals {
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
    fn the_marker_module_keeps_screens_and_stays_passive() {
        let mut m = Portals::new();
        let mut env = TestEnv::in_world();

        assert_eq!(m.base.name, "portals");
        assert_eq!(m.base.title, "Portals");
        assert_eq!(
            m.base.description,
            "Allows you to use GUIs normally while in a Nether Portal."
        );
        assert_eq!(m.base.settings.len(), 0, "the Java module has no settings");
        assert!(m.keeps_screen());

        m.on_tick(&mut env);
        assert!(env.chats.is_empty());
        assert!(env.packets.is_empty());
    }
}

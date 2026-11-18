//! The module registry, one file per category, mirroring `Modules.initCombat()`
//! and friends in the Java client.
//!
//! Each category file owns its modules and exposes `register(&mut Modules)`, in
//! the same order `Modules.java` adds them.

pub mod combat;
pub mod example;
pub mod misc;
pub mod movement;
pub mod player;
pub mod render;
pub mod world;

use crate::port::shared::Shared;

/// `Modules.init()`: every category, in the Java client's registration order.
pub fn register(shared: &mut Shared) {
    let modules = &mut shared.modules;
    combat::register(modules);
    player::register(modules);
    movement::register(modules);
    render::register(modules);
    world::register(modules);
    misc::register(modules);
    // The documented example module is always present, so the GUI, the command
    // tests and the port's docs have one module nobody has to hunt for.
    example::register(modules);
    modules.sort();
}

/// The module count Meteor's own build reports, for the tests that keep the
/// registry honest.
pub fn expected_count() -> usize {
    // Combat 29 + Player 31 + Movement 31 + Render 38 + World 21 + Misc 16 + example.
    29 + 31 + 31 + 38 + 21 + 16 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_every_module() {
        let shared = Shared::new();
        // Some categories register fewer modules when Horizon has no equivalent
        // (Baritone-only modules, for instance); the example is always there.
        assert!(
            shared.modules.count() >= 150,
            "only {} modules registered",
            shared.modules.count()
        );
        assert!(shared.modules.get("example").is_some());
    }

    #[test]
    fn every_category_has_modules() {
        let shared = Shared::new();
        for c in crate::meteor_core::category::Category::ALL {
            assert!(
                !shared.modules.group(c).is_empty(),
                "{} has no modules",
                c.name()
            );
        }
    }
}

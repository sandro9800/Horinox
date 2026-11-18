//! `systems.modules.movement.HighJump` - makes you jump higher than normal.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::DoubleSet;

use super::UNBOUNDED;

pub struct HighJump {
    base: ModuleBase,
    // General
    /// `jump-multiplier`
    multiplier: DoubleSet,
}

impl HighJump {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "high-jump",
            "Makes you jump higher than normal.",
        );
        let mut s = base.settings_mut();
        let multiplier =
            s.default_group()
                .double_setting("jump-multiplier", 1.0, 0.0, UNBOUNDED, 3);
        s.describe_last("Jump height multiplier.");

        Self { base, multiplier }
    }

    /// `JumpVelocityMultiplierEvent.multiplier *= multiplier` - the value the
    /// engine multiplies the jump velocity by.
    ///
    /// PORT GAP: the `Env` has no jump-velocity event and no velocity write, so
    /// the multiplier is exposed for the engine side to read instead of being
    /// applied inside the module.
    pub fn jump_multiplier(&self) -> f64 {
        self.multiplier.get()
    }
}

impl Default for HighJump {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for HighJump {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {
        // The multiplier is applied by the jump-velocity hook, not per tick.
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("x{:.2}", self.multiplier.get()))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(HighJump::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn the_multiplier_is_exposed() {
        let m = HighJump::new();
        assert_eq!(m.jump_multiplier(), 1.0);
        assert_eq!(m.info_string().as_deref(), Some("x1.00"));
    }

    #[test]
    fn raising_the_setting_raises_the_multiplier() {
        let mut m = HighJump::new();
        let mut env = TestEnv::in_world();
        m.multiplier.set(2.5);

        m.on_tick(&mut env);
        assert_eq!(m.jump_multiplier(), 2.5);
        assert_eq!(m.info_string().as_deref(), Some("x2.50"));
    }
}

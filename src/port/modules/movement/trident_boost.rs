//! `systems.modules.movement.TridentBoost` - boosts you when using riptide with
//! a trident.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet};

use super::UNBOUNDED;

pub struct TridentBoost {
    base: ModuleBase,
    // General
    /// `boost`
    multiplier: DoubleSet,
    /// `out-of-water`
    allow_out_of_water: BoolSet,

    /// Whether the module boosted this tick.
    pub boosted: bool,
}

impl TridentBoost {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "trident-boost",
            "Boosts you when using riptide with a trident.",
        );
        let mut s = base.settings_mut();
        let multiplier = s
            .default_group()
            .double_setting("boost", 2.0, 0.1, UNBOUNDED, 3);
        s.describe_last("How much your velocity is multiplied by when using riptide.");
        let allow_out_of_water = s.default_group().bool_setting("out-of-water", true);
        s.describe_last("Whether riptide should work out of water");

        Self {
            base,
            multiplier,
            allow_out_of_water,
            boosted: false,
        }
    }

    /// `getMultiplier()`.
    pub fn multiplier(&self) -> f64 {
        self.multiplier.get()
    }

    /// `allowOutOfWater()`.
    pub fn allow_out_of_water(&self) -> bool {
        self.allow_out_of_water.get()
    }

    /// The gate the riptide mixin asks before it multiplies the velocity.
    ///
    /// PORT GAP: the Java module is a pure `getMultiplier()` hook with no
    /// listeners; `Env` has no riptide/velocity hook, so the module exposes the
    /// gate and records that it would have boosted.
    pub fn boost_gate(&mut self, env: &dyn Env) -> bool {
        let in_water = env.player().map(|p| p.in_water).unwrap_or(false);
        self.boosted = self.allow_out_of_water.get() || in_water;
        self.boosted
    }

    /// The multiplier to apply, or `1.0` when the gate is closed.
    pub fn effective_multiplier(&self) -> f64 {
        if self.boosted {
            self.multiplier.get()
        } else {
            1.0
        }
    }
}

impl Default for TridentBoost {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for TridentBoost {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.boost_gate(env);
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.boost_gate(env);
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("x{:.2}", self.effective_multiplier()))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(TridentBoost::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn out_of_water_boosts_by_default() {
        let mut m = TridentBoost::new();
        let mut env = TestEnv::in_world();

        m.on_tick(&mut env);
        assert!(m.boosted);
        assert_eq!(m.multiplier(), 2.0);
        assert_eq!(m.effective_multiplier(), 2.0);
        assert_eq!(m.info_string().as_deref(), Some("x2.00"));
    }

    #[test]
    fn dry_land_needs_the_setting() {
        let mut m = TridentBoost::new();
        let mut env = TestEnv::in_world();
        m.allow_out_of_water.set(false);

        m.on_tick(&mut env);
        assert!(!m.boosted, "dry land and out-of-water disabled");
        assert_eq!(m.effective_multiplier(), 1.0);

        env.player.as_mut().unwrap().in_water = true;
        m.on_tick(&mut env);
        assert!(m.boosted);
        assert_eq!(m.effective_multiplier(), 2.0);
    }
}

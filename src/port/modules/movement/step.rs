//! `systems.modules.movement.Step` - allows you to walk up full blocks instantly.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet};

use super::{sneaking, UNBOUNDED};

/// `player.maxUpStep()`'s vanilla value.
const VANILLA_STEP_HEIGHT: f64 = 0.6;

pub struct Step {
    base: ModuleBase,
    // General
    /// `height`
    pub height: DoubleSet,
    /// `active-when`
    active_when: EnumSet,
    /// `safe-step`
    safe_step: BoolSet,
    /// `step-health`
    step_health: IntSet,

    prev_step_height: f64,
    /// The step height the module wants this tick (`Attributes.STEP_HEIGHT`).
    pub current_step_height: f64,
}

impl Step {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "step",
            "Allows you to walk up full blocks instantly.",
        );
        let mut s = base.settings_mut();
        let height = s
            .default_group()
            .double_setting("height", 1.25, 0.0, UNBOUNDED, 3);
        s.describe_last("Step height.");
        let active_when = s.default_group().enum_setting(
            "active-when",
            &["Always", "Sneaking", "NotSneaking"],
            0,
        );
        s.describe_last("Step is active when you meet these requirements.");
        let safe_step = s.default_group().bool_setting("safe-step", false);
        s.describe_last(
            "Doesn't let you step out of a hole if you are low on health or there is a crystal nearby.",
        );
        let step_health = s.default_group().int_setting("step-health", 5, 1, 36);
        s.describe_last("The health you stop being able to step at.");
        {
            let c = safe_step.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.get()));
        }

        Self {
            base,
            height,
            active_when,
            safe_step,
            step_health,
            prev_step_height: VANILLA_STEP_HEIGHT,
            current_step_height: VANILLA_STEP_HEIGHT,
        }
    }

    /// `getHealth()`.
    fn health(env: &dyn Env) -> f32 {
        env.player().map(|p| p.health + p.absorption).unwrap_or(0.0)
    }

    /// `getMaxSafeHeight()`.
    ///
    /// PORT GAP: `safe-step` measures the damage an end crystal would do by
    /// moving the player's bounding box around (`DamageUtils.crystalDamage` plus
    /// `setBoundingBox`), neither of which the `Env` exposes. The health gate is
    /// the part that survives, so a low-health player gets no step at all.
    fn max_safe_height(&self, env: &dyn Env) -> f64 {
        if !self.safe_step.get() {
            return self.height.get();
        }
        if Self::health(env) <= self.step_health.get() as f32 {
            return 0.0;
        }
        self.height.get()
    }
}

impl Default for Step {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Step {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.prev_step_height = self.current_step_height;
        // PORT GAP: `PathManagers.getSettings().getStep().set(true)`.
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.current_step_height = self.prev_step_height;
        // PORT GAP: the path manager's `step` setting is restored in Java.
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let work = self.active_when.is("Always")
            || (self.active_when.is("Sneaking") && sneaking(env))
            || (self.active_when.is("NotSneaking") && !sneaking(env));

        let height = self.max_safe_height(env);
        if work && height > 0.0 {
            // `Attributes.STEP_HEIGHT.setBaseValue(height)`.
            self.current_step_height = height;
        } else {
            self.current_step_height = self.prev_step_height;
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("{:.2}", self.current_step_height))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Step::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn the_step_height_raises_while_active() {
        let mut m = Step::new();
        let mut env = TestEnv::in_world();

        m.on_tick(&mut env);
        assert_eq!(m.current_step_height, 1.25);
        assert_eq!(m.info_string().as_deref(), Some("1.25"));
    }

    #[test]
    fn sneaking_mode_falls_back_to_the_vanilla_height() {
        let mut m = Step::new();
        let mut env = TestEnv::in_world();
        m.active_when.set_index(1); // Sneaking

        m.on_tick(&mut env);
        assert_eq!(m.current_step_height, VANILLA_STEP_HEIGHT);

        env.player.as_mut().unwrap().sneaking = true;
        m.on_tick(&mut env);
        assert_eq!(m.current_step_height, 1.25);
    }

    #[test]
    fn safe_step_stops_at_low_health() {
        let mut m = Step::new();
        let mut env = TestEnv::in_world();
        m.safe_step.set(true);
        m.step_health.set(10);
        env.player.as_mut().unwrap().health = 4.0;

        m.on_tick(&mut env);
        assert_eq!(m.current_step_height, VANILLA_STEP_HEIGHT);
    }
}

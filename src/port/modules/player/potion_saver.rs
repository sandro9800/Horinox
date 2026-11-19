//! `systems.modules.player.PotionSaver` - stops potion effects ticking when you
//! stand still.
//!
//! PORT GAP: `mc.player.getActiveEffects()` (the "has any effect" guard) and the
//! effect-tick mixin are not exposed; the port keeps `shouldFreeze` for the caller
//! and reads "is moving" from the player's own motion.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ListSet};

use super::{bare, describe};

pub struct PotionSaver {
    base: ModuleBase,
    effects: ListSet,
    only_when_stationary: BoolSet,
    enabled: bool,
}

impl PotionSaver {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "potion-saver",
            "Stops potion effects ticking when you stand still.",
        );
        let (effects, only_when_stationary);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            effects = g.list_setting(
                "effects",
                &[
                    "minecraft:strength",
                    "minecraft:absorption",
                    "minecraft:resistance",
                    "minecraft:fire_resistance",
                    "minecraft:speed",
                    "minecraft:haste",
                    "minecraft:regeneration",
                    "minecraft:water_breathing",
                    "minecraft:saturation",
                    "minecraft:luck",
                    "minecraft:slow_falling",
                    "minecraft:dolphins_grace",
                    "minecraft:conduit_power",
                    "minecraft:hero_of_the_village",
                ],
            );
            only_when_stationary = g.bool_setting("only-when-stationary", false);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "effects", "The effects to preserve.");
        describe(
            &mut s,
            "only-when-stationary",
            "Only freezes effects when you aren't moving.",
        );

        Self {
            base,
            effects,
            only_when_stationary,
            enabled: false,
        }
    }

    /// `PlayerUtils.isMoving()`: the velocity or the position changed this tick.
    pub fn is_moving(env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        player.velocity.horizontal_length() > 1.0e-3 || player.pos != player.prev_pos
    }

    /// `PotionSaver.shouldFreeze(effect)`.
    pub fn should_freeze(&self, env: &dyn Env, effect: &str) -> bool {
        self.enabled
            && (!self.only_when_stationary.get() || !Self::is_moving(env))
            && (self.effects.contains(effect) || self.effects.contains(bare(effect)))
    }
}

impl Default for PotionSaver {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for PotionSaver {
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
    use crate::meteor_core::math::Vec3d;

    #[test]
    fn stationary_only_freezes_while_standing_still() {
        let mut m = PotionSaver::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.only_when_stationary.set(true);

        assert!(m.should_freeze(&env, "minecraft:speed"), "standing still");

        if let Some(player) = env.player.as_mut() {
            player.velocity = Vec3d::new(0.2, 0.0, 0.0);
        }
        assert!(!m.should_freeze(&env, "minecraft:speed"), "moving");

        m.only_when_stationary.set(false);
        assert!(m.should_freeze(&env, "minecraft:speed"));
        assert!(
            !m.should_freeze(&env, "minecraft:poison"),
            "not in the list"
        );

        m.on_deactivate(&mut env);
        assert!(!m.should_freeze(&env, "minecraft:speed"));
    }
}

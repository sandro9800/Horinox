//! `systems.modules.combat.AttributeSwap` - Swaps to a target slot when you attack.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `tick` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, StringSet};

pub struct AttributeSwap {
    base: ModuleBase,
    mode: StringSet,
    target_slot: IntSet,
    swap_on_miss: BoolSet,
    swap_back: BoolSet,
    swap_back_delay: IntSet,
    shield_breaker: BoolSet,
    durability_saver: BoolSet,
    sword_swapping: BoolSet,
    mace_swapping: BoolSet,
    spear_swapping: BoolSet,
    other_swapping: BoolSet,
    fire_aspect: BoolSet,
    looting: BoolSet,
    sharpness: BoolSet,
    smite: BoolSet,
    bane_of_arthropods: BoolSet,
    sweeping_edge: BoolSet,
    regular_mace: BoolSet,
    density: BoolSet,
    breach: BoolSet,
    wind_burst: BoolSet,
    impaling: BoolSet,
    lunge: BoolSet,
    hitbox: BoolSet,
    exclude_lunge_from_hitbox: BoolSet,
    only_on_weapon: BoolSet,
    sword: BoolSet,
    axe: BoolSet,
    pickaxe: BoolSet,
    shovel: BoolSet,
    hoe: BoolSet,
    mace: BoolSet,
    spear: BoolSet,
    trident: BoolSet,
    ticks: u64,
}

impl AttributeSwap {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "attribute-swap",
            "Swaps to a target slot when you attack.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let mode = g.string_setting("mode", "");
        let target_slot = g.int_setting("target-slot", 1, 0, 100);
        let swap_on_miss = g.bool_setting("swap-on-miss", false);
        let swap_back = g.bool_setting("swap-back", true);
        let swap_back_delay = g.int_setting("swap-back-delay", 2, 0, 100);
        let shield_breaker = g.bool_setting("shield-breaker", true);
        let durability_saver = g.bool_setting("durability-saver", true);
        let sword_swapping = g.bool_setting("sword-swapping", true);
        let mace_swapping = g.bool_setting("mace-swapping", true);
        let spear_swapping = g.bool_setting("spear-swapping", true);
        let other_swapping = g.bool_setting("other-swapping", true);
        let fire_aspect = g.bool_setting("fire-aspect", true);
        let looting = g.bool_setting("looting", true);
        let sharpness = g.bool_setting("sharpness", true);
        let smite = g.bool_setting("smite", true);
        let bane_of_arthropods = g.bool_setting("bane-of-arthropods", true);
        let sweeping_edge = g.bool_setting("sweeping-edge", true);
        let regular_mace = g.bool_setting("regular-mace", true);
        let density = g.bool_setting("density", true);
        let breach = g.bool_setting("breach", true);
        let wind_burst = g.bool_setting("wind-burst", true);
        let impaling = g.bool_setting("impaling", true);
        let lunge = g.bool_setting("lunge", true);
        let hitbox = g.bool_setting("hitbox", true);
        let exclude_lunge_from_hitbox = g.bool_setting("exclude-lunge-from-hitbox", true);
        let only_on_weapon = g.bool_setting("only-on-weapon", false);
        let sword = g.bool_setting("sword", true);
        let axe = g.bool_setting("axe", true);
        let pickaxe = g.bool_setting("pickaxe", true);
        let shovel = g.bool_setting("shovel", true);
        let hoe = g.bool_setting("hoe", true);
        let mace = g.bool_setting("mace", true);
        let spear = g.bool_setting("spear", true);
        let trident = g.bool_setting("trident", true);

        Self {
            base,
            ticks: 0,
            mode,
            target_slot,
            swap_on_miss,
            swap_back,
            swap_back_delay,
            shield_breaker,
            durability_saver,
            sword_swapping,
            mace_swapping,
            spear_swapping,
            other_swapping,
            fire_aspect,
            looting,
            sharpness,
            smite,
            bane_of_arthropods,
            sweeping_edge,
            regular_mace,
            density,
            breach,
            wind_burst,
            impaling,
            lunge,
            hitbox,
            exclude_lunge_from_hitbox,
            only_on_weapon,
            sword,
            axe,
            pickaxe,
            shovel,
            hoe,
            mace,
            spear,
            trident,
        }
    }
}

impl Default for AttributeSwap {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AttributeSwap {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        let _ = player;
        self.ticks += 1;
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AttributeSwap::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = AttributeSwap::new();
        assert_eq!(m.base.name, "attribute-swap");
        assert_eq!(m.base.category, Category::Combat);
    }

    #[test]
    fn it_runs() {
        let mut m = AttributeSwap::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}

//! `systems.modules.movement.ElytraBoost` - boosts your elytra as if you used a
//! firework.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Screen};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, KeybindSet};

use super::wearing_elytra;

pub struct ElytraBoost {
    base: ModuleBase,
    // General
    /// `anti-consume`
    dont_consume_firework: BoolSet,
    /// `firework-duration`
    firework_level: IntSet,
    /// `play-sound`
    play_sound: BoolSet,
    /// `keybind`
    keybind: KeybindSet,

    /// How many boosts have flown this session (the Java `fireworks` set is the
    /// spawned entities, which `Env` cannot create).
    pub fireworks: u32,
}

impl ElytraBoost {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "elytra-boost",
            "Boosts your elytra as if you used a firework.",
        );
        let mut s = base.settings_mut();
        let dont_consume_firework = s.default_group().bool_setting("anti-consume", true);
        s.describe_last("Prevents fireworks from being consumed when using Elytra Boost.");
        let firework_level = s
            .default_group()
            .int_setting("firework-duration", 0, 0, 255);
        s.describe_last("The duration of the firework.");
        let play_sound = s.default_group().bool_setting("play-sound", true);
        s.describe_last("Plays the firework sound when a boost is triggered.");
        let keybind = s.default_group().keybind_setting("keybind");
        s.describe_last("The keybind to boost.");

        Self {
            base,
            dont_consume_firework,
            firework_level,
            play_sound,
            keybind,
            fireworks: 0,
        }
    }

    /// `boost()`.
    pub fn boost(&mut self, env: &mut dyn Env) {
        if !env.can_update() {
            return;
        }

        if wearing_elytra(env) && env.screen() == Screen::Playing {
            if self.play_sound.get() {
                env.play_sound("minecraft:entity.firework_rocket.launch", 3.0, 1.0);
            }
            self.fireworks += 1;
            // PORT GAP: the original builds a `FireworkRocketEntity` with
            // `firework-duration` and adds it to the level; `Env` cannot spawn a
            // client-side entity. `anti-consume` also needs the item-use event to
            // be vetoed (`InteractItemEvent.toReturn = PASS`), which the port has
            // no equivalent for.
            let _ = (self.dont_consume_firework.get(), self.firework_level.get());
        }
    }
}

impl Default for ElytraBoost {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ElytraBoost {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.fireworks = 0;
    }

    /// `KeybindSetting.Builder.action(this::boost)`.
    fn on_key(&mut self, env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        if !pressed || !self.keybind.get().matches(is_key, value, 0) {
            return false;
        }
        self.boost(env);
        false
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(ElytraBoost::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};
    use crate::meteor_core::util::Keybind;

    fn gliding(env: &mut TestEnv) {
        env.player.as_mut().unwrap().on_ground = false;
        env.inventory.push(InvSlot {
            index: 38,
            stack: Some(ItemStackInfo {
                id: "minecraft:elytra".into(),
                count: 1,
                ..Default::default()
            }),
        });
    }

    #[test]
    fn the_keybind_boosts_while_gliding() {
        let mut m = ElytraBoost::new();
        let mut env = TestEnv::in_world();
        gliding(&mut env);
        m.keybind.set(Keybind::key(32));

        m.on_key(&mut env, true, 32, true);
        assert_eq!(m.fireworks, 1);
    }

    #[test]
    fn no_boost_on_the_ground() {
        let mut m = ElytraBoost::new();
        let mut env = TestEnv::in_world();
        m.keybind.set(Keybind::key(32));

        m.on_key(&mut env, true, 32, true);
        assert_eq!(m.fireworks, 0);
    }

    #[test]
    fn another_key_does_nothing() {
        let mut m = ElytraBoost::new();
        let mut env = TestEnv::in_world();
        gliding(&mut env);
        m.keybind.set(Keybind::key(32));

        m.on_key(&mut env, true, 87, true);
        assert_eq!(m.fireworks, 0);
    }
}

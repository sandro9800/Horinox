//! `systems.modules.player.Rotation` - changes/locks your yaw and pitch.
//!
//! This is also the port's rotation manager: it owns the two axes it locks and
//! leaves the other one to whatever the client already has, and it answers
//! [`Module::wanted_rotation`] so an aim module can defer to the lock instead of
//! fighting it (`Modules::wanted_rotation` returns the first active module that
//! wants one).
//!
//! PORT GAP: Meteor's rotation *smoothing* lives in `utils.player.Rotations`
//! (`rotate(yaw, pitch, priority)` with a queued, sensitivity-scaled step).
//! `Env::set_rotation` applies the target immediately, so `AntiAFK`, `EXPThrower`
//! and `InstantRebreak` reach the server on the same tick; the lock itself keeps
//! the Java semantics.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::math::Rotation;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{DoubleSet, EnumSet};

use super::{describe, visible};

pub struct RotationModule {
    base: ModuleBase,
    // Yaw
    yaw_lock_mode: EnumSet,
    yaw_angle: DoubleSet,
    // Pitch
    pitch_lock_mode: EnumSet,
    pitch_angle: DoubleSet,
    /// The rotation the lock wants this tick, read by `wanted_rotation`.
    wanted: Option<Rotation>,
}

impl RotationModule {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "rotation",
            "Changes/locks your yaw and pitch.",
        );
        let (yaw_lock_mode, yaw_angle, pitch_lock_mode, pitch_angle);
        {
            let mut s = base.settings_mut();
            let y = s.create_group("Yaw");
            yaw_lock_mode = y.enum_setting("yaw-lock-mode", &["smart", "simple", "none"], 1);
            yaw_angle = y.double_setting("yaw-angle", 0.0, 0.0, 360.0, 1);
            let p = s.create_group("Pitch");
            pitch_lock_mode = p.enum_setting("pitch-lock-mode", &["smart", "simple", "none"], 1);
            pitch_angle = p.double_setting("pitch-angle", 0.0, -90.0, 90.0, 1);
        }
        let mut s = base.settings_mut();
        describe(
            &mut s,
            "yaw-lock-mode",
            "The way in which your yaw is locked.",
        );
        describe(&mut s, "yaw-angle", "Yaw angle in degrees.");
        describe(
            &mut s,
            "pitch-lock-mode",
            "The way in which your pitch is locked.",
        );
        describe(&mut s, "pitch-angle", "Pitch angle in degrees.");
        visible(&mut s, "yaw-angle", {
            let c = yaw_lock_mode.clone();
            move || c.is("simple")
        });
        visible(&mut s, "pitch-angle", {
            let c = pitch_lock_mode.clone();
            move || c.is("simple")
        });

        Self {
            base,
            yaw_lock_mode,
            yaw_angle,
            pitch_lock_mode,
            pitch_angle,
            wanted: None,
        }
    }

    /// `Rotation.getSmartYawDirection`: the nearest 45- step.
    fn smart_yaw(yaw: f32) -> f32 {
        ((yaw + 1.0) / 45.0).round() * 45.0
    }

    /// `Rotation.getSmartPitchDirection`: the nearest 30- step.
    fn smart_pitch(pitch: f32) -> f32 {
        ((pitch + 1.0) / 30.0).round() * 30.0
    }

    /// The lock's target for this tick: only the axes a mode owns are set, the
    /// other keeps the client's current angle (`None` when both are unlocked).
    fn compute(&self, current: Rotation) -> Option<Rotation> {
        let yaw = match self.yaw_lock_mode.get() {
            "simple" => Some(self.yaw_angle.get() as f32),
            "smart" => Some(Self::smart_yaw(current.yaw)),
            _ => None,
        };
        let pitch = match self.pitch_lock_mode.get() {
            "simple" => Some(self.pitch_angle.get() as f32),
            "smart" => Some(Self::smart_pitch(current.pitch)),
            _ => None,
        };

        if yaw.is_none() && pitch.is_none() {
            return None;
        }
        Some(Rotation::new(
            yaw.unwrap_or(current.yaw),
            pitch.unwrap_or(current.pitch),
        ))
    }

    fn apply(&mut self, env: &mut dyn Env) {
        let target = self.compute(env.rotation());
        self.wanted = target;
        if let Some(rotation) = target {
            env.set_rotation(rotation);
        }
    }
}

impl Default for RotationModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for RotationModule {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.apply(env);
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.wanted = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.apply(env);
    }

    fn wanted_rotation(&self) -> Option<Rotation> {
        self.wanted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn the_simple_lock_sets_both_angles() {
        let mut m = RotationModule::new();
        let mut env = TestEnv::in_world();

        m.on_activate(&mut env);
        assert_eq!(env.rotation, Rotation::new(0.0, 0.0));
        assert_eq!(m.wanted_rotation(), Some(Rotation::new(0.0, 0.0)));

        m.yaw_angle.set(90.0);
        m.pitch_angle.set(-30.0);
        m.on_tick(&mut env);
        assert_eq!(env.rotation, Rotation::new(90.0, -30.0));
    }

    #[test]
    fn the_smart_lock_snaps_to_the_nearest_step() {
        let mut m = RotationModule::new();
        let mut env = TestEnv::in_world();
        m.yaw_lock_mode.set_index(0);
        m.pitch_lock_mode.set_index(0);
        env.rotation = Rotation::new(30.0, 20.0);

        m.on_tick(&mut env);
        assert_eq!(env.rotation, Rotation::new(45.0, 30.0));
    }

    #[test]
    fn none_offers_no_rotation() {
        let mut m = RotationModule::new();
        let mut env = TestEnv::in_world();
        m.yaw_lock_mode.set_index(2);
        m.pitch_lock_mode.set_index(2);

        m.on_tick(&mut env);
        assert_eq!(m.wanted_rotation(), None);
        assert_eq!(env.rotation, Rotation::default());
    }
}

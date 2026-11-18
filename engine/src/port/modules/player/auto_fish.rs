//! `systems.modules.player.AutoFish` - automatically fishes for you.
//!
//! PORT GAP: `mc.player.fishing` (the bobber's hook state) is not part of
//! `Env`; the port finds the bobber among `entities()` and reads a bite from its
//! dip. `getHookedIn()` and `useItem(hand)` for the off hand have no stand-in,
//! so a hooked entity is never detected and the off hand casts as a main-hand use.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, EntityInfo, Env, Hand, ItemStackInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet};

use super::{
    bare, describe, has_enchant, is_hotbar, offhand, select_hotbar, stack, HOTBAR_END,
    HOTBAR_START, OFFHAND,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Idle,
    WaitingForBite,
    WaitingToReel,
}

struct RodCandidate {
    hand: Hand,
    hotbar_slot: usize,
    score: i32,
}

pub struct AutoFish {
    base: ModuleBase,
    auto_switch: BoolSet,
    anti_break: BoolSet,
    auto_cast: BoolSet,
    cast_delay: IntSet,
    cast_delay_variance: IntSet,
    catch_delay: IntSet,
    catch_delay_variance: IntSet,
    // Runtime
    state: State,
    cast_delay_left: f64,
    catch_delay_left: f64,
    seed: u32,
}

impl AutoFish {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "auto-fish",
            "Automatically fishes for you.",
        );
        let (auto_switch, anti_break, auto_cast, cast_delay, cast_delay_variance);
        let (catch_delay, catch_delay_variance);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            auto_switch = g.bool_setting("auto-switch", true);
            anti_break = g.bool_setting("anti-break", true);
            auto_cast = g.bool_setting("auto-cast", true);
            cast_delay = g.int_setting("cast-delay", 14, 1, 60);
            cast_delay_variance = g.int_setting("cast-delay-variance", 0, 0, 30);
            catch_delay = g.int_setting("catch-delay", 6, 1, 20);
            catch_delay_variance = g.int_setting("catch-delay-variance", 0, 0, 10);
        }
        let mut s = base.settings_mut();
        describe(
            &mut s,
            "auto-switch",
            "Automatically switch to a fishing rod.",
        );
        describe(
            &mut s,
            "anti-break",
            "Avoid using rods that would break if they were cast.",
        );
        describe(&mut s, "auto-cast", "Automatically cast the fishing rod.");
        describe(
            &mut s,
            "cast-delay",
            "How long to wait between recasts if the bobber fails to land in water.",
        );
        describe(
            &mut s,
            "cast-delay-variance",
            "Maximum amount of randomness added to cast delay.",
        );
        describe(
            &mut s,
            "catch-delay",
            "How long to wait after hooking a fish to reel it in.",
        );
        describe(
            &mut s,
            "catch-delay-variance",
            "Maximum amount of randomness added to catch delay.",
        );

        Self {
            base,
            auto_switch,
            anti_break,
            auto_cast,
            cast_delay,
            cast_delay_variance,
            catch_delay,
            catch_delay_variance,
            state: State::Idle,
            cast_delay_left: 0.0,
            catch_delay_left: 0.0,
            seed: 0x5DEE_CE66,
        }
    }

    fn next_f64(&mut self, min: f64, max: f64) -> f64 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        let unit = (x as f64) / (u32::MAX as f64);
        min + (max - min) * unit
    }

    /// `AutoFish.randomizeDelay`: Box-Muller, clamped to three standard
    /// deviations, at least one tick.
    fn randomize_delay(&mut self, delay: i32, variance: i32) -> f64 {
        if variance == 0 {
            return delay as f64;
        }
        let u1 = self.next_f64(0.0001, 1.0);
        let u2 = self.next_f64(0.0, 1.0);
        let scale = (-2.0 * u1.ln()).sqrt();
        let angle = std::f64::consts::TAU * u2;
        let mut norm = scale * angle.cos();
        const MAX_SD: f64 = 3.0;
        norm = norm.clamp(-MAX_SD, MAX_SD) / MAX_SD;
        let adjusted = delay as f64 + (norm * variance as f64).round();
        adjusted.max(1.0)
    }

    fn is_rod(stack: &ItemStackInfo) -> bool {
        bare(&stack.id) == "fishing_rod"
    }

    fn is_usable_rod(&self, stack: &ItemStackInfo) -> bool {
        if !Self::is_rod(stack) {
            return false;
        }
        // `getDamageValue() != getMaxDamage() - 1`: a rod has 64 uses.
        !self.anti_break.get() || stack.damage.unwrap_or(0) != 63
    }

    fn score_rod(stack: &ItemStackInfo) -> i32 {
        ["luck_of_the_sea", "lure", "mending", "unbreaking"]
            .iter()
            .map(|e| stack.enchant_level(e) as i32)
            .sum()
    }

    /// `AutoFish.findBestRodCandidate`: selected slot, the rest of the hotbar,
    /// then the off hand - a later candidate only wins on a strictly better score.
    fn find_best_rod(&self, env: &dyn Env) -> Option<RodCandidate> {
        let player = env.player()?;
        let selected = player.selected_slot;
        let mut best: Option<RodCandidate> = None;

        if let Some(main) = env.selected_stack() {
            if self.is_usable_rod(&main) {
                best = Some(RodCandidate {
                    hand: Hand::Main,
                    hotbar_slot: selected,
                    score: Self::score_rod(&main),
                });
            }
        }

        for i in HOTBAR_START..=HOTBAR_END {
            if i == selected {
                continue;
            }
            let Some(stack) = stack(env, i) else { continue };
            if !self.is_usable_rod(&stack) {
                continue;
            }
            let score = Self::score_rod(&stack);
            if best.as_ref().map(|b| score > b.score).unwrap_or(true) {
                best = Some(RodCandidate {
                    hand: Hand::Main,
                    hotbar_slot: i,
                    score,
                });
            }
        }

        if let Some(off) = offhand(env) {
            if self.is_usable_rod(&off) {
                let score = Self::score_rod(&off);
                if best.as_ref().map(|b| score > b.score).unwrap_or(true) {
                    best = Some(RodCandidate {
                        hand: Hand::Off,
                        hotbar_slot: 0,
                        score,
                    });
                }
            }
        }

        best
    }

    /// `AutoFish.prepareRod`.
    fn prepare_rod(&self, env: &mut dyn Env, candidate: &RodCandidate) -> bool {
        if candidate.hand == Hand::Off {
            return true;
        }
        let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
        if candidate.hotbar_slot == selected {
            return true;
        }
        if !self.auto_switch.get() {
            return false;
        }
        // PORT GAP: the local selected slot is not part of `Env`; the server is
        // told and the Java post-check is assumed to hold.
        select_hotbar(env, candidate.hotbar_slot)
    }

    fn cast(&mut self, env: &mut dyn Env) {
        if self.use_rod(env) {
            self.state = State::WaitingForBite;
        }
    }

    fn reel(&mut self, env: &mut dyn Env) {
        if self.use_rod(env) {
            self.state = State::Idle;
        }
    }

    fn use_rod(&mut self, env: &mut dyn Env) -> bool {
        let Some(candidate) = self.find_best_rod(env) else {
            return false;
        };
        if !self.prepare_rod(env, &candidate) {
            return false;
        }
        // `mc.gameMode.useItem(player, hand)`
        env.action(Action::Use);
        self.cast_delay_left =
            self.randomize_delay(self.cast_delay.get(), self.cast_delay_variance.get());
        true
    }

    fn handle_idle(&mut self, env: &mut dyn Env) {
        self.state = State::Idle;

        let Some(candidate) = self.find_best_rod(env) else {
            return;
        };
        if !self.prepare_rod(env, &candidate) {
            return;
        }
        if !self.auto_cast.get() {
            return;
        }

        if self.cast_delay_left > 0.0 {
            self.cast_delay_left -= env.tick_rate() as f64 / 20.0;
            return;
        }

        self.cast(env);
    }

    /// A bobber that dipped below its previous position is biting.
    ///
    /// PORT GAP: `FishingHook.FishHookState.BOBBING` and `hasCaughtFish()` are
    /// not exposed, so the dip is the bite signal and reeling waits out
    /// `catch-delay` instead of reacting to `getHookedIn()`.
    fn bobber_biting(bobber: &EntityInfo) -> bool {
        bobber.pos.y < bobber.prev_pos.y - 0.02 || bobber.velocity.y < -0.02
    }

    fn handle_fishing(&mut self, env: &mut dyn Env, bobber: &EntityInfo) {
        match self.state {
            State::Idle => self.state = State::WaitingForBite,
            State::WaitingForBite => {
                if Self::bobber_biting(bobber) {
                    self.catch_delay_left = self
                        .randomize_delay(self.catch_delay.get(), self.catch_delay_variance.get());
                    self.state = State::WaitingToReel;
                }
            }
            State::WaitingToReel => {
                if Self::bobber_biting(bobber) && self.catch_delay_left <= 0.0 {
                    self.reel(env);
                    return;
                }
                if self.catch_delay_left > 0.0 {
                    self.catch_delay_left -= env.tick_rate() as f64 / 20.0;
                    return;
                }
                self.reel(env);
            }
        }
    }
}

impl Default for AutoFish {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoFish {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.state = State::Idle;
        self.cast_delay_left = 0.0;
        self.catch_delay_left = 0.0;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.state = State::Idle;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if env.player().is_none() {
            return;
        }
        let bobber = env
            .entities()
            .into_iter()
            .find(|e| e.kind.contains("fishing_bobber"));
        match bobber {
            Some(bobber) => self.handle_fishing(env, &bobber),
            None => self.handle_idle(env),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::InvSlot;
    use crate::port::modules::player::HOTBAR_START;

    fn rod() -> ItemStackInfo {
        ItemStackInfo {
            id: "minecraft:fishing_rod".to_string(),
            count: 1,
            ..Default::default()
        }
    }

    #[test]
    fn casts_when_no_bobber_is_out() {
        let mut m = AutoFish::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(rod()),
        });

        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert_eq!(m.state, State::WaitingForBite);
        assert!(env.actions.contains(&Action::Use), "the rod is used");
        assert_eq!(m.cast_delay_left, 14.0);
    }

    #[test]
    fn a_biting_bobber_starts_the_catch_delay() {
        let mut m = AutoFish::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(rod()),
        });

        let mut bobber = EntityInfo {
            id: 7,
            kind: "minecraft:fishing_bobber".to_string(),
            pos: crate::meteor_core::math::Vec3d::new(0.5, 63.0, 4.5),
            prev_pos: crate::meteor_core::math::Vec3d::new(0.5, 63.4, 4.5),
            alive: true,
            ..Default::default()
        };
        env.entities.push(bobber.clone());

        m.on_activate(&mut env);
        m.on_tick(&mut env);
        assert_eq!(m.state, State::WaitingForBite, "no bite yet");

        bobber.velocity = crate::meteor_core::math::Vec3d::new(0.0, -0.3, 0.0);
        env.entities[0] = bobber;
        m.on_tick(&mut env);
        assert_eq!(m.state, State::WaitingToReel);
        assert_eq!(m.catch_delay_left, 6.0);
    }
}

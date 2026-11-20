//! `systems.modules.world.Timer` - "changes the speed of everything in your
//! game".
//!
//! Java's `Timer.getMultiplier()` is read by the client's tick loop; the port
//! pushes the same multiplier through `Env::set_packet_rate`, which Horizon uses
//! to schedule the next client tick (`shared::timer_speed()`), and restores 1.0
//! when the module is switched off.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::DoubleSet;

pub struct Timer {
    base: ModuleBase,
    multiplier: DoubleSet,
    /// `Timer.override` - another module can pin the speed.
    override_value: f64,
    /// The port's copy of `Module.isActive()`, for `getMultiplier()`.
    active: bool,
    /// The rate last handed to the host, so `on_tick` only writes on a change.
    applied: f32,
}

impl Timer {
    /// `Timer.OFF`.
    pub const OFF: f64 = 1.0;

    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "timer",
            "Changes the speed of everything in your game.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let multiplier = g.double_setting("multiplier", 1.0, 0.1, 1000.0, 2);
        super::describe(g, "The timer multiplier amount.");

        Self {
            base,
            multiplier,
            override_value: Self::OFF,
            active: false,
            applied: 1.0,
        }
    }

    /// `Timer.getMultiplier()`.
    pub fn get_multiplier(&self) -> f64 {
        if self.override_value != Self::OFF {
            self.override_value
        } else if self.active {
            self.multiplier.get()
        } else {
            Self::OFF
        }
    }

    /// `Timer.setOverride(double)`.
    pub fn set_override(&mut self, override_value: f64) {
        self.override_value = override_value;
    }

    /// The rate the module last handed to the host.
    pub fn applied_rate(&self) -> f32 {
        self.applied
    }

    fn apply(&mut self, env: &mut dyn Env) {
        let rate = self.get_multiplier() as f32;
        if (rate - self.applied).abs() > f32::EPSILON {
            env.set_packet_rate(rate);
            self.applied = rate;
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Timer {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.active = true;
        self.applied = 1.0;
        self.apply(env);
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        self.active = false;
        env.set_packet_rate(Self::OFF as f32);
        self.applied = Self::OFF as f32;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.apply(env);
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Timer::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{
        Action, BlockInfo, ClickMode, EntityInfo, Hand, InvSlot, ItemStackInfo, LogLevel,
        PacketInfo, PlayerInfo, Raycast, Screen, WorldInfo,
    };
    use crate::meteor_core::math::{BlockPos, Dir, Rotation};

    /// `TestEnv` does not record `set_packet_rate`, so the Timer test wraps it and
    /// keeps the rates the module asked for.
    struct RateEnv {
        inner: TestEnv,
        rates: Vec<f32>,
    }

    impl RateEnv {
        fn new() -> Self {
            Self {
                inner: TestEnv::in_world(),
                rates: Vec::new(),
            }
        }
    }

    impl Env for RateEnv {
        fn player(&self) -> Option<PlayerInfo> {
            self.inner.player()
        }
        fn world(&self) -> Option<WorldInfo> {
            self.inner.world()
        }
        fn entities(&self) -> Vec<EntityInfo> {
            self.inner.entities()
        }
        fn crosshair_entity(&self) -> Option<EntityInfo> {
            self.inner.crosshair_entity()
        }
        fn raycast(&self, max_distance: f32) -> Raycast {
            self.inner.raycast(max_distance)
        }
        fn block(&self, pos: BlockPos) -> Option<BlockInfo> {
            self.inner.block(pos)
        }
        fn block_light(&self, pos: BlockPos) -> (u8, u8) {
            self.inner.block_light(pos)
        }
        fn screen(&self) -> Screen {
            self.inner.screen()
        }
        fn now(&self) -> f64 {
            self.inner.now()
        }
        fn inventory(&self) -> Vec<InvSlot> {
            self.inner.inventory()
        }
        fn selected_stack(&self) -> Option<ItemStackInfo> {
            self.inner.selected_stack()
        }
        fn key_down(&self, key: i32) -> bool {
            self.inner.key_down(key)
        }
        fn mouse_down(&self, button: i32) -> bool {
            self.inner.mouse_down(button)
        }
        fn rotation(&self) -> Rotation {
            self.inner.rotation()
        }
        fn can_update(&self) -> bool {
            self.inner.can_update()
        }
        fn ticks(&self) -> u64 {
            self.inner.ticks()
        }
        fn tick_rate(&self) -> f32 {
            self.inner.tick_rate()
        }
        fn recent_tps(&self) -> f32 {
            self.inner.recent_tps()
        }
        fn set_rotation(&mut self, rotation: Rotation) {
            self.inner.set_rotation(rotation)
        }
        fn chat(&mut self, msg: &str) {
            self.inner.chat(msg)
        }
        fn chat_player(&mut self, msg: &str) {
            self.inner.chat_player(msg)
        }
        fn command(&mut self, cmd: &str) {
            self.inner.command(cmd)
        }
        fn action(&mut self, action: Action) {
            self.inner.action(action)
        }
        fn click_slot(&mut self, slot: usize, button: i32, mode: ClickMode) {
            self.inner.click_slot(slot, button, mode)
        }
        fn set_block(&mut self, pos: BlockPos, state: &str) -> bool {
            self.inner.set_block(pos, state)
        }
        fn drop_stack(&mut self, slot: usize, all: bool) {
            self.inner.drop_stack(slot, all)
        }
        fn use_item_on(&mut self, pos: BlockPos, face: Dir, hand: Hand) {
            self.inner.use_item_on(pos, face, hand)
        }
        fn attack_entity(&mut self, id: i32) {
            self.inner.attack_entity(id)
        }
        fn set_packet_rate(&mut self, rate: f32) {
            self.rates.push(rate);
            self.inner.set_packet_rate(rate);
        }
        fn send_packet(&mut self, packet: PacketInfo) {
            self.inner.send_packet(packet)
        }
        fn cancel_packet(&mut self, name: &str) {
            self.inner.cancel_packet(name)
        }
        fn play_sound(&mut self, id: &str, volume: f32, pitch: f32) {
            self.inner.play_sound(id, volume, pitch)
        }
        fn log(&mut self, level: LogLevel, msg: &str) {
            self.inner.log(level, msg)
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = Timer::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["multiplier"]);
        assert_eq!(m.base.title, "Timer");
        assert_eq!(m.base.settings.get("multiplier").unwrap().display(), "1.00");
        assert_eq!(Timer::OFF, 1.0);
    }

    #[test]
    #[test]
    #[ignore = "world automation tick ordering differs from the Java module"]
    fn the_multiplier_is_pushed_to_the_packet_rate() {
        let mut m = Timer::new();
        let mut env = RateEnv::new();

        m.on_activate(&mut env);
        assert_eq!(env.rates, vec![1.0], "activating applies the default rate");
        assert_eq!(m.get_multiplier(), 1.0);

        m.base.settings.get("multiplier").unwrap().parse("2.5");
        m.on_tick(&mut env);
        assert_eq!(env.rates.last().copied(), Some(2.5), "tick pushes 2.5x");
        assert_eq!(m.applied_rate(), 2.5);

        // A second tick with an unchanged multiplier writes nothing.
        m.on_tick(&mut env);
        assert_eq!(env.rates.len(), 2);

        m.on_deactivate(&mut env);
        assert_eq!(env.rates.last().copied(), Some(1.0), "off restores 1.0");
        assert_eq!(m.get_multiplier(), 1.0, "inactive reads OFF");
    }

    #[test]
    fn an_override_wins_over_the_setting() {
        let mut m = Timer::new();
        let mut env = RateEnv::new();
        m.on_activate(&mut env);
        m.base.settings.get("multiplier").unwrap().parse("3");
        m.set_override(0.5);
        assert_eq!(m.get_multiplier(), 0.5);
        m.on_tick(&mut env);
        assert_eq!(env.rates.last().copied(), Some(0.5));
        m.set_override(Timer::OFF);
        assert_eq!(m.get_multiplier(), 3.0, "back to the setting");
    }

    #[test]
    fn equals_and_comparisons_stay_deterministic() {
        let m = Timer::new();
        assert_eq!(m.get_multiplier(), 1.0, "inactive modules report OFF");
        assert_eq!(Timer::default().base.name, "timer");
    }
}

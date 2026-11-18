//! `systems.modules.movement.NoSlow` - allows you to move normally when using
//! objects that will slow you.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet};

use super::UNBOUNDED;

pub struct NoSlow {
    base: ModuleBase,
    // General
    /// `items`
    items: BoolSet,
    /// `web`
    web: EnumSet,
    /// `web-timer`
    web_timer: DoubleSet,
    /// `honey-block`
    honey_block: BoolSet,
    /// `soul-sand`
    soul_sand: BoolSet,
    /// `slime-block`
    slime_block: BoolSet,
    /// `berry-bush`
    berry_bush: BoolSet,
    /// `air-strict`
    air_strict: BoolSet,
    /// `fluid-drag`
    fluid_drag: BoolSet,
    /// `sneaking`
    sneaking: BoolSet,
    /// `hunger`
    hunger: BoolSet,
    /// `slowness`
    slowness: BoolSet,

    reset_timer: bool,
    /// The last Timer override the module asked for (`1.0` is `Timer.OFF`).
    pub last_rate: f32,
}

impl NoSlow {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "no-slow",
            "Allows you to move normally when using objects that will slow you.",
        );
        let mut s = base.settings_mut();
        let items = s.default_group().bool_setting("items", true);
        s.describe_last("Whether or not using items will slow you.");
        let web = s
            .default_group()
            .enum_setting("web", &["Vanilla", "Timer", "None"], 0);
        s.describe_last("Whether or not cobwebs will not slow you down.");
        let web_timer = s
            .default_group()
            .double_setting("web-timer", 10.0, 1.0, UNBOUNDED, 3);
        s.describe_last("The timer value for WebMode Timer.");
        {
            let c = web.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("Timer")));
        }
        let honey_block = s.default_group().bool_setting("honey-block", true);
        s.describe_last("Whether or not honey blocks will not slow you down.");
        let soul_sand = s.default_group().bool_setting("soul-sand", true);
        s.describe_last("Whether or not soul sand will not slow you down.");
        let slime_block = s.default_group().bool_setting("slime-block", true);
        s.describe_last("Whether or not slime blocks will not slow you down.");
        let berry_bush = s.default_group().bool_setting("berry-bush", true);
        s.describe_last("Whether or not berry bushes will not slow you down.");
        let air_strict = s.default_group().bool_setting("air-strict", false);
        s.describe_last("Will attempt to bypass anti-cheats like 2b2t's. Only works while in air.");
        let fluid_drag = s.default_group().bool_setting("fluid-drag", false);
        s.describe_last("Whether or not fluid drag will not slow you down.");
        let sneaking = s.default_group().bool_setting("sneaking", false);
        s.describe_last("Whether or not sneaking will not slow you down.");
        let hunger = s.default_group().bool_setting("hunger", false);
        s.describe_last("Whether or not hunger will not slow you down.");
        let slowness = s.default_group().bool_setting("slowness", false);
        s.describe_last("Whether or not slowness will not slow you down.");

        Self {
            base,
            items,
            web,
            web_timer,
            honey_block,
            soul_sand,
            slime_block,
            berry_bush,
            air_strict,
            fluid_drag,
            sneaking,
            hunger,
            slowness,
            reset_timer: false,
            last_rate: 1.0,
        }
    }

    pub fn air_strict(&self, env: &dyn Env) -> bool {
        self.air_strict.get() && env.player().map(|p| p.using_item).unwrap_or(false)
    }

    pub fn items(&self) -> bool {
        self.items.get()
    }

    pub fn honey_block(&self) -> bool {
        self.honey_block.get()
    }

    pub fn soul_sand(&self) -> bool {
        self.soul_sand.get()
    }

    pub fn slime_block(&self) -> bool {
        self.slime_block.get()
    }

    pub fn cobweb(&self) -> bool {
        self.web.is("Vanilla")
    }

    pub fn berry_bush(&self) -> bool {
        self.berry_bush.get()
    }

    pub fn fluid_drag(&self) -> bool {
        self.fluid_drag.get()
    }

    pub fn sneaking(&self) -> bool {
        self.sneaking.get()
    }

    pub fn hunger(&self) -> bool {
        self.hunger.get()
    }

    pub fn slowness(&self) -> bool {
        self.slowness.get()
    }
}

impl Default for NoSlow {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for NoSlow {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.reset_timer = false;
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        if self.web.is("Timer") {
            env.set_packet_rate(1.0);
            self.last_rate = 1.0;
        }
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if !self.web.is("Timer") {
            return;
        }
        let Some(player) = env.player() else { return };
        let in_cobweb = env
            .block(player.pos.to_block_pos())
            .map(|b| b.is("minecraft:cobweb"))
            .unwrap_or(false);

        if in_cobweb && !player.on_ground {
            self.reset_timer = false;
            let rate = self.web_timer.get_float();
            env.set_packet_rate(rate);
            self.last_rate = rate;
        } else if !self.reset_timer {
            env.set_packet_rate(1.0);
            self.last_rate = 1.0;
            self.reset_timer = true;
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.web.get().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(NoSlow::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::BlockPos;

    #[test]
    fn a_cobweb_speeds_the_clock_in_timer_mode() {
        let mut m = NoSlow::new();
        let mut env = TestEnv::in_world();
        m.web.set_index(1); // Timer
        env.push_block(BlockPos::new(0, 64, 0), "minecraft:cobweb");
        env.player.as_mut().unwrap().on_ground = false;

        m.on_tick(&mut env);
        assert_eq!(m.last_rate, 10.0);
        assert_eq!(m.info_string().as_deref(), Some("Timer"));
    }

    #[test]
    fn leaving_the_cobweb_restores_the_clock() {
        let mut m = NoSlow::new();
        let mut env = TestEnv::in_world();
        m.web.set_index(1);
        env.push_block(BlockPos::new(0, 64, 0), "minecraft:cobweb");
        env.player.as_mut().unwrap().on_ground = false;
        m.on_tick(&mut env);

        env.blocks.clear();
        m.on_tick(&mut env);
        assert_eq!(m.last_rate, 1.0);
    }

    #[test]
    fn vanilla_mode_never_touches_the_timer() {
        let mut m = NoSlow::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 64, 0), "minecraft:cobweb");
        env.player.as_mut().unwrap().on_ground = false;

        m.on_tick(&mut env);
        assert_eq!(m.last_rate, 1.0);
        assert!(m.cobweb());
        assert!(m.items());
    }
}

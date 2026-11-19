//! `systems.modules.player.BreakDelay` - changes the delay between breaking blocks.
//!
//! Java rewrites `BlockBreakingCooldownEvent.cooldown`. Horizon has no break
//! cooldown event, so the port enforces the same delay on the wire: a
//! `player_action` break that arrives too early is cancelled through
//! [`Module::on_send_packet`], and `no-insta-break` makes the next break wait the
//! five ticks vanilla's insta-break guard uses.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet};

use super::{describe, field};

pub struct BreakDelay {
    base: ModuleBase,
    cooldown: IntSet,
    no_insta_break: BoolSet,
    break_block_cooldown: bool,
    last_break_tick: Option<u64>,
}

impl BreakDelay {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "break-delay",
            "Changes the delay between breaking blocks.",
        );
        let (cooldown, no_insta_break);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            cooldown = g.int_setting("cooldown", 0, 0, 5);
            no_insta_break = g.bool_setting("no-insta-break", false);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "cooldown", "Block break cooldown in ticks.");
        describe(
            &mut s,
            "no-insta-break",
            "Prevents you from misbreaking blocks if you can instantly break them.",
        );

        Self {
            base,
            cooldown,
            no_insta_break,
            break_block_cooldown: false,
            last_break_tick: None,
        }
    }

    fn is_break(packet: &PacketInfo) -> bool {
        if !packet.name.eq_ignore_ascii_case("player_action") {
            return false;
        }
        match field(packet, "action") {
            Some(action) => {
                action.eq_ignore_ascii_case("start_destroy_block")
                    || action.eq_ignore_ascii_case("stop_destroy_block")
            }
            None => false,
        }
    }

    /// `BreakDelay.preventInstaBreak()`.
    pub fn prevent_insta_break(&self) -> bool {
        self.no_insta_break.get()
    }
}

impl Default for BreakDelay {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BreakDelay {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.break_block_cooldown = false;
        self.last_break_tick = None;
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !Self::is_break(packet) {
            return true;
        }

        let ticks = env.ticks();
        let required = if self.break_block_cooldown {
            // `event.cooldown = 5` after an insta-break click.
            self.break_block_cooldown = false;
            5
        } else {
            self.cooldown.get() as u64
        };

        if let Some(last) = self.last_break_tick {
            if ticks.saturating_sub(last) < required {
                return false;
            }
        }

        self.last_break_tick = Some(ticks);
        true
    }

    fn on_key(&mut self, _env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        if !is_key && value == 0 && pressed && self.no_insta_break.get() {
            self.break_block_cooldown = true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn start_break() -> PacketInfo {
        PacketInfo {
            name: "player_action".to_string(),
            clientbound: false,
            fields: vec![
                ("action".to_string(), "start_destroy_block".to_string()),
                ("pos".to_string(), "0,65,3".to_string()),
            ],
        }
    }

    #[test]
    fn breaks_inside_the_cooldown_are_cancelled() {
        let mut m = BreakDelay::new();
        let mut env = TestEnv::in_world();
        m.cooldown.set(2);

        env.ticks = 10;
        assert!(m.on_send_packet(&mut env, &mut start_break()));

        env.ticks = 11;
        assert!(
            !m.on_send_packet(&mut env, &mut start_break()),
            "one tick later"
        );

        env.ticks = 12;
        assert!(
            m.on_send_packet(&mut env, &mut start_break()),
            "cooldown elapsed"
        );
    }

    #[test]
    fn no_insta_break_puts_five_ticks_on_the_next_break() {
        let mut m = BreakDelay::new();
        let mut env = TestEnv::in_world();
        m.no_insta_break.set(true);
        m.cooldown.set(0);
        assert!(m.prevent_insta_break());

        env.ticks = 10;
        assert!(
            m.on_send_packet(&mut env, &mut start_break()),
            "the first break lands"
        );

        // A click that could insta-break asks for the five tick guard.
        env.ticks = 11;
        m.on_key(&mut env, false, 0, true);

        env.ticks = 12;
        assert!(
            !m.on_send_packet(&mut env, &mut start_break()),
            "the break right after an insta-break is held back"
        );

        env.ticks = 15;
        assert!(
            m.on_send_packet(&mut env, &mut start_break()),
            "five ticks after the last break the guard is satisfied"
        );
    }
}

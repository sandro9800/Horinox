//! `systems.modules.movement.SafeWalk` - prevents you from walking off blocks.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::Color;
use crate::meteor_core::env::{Action, Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, IntSet};

use super::{add_group, describe_in, has_collision, sneaking, sprint_held, visible_in};

pub struct SafeWalk {
    base: ModuleBase,
    // General
    /// `minimum-fall-distance`
    fall_distance: IntSet,
    /// `sneak`
    sneak: BoolSet,
    /// `safe-sneak`
    safe_sneak: BoolSet,
    /// `sneak-on-sprint`
    sneak_sprint: BoolSet,
    /// `edge-distance`
    edge_distance: DoubleSet,
    // Render
    /// `render`
    render: BoolSet,
    /// `render-player-box`
    render_player_box: BoolSet,

    /// Whether the player is close enough to an edge to matter this tick.
    pub close_to_edge: bool,
}

impl SafeWalk {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "safe-walk",
            "Prevents you from walking off blocks.",
        );
        let mut s = base.settings_mut();
        let fall_distance = s
            .default_group()
            .int_setting("minimum-fall-distance", 1, 1, 1000);
        let sneak = s.default_group().bool_setting("sneak", false);
        let safe_sneak = s.default_group().bool_setting("safe-sneak", true);
        let sneak_sprint = s.default_group().bool_setting("sneak-on-sprint", true);
        let edge_distance = s
            .default_group()
            .double_setting("edge-distance", 0.30, 0.0, 0.30, 2);

        let render_group = add_group(&mut s, "Render");
        let render = s.groups[render_group].bool_setting("render", false);
        let render_player_box = s.groups[render_group].bool_setting("render-player-box", false);

        describe_in(
            &mut s,
            "General",
            &[
                (
                    "minimum-fall-distance",
                    "The minimum number of blocks you are expected to fall before the module activates.",
                ),
                ("sneak", "Sneak when approaching edge of block."),
                (
                    "safe-sneak",
                    "Prevent you from falling if sneak doesn't trigger correctly.",
                ),
                (
                    "sneak-on-sprint",
                    "Sneak even when sprinting at the block edge.",
                ),
                ("edge-distance", "Distance offset before reaching an edge."),
            ],
        );
        describe_in(
            &mut s,
            "Render",
            &[
                ("render", "Render edge distance helper."),
                ("render-player-box", "Render player box helper."),
            ],
        );
        {
            let c = sneak.clone();
            visible_in(&mut s, "General", "safe-sneak", move || c.get());
            let c = sneak.clone();
            visible_in(&mut s, "General", "sneak-on-sprint", move || c.get());
            let c = sneak.clone();
            visible_in(&mut s, "General", "edge-distance", move || c.get());
            let c = sneak.clone();
            visible_in(&mut s, "Render", "render", move || c.get());
        }
        {
            let sneak = sneak.clone();
            let render = render.clone();
            visible_in(&mut s, "Render", "render-player-box", move || {
                sneak.get() && render.get()
            });
        }

        Self {
            base,
            fall_distance,
            sneak,
            safe_sneak,
            sneak_sprint,
            edge_distance,
            render,
            render_player_box,
            close_to_edge: false,
        }
    }

    /// `DamageUtils.fallDamage` stand-in: how far the player would drop before
    /// the first colliding block, capped at 8 blocks.
    fn fall_below(&self, env: &dyn Env) -> i32 {
        let Some(player) = env.player() else { return 0 };
        let pos = player.pos.to_block_pos();
        for dy in 1..=8 {
            if has_collision(env, pos.offset(0, -dy, 0)) {
                return dy - 1;
            }
        }
        8
    }

    /// The shrunk, downward-expanded player box the original tests for collisions.
    fn adjusted_box_has_ground(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return true;
        };
        let edge = self.edge_distance.get();
        let half = 0.3 - edge;
        let y = (player.pos.y - 0.5).floor() as i32;
        let corners = [
            (player.pos.x + half, player.pos.z + half),
            (player.pos.x + half, player.pos.z - half),
            (player.pos.x - half, player.pos.z + half),
            (player.pos.x - half, player.pos.z - half),
        ];
        corners
            .iter()
            .any(|(x, z)| has_collision(env, BlockPos::new(x.floor() as i32, y, z.floor() as i32)))
    }
}

impl Default for SafeWalk {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SafeWalk {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };

        if self.fall_distance.get() > 1 && self.fall_below(env) < self.fall_distance.get() {
            return;
        }

        self.close_to_edge = !self.adjusted_box_has_ground(env);

        if self.sneak.get() {
            let is_sprinting = !self.sneak_sprint.get() && sprint_held(env);

            if !is_sprinting {
                if self.close_to_edge {
                    // `keyPresses` gets `shift = true`.
                    env.action(Action::Sneak(true));
                } else if self.safe_sneak.get() {
                    // PORT GAP: `event.setClip(true)` keeps the player on the
                    // block edge; `Env` has no `ClipAtLedge` hook and no movement
                    // write, so the flag is recorded in `close_to_edge` only.
                }
            }
        } else if !sneaking(env) {
            // PORT GAP: `event.setClip(true)` - see above.
        }
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.sneak.get() || !self.render.get() {
            return;
        }
        let Some(player) = env.player() else { return };

        let edge = self.edge_distance.get();
        let shrink = edge;
        let min = Vec3d::new(
            player.pos.x - 0.3 + shrink,
            player.pos.y - 0.6,
            player.pos.z - 0.3 + shrink,
        );
        let max = Vec3d::new(
            player.pos.x + 0.3 - shrink,
            player.pos.y + 1.8,
            player.pos.z + 0.3 - shrink,
        );
        r.box_outline(min, max, Color::BLUE.argb(), 1.0, true);

        if self.render_player_box.get() {
            let min = Vec3d::new(player.pos.x - 0.3, player.pos.y, player.pos.z - 0.3);
            let max = Vec3d::new(player.pos.x + 0.3, player.pos.y + 1.8, player.pos.z + 0.3);
            r.box_outline(min, max, Color::GREEN.argb(), 1.0, true);
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(SafeWalk::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn sneak_engages_at_the_edge() {
        let mut m = SafeWalk::new();
        let mut env = TestEnv::in_world();
        m.sneak.set(true);

        m.on_tick(&mut env);
        assert!(m.close_to_edge);
        assert_eq!(env.actions, vec![Action::Sneak(true)]);
    }

    #[test]
    fn a_block_under_the_feet_keeps_walking() {
        let mut m = SafeWalk::new();
        let mut env = TestEnv::in_world();
        m.sneak.set(true);
        env.push_block(BlockPos::new(0, 63, 0), "minecraft:stone");

        m.on_tick(&mut env);
        assert!(!m.close_to_edge);
        assert!(env.actions.is_empty());
    }

    #[test]
    fn rendering_the_edge_helper_draws_a_box() {
        let mut m = SafeWalk::new();
        let mut env = TestEnv::in_world();
        m.sneak.set(true);
        m.render.set(true);
        m.render_player_box.set(true);

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.boxes, 2);
    }
}
